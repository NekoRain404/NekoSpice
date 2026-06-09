use super::NekoSpiceApp;
use crate::canvas;
use crate::document::KicadGuiDocument;
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Stroke};
use osl_kicad::{KicadAt, KicadEditSummary, KicadLabelKind, KicadPoint};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SchematicTool {
    Select,
    Wire,
    Label,
    GlobalLabel,
    Text,
    Junction,
    NoConnect,
}

impl SchematicTool {
    const ALL: [Self; 7] = [
        Self::Select,
        Self::Wire,
        Self::Label,
        Self::GlobalLabel,
        Self::Text,
        Self::Junction,
        Self::NoConnect,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Select => "Select",
            Self::Wire => "Wire",
            Self::Label => "Label",
            Self::GlobalLabel => "Global",
            Self::Text => "Text",
            Self::Junction => "Junction",
            Self::NoConnect => "No Connect",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SchematicToolState {
    active: SchematicTool,
    label_text: String,
    text_item: String,
    pending_wire_start: Option<KicadPoint>,
}

impl Default for SchematicToolState {
    fn default() -> Self {
        Self {
            active: SchematicTool::Select,
            label_text: "net".to_string(),
            text_item: ".save v(out)".to_string(),
            pending_wire_start: None,
        }
    }
}

impl SchematicToolState {
    pub(super) fn clear_pending(&mut self) {
        self.pending_wire_start = None;
    }

    fn set_active(&mut self, tool: SchematicTool) {
        if self.active != tool {
            self.active = tool;
            self.clear_pending();
        }
    }
}

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
            SchematicTool::Label | SchematicTool::GlobalLabel => {
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
            SchematicTool::Select | SchematicTool::Junction | SchematicTool::NoConnect => {}
        }
    }

    pub(super) fn select_schematic_tool(&mut self) {
        self.schematic_tools.set_active(SchematicTool::Select);
    }

    pub(super) fn cancel_schematic_tool_pending(&mut self) {
        if self.schematic_tools.pending_wire_start.is_some() {
            self.schematic_tools.clear_pending();
            self.status_message = Some("Canceled pending wire".to_string());
        }
    }

    pub(super) fn handle_schematic_tool_click(&mut self, point: KicadPoint) -> bool {
        match self.schematic_tools.active {
            SchematicTool::Select => false,
            SchematicTool::Wire => {
                self.handle_wire_tool_click(point);
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
        match self.schematic_tools.active {
            SchematicTool::Wire => {
                if let Some(start) = self.schematic_tools.pending_wire_start {
                    canvas::draw_line(
                        painter,
                        rect,
                        self.viewport,
                        start,
                        point,
                        Color32::from_rgb(0, 130, 85),
                        1.5,
                    );
                }
            }
            SchematicTool::Label | SchematicTool::GlobalLabel => {
                painter.text(
                    self.viewport.world_to_screen(rect, point),
                    Align2::LEFT_TOP,
                    &self.schematic_tools.label_text,
                    FontId::monospace(12.0),
                    Color32::from_rgb(0, 95, 180),
                );
            }
            SchematicTool::Text => {
                painter.text(
                    self.viewport.world_to_screen(rect, point),
                    Align2::LEFT_TOP,
                    &self.schematic_tools.text_item,
                    FontId::monospace(12.0),
                    Color32::from_rgb(165, 45, 45),
                );
            }
            SchematicTool::Junction => {
                painter.circle_filled(
                    self.viewport.world_to_screen(rect, point),
                    3.0,
                    Color32::from_rgb(0, 150, 72),
                );
            }
            SchematicTool::NoConnect => {
                draw_no_connect_preview(painter, rect, self.viewport.world_to_screen(rect, point));
            }
            SchematicTool::Select => {}
        }
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

fn draw_no_connect_preview(painter: &egui::Painter, rect: Rect, center: Pos2) {
    if !rect.contains(center) {
        return;
    }
    let size = 5.0;
    painter.line_segment(
        [
            Pos2::new(center.x - size, center.y - size),
            Pos2::new(center.x + size, center.y + size),
        ],
        Stroke::new(1.5, Color32::from_rgb(55, 55, 55)),
    );
    painter.line_segment(
        [
            Pos2::new(center.x - size, center.y + size),
            Pos2::new(center.x + size, center.y - size),
        ],
        Stroke::new(1.5, Color32::from_rgb(55, 55, 55)),
    );
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
