use super::NekoSpiceApp;
use crate::document::KicadGuiDocument;
use eframe::egui::{self, Rect};
use osl_kicad::{KicadAt, KicadEditSummary, KicadLabelKind, KicadPoint};

mod preview;
mod state;

use state::SchematicTool;
pub(crate) use state::SchematicToolState;

impl NekoSpiceApp {
    pub(super) fn draw_schematic_tool_controls(&mut self, ui: &mut egui::Ui) {
        ui.heading("Tools");
        ui.horizontal_wrapped(|ui| {
            for tool in SchematicTool::ALL {
                let selected = self.schematic_tools.active == tool;
                if ui.selectable_label(selected, tool.label()).clicked() {
                    self.activate_schematic_tool(tool);
                }
            }
        });

        match self.schematic_tools.active {
            SchematicTool::Label
            | SchematicTool::GlobalLabel
            | SchematicTool::HierarchicalLabel => {
                ui.horizontal(|ui| {
                    ui.label("Name");
                    ui.text_edit_singleline(&mut self.schematic_tools.label_text);
                });
            }
            SchematicTool::Text => {
                ui.horizontal(|ui| {
                    ui.label("Text");
                    ui.text_edit_singleline(&mut self.schematic_tools.text_item);
                });
            }
            SchematicTool::Wire => {
                if let Some(start) = self.schematic_tools.pending_wire_start {
                    ui.label(format!("Wire start: {:.2}, {:.2}", start.x, start.y));
                }
            }
            SchematicTool::Bus => {
                if let Some(start) = self.schematic_tools.pending_bus_start {
                    ui.label(format!("Bus start: {:.2}, {:.2}", start.x, start.y));
                }
            }
            SchematicTool::BusEntry => {
                ui.horizontal(|ui| {
                    ui.label("dx");
                    ui.add(egui::DragValue::new(
                        &mut self.schematic_tools.bus_entry_size.width,
                    ));
                    ui.label("dy");
                    ui.add(egui::DragValue::new(
                        &mut self.schematic_tools.bus_entry_size.height,
                    ));
                });
            }
            SchematicTool::Select | SchematicTool::Junction | SchematicTool::NoConnect => {}
        }
    }

    pub(super) fn select_schematic_tool(&mut self) {
        self.schematic_tools.set_active(SchematicTool::Select);
    }

    pub(super) fn cancel_schematic_tool_pending(&mut self) {
        if self.schematic_tools.has_pending() {
            self.schematic_tools.clear_pending();
            self.status_message = Some("Canceled pending schematic tool".to_string());
        }
    }

    pub(super) fn handle_schematic_tool_click(&mut self, point: KicadPoint) -> bool {
        match self.schematic_tools.active {
            SchematicTool::Select => false,
            SchematicTool::Wire => {
                self.handle_wire_tool_click(point);
                true
            }
            SchematicTool::Bus => {
                self.handle_bus_tool_click(point);
                true
            }
            SchematicTool::BusEntry => {
                let size = self.schematic_tools.bus_entry_size;
                self.apply_schematic_tool_edit(Some(point), |document| {
                    document.add_bus_entry(point, size)
                });
                true
            }
            SchematicTool::Label => {
                let text = self.schematic_tools.label_text.clone();
                self.apply_schematic_tool_edit(Some(point), |document| {
                    document.add_label(text, KicadLabelKind::Local, at_from_point(point))
                });
                true
            }
            SchematicTool::GlobalLabel => {
                let text = self.schematic_tools.label_text.clone();
                self.apply_schematic_tool_edit(Some(point), |document| {
                    document.add_label(text, KicadLabelKind::Global, at_from_point(point))
                });
                true
            }
            SchematicTool::HierarchicalLabel => {
                let text = self.schematic_tools.label_text.clone();
                self.apply_schematic_tool_edit(Some(point), |document| {
                    document.add_label(text, KicadLabelKind::Hierarchical, at_from_point(point))
                });
                true
            }
            SchematicTool::Text => {
                let text = self.schematic_tools.text_item.clone();
                self.apply_schematic_tool_edit(Some(point), |document| {
                    document.add_text(text, at_from_point(point))
                });
                true
            }
            SchematicTool::Junction => {
                self.apply_schematic_tool_edit(Some(point), |document| {
                    document.add_junction(point)
                });
                true
            }
            SchematicTool::NoConnect => {
                self.apply_schematic_tool_edit(Some(point), |document| {
                    document.add_no_connect(point)
                });
                true
            }
        }
    }

    pub(super) fn draw_schematic_tool_preview(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        point: KicadPoint,
    ) {
        preview::draw_schematic_tool_preview(
            painter,
            rect,
            self.viewport,
            &self.schematic_tools,
            point,
        );
    }

    fn activate_schematic_tool(&mut self, tool: SchematicTool) {
        self.schematic_tools.set_active(tool);
        if tool != SchematicTool::Select {
            self.placement = None;
        }
        self.status_message = Some(format!("Tool: {}", tool.label()));
    }

    fn handle_wire_tool_click(&mut self, point: KicadPoint) {
        let Some(start) = self.schematic_tools.pending_wire_start.take() else {
            self.schematic_tools.pending_wire_start = Some(point);
            self.status_message = Some(format!("Wire start {:.2}, {:.2}", point.x, point.y));
            return;
        };
        if same_point(start, point) {
            self.schematic_tools.pending_wire_start = Some(start);
            self.status_message = Some("Wire end must differ from start".to_string());
            return;
        }

        let did_apply =
            self.apply_schematic_tool_edit(None, |document| document.add_wire(vec![start, point]));
        self.schematic_tools.pending_wire_start = Some(if did_apply { point } else { start });
    }

    fn handle_bus_tool_click(&mut self, point: KicadPoint) {
        let Some(start) = self.schematic_tools.pending_bus_start.take() else {
            self.schematic_tools.pending_bus_start = Some(point);
            self.status_message = Some(format!("Bus start {:.2}, {:.2}", point.x, point.y));
            return;
        };
        if same_point(start, point) {
            self.schematic_tools.pending_bus_start = Some(start);
            self.status_message = Some("Bus end must differ from start".to_string());
            return;
        }

        let did_apply =
            self.apply_schematic_tool_edit(None, |document| document.add_bus(vec![start, point]));
        self.schematic_tools.pending_bus_start = Some(if did_apply { point } else { start });
    }

    fn apply_schematic_tool_edit<F>(&mut self, selection_point: Option<KicadPoint>, edit: F) -> bool
    where
        F: FnOnce(&mut KicadGuiDocument) -> Result<KicadEditSummary, String>,
    {
        let Some(document) = &mut self.document else {
            self.status_message = Some("No editable schematic loaded".to_string());
            return false;
        };

        match edit(document) {
            Ok(summary) => {
                let scene = document.scene();
                self.selected_hit =
                    selection_point.and_then(|point| scene.hit_test(point).hits.into_iter().next());
                self.scene = Some(scene);
                self.load_error = None;
                self.status_message =
                    Some(format!("Edited {} {}", summary.operation, summary.target));
                true
            }
            Err(error) => {
                self.status_message = Some(error);
                false
            }
        }
    }
}

fn at_from_point(point: KicadPoint) -> KicadAt {
    KicadAt {
        x: point.x,
        y: point.y,
        rotation: 0.0,
    }
}

fn same_point(left: KicadPoint, right: KicadPoint) -> bool {
    (left.x - right.x).abs() < 1e-6 && (left.y - right.y).abs() < 1e-6
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn switching_schematic_tool_clears_pending_wire_start() {
        let mut state = SchematicToolState::default();
        state.set_active(SchematicTool::Wire);
        state.pending_wire_start = Some(KicadPoint { x: 1.0, y: 2.0 });

        state.set_active(SchematicTool::Label);

        assert_eq!(state.active, SchematicTool::Label);
        assert!(state.pending_wire_start.is_none());
    }

    #[test]
    fn same_point_allows_tiny_rounding_drift() {
        assert!(same_point(
            KicadPoint { x: 1.0, y: 2.0 },
            KicadPoint {
                x: 1.0 + 1e-7,
                y: 2.0 - 1e-7
            }
        ));
    }
}
