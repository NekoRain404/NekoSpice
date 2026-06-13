//! Schematic tool editing — handles click-to-place logic for wires, buses, labels, and other tools.

use super::state::SchematicTool;
use crate::app::NekoSpiceApp;
use crate::document::KicadGuiDocument;
use osl_kicad::{KicadAt, KicadEditSummary, KicadLabelKind, KicadPoint, KicadSheetPin, KicadSize};

impl NekoSpiceApp {
    /// cancel schematic tool pending。
    pub(in crate::app) fn cancel_schematic_tool_pending(&mut self) {
        if self.schematic_tools.has_pending() {
            self.schematic_tools.clear_pending();
            self.status_message = Some("Canceled pending schematic tool".to_string());
        }
    }

    /// handle schematic tool click。
    pub(in crate::app) fn handle_schematic_tool_click(&mut self, point: KicadPoint) -> bool {
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
            SchematicTool::Sheet => {
                let name = self.schematic_tools.sheet_name.clone();
                let file = self.schematic_tools.sheet_file.clone();
                let size = self.schematic_tools.sheet_size;
                let pins = sheet_pins_for_tool(point, size, &self.schematic_tools.sheet_pin_names);
                self.apply_schematic_tool_edit(Some(point), |document| {
                    document.add_sheet(name, file, at_from_point(point), size, pins)
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
        // Snapshot before edit for undo support
        self.history.push(document.snapshot());

        match edit(document) {
            Ok(summary) => {
                let scene = document.scene();
                self.selected_hit =
                    selection_point.and_then(|point| scene.hit_test(point).hits.into_iter().next());
                self.scene = Some(scene);
                self.sync_property_editor_from_selection();
                self.load_error = None;
                self.history.clear_redo();
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

fn sheet_pins_for_tool(
    point: KicadPoint,
    size: KicadSize,
    names: &[String; 2],
) -> Vec<KicadSheetPin> {
    let mut pins = Vec::new();
    let y = point.y + size.height / 2.0;
    let left = names[0].trim();
    if !left.is_empty() {
        pins.push(KicadSheetPin {
            name: left.to_string(),
            pin_type: "input".to_string(),
            at: Some(KicadAt {
                x: point.x,
                y,
                rotation: 180.0,
            }),
            uuid: None,
            effects: None,
        });
    }
    let right = names[1].trim();
    if !right.is_empty() {
        pins.push(KicadSheetPin {
            name: right.to_string(),
            pin_type: "output".to_string(),
            at: Some(KicadAt {
                x: point.x + size.width,
                y,
                rotation: 0.0,
            }),
            uuid: None,
            effects: None,
        });
    }
    pins
}

fn same_point(left: KicadPoint, right: KicadPoint) -> bool {
    (left.x - right.x).abs() < 1e-6 && (left.y - right.y).abs() < 1e-6
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn sheet_pins_follow_sheet_edges() {
        let pins = sheet_pins_for_tool(
            KicadPoint { x: 10.0, y: 20.0 },
            KicadSize {
                width: 30.0,
                height: 12.0,
            },
            &["in".to_string(), "out".to_string()],
        );

        assert_eq!(pins.len(), 2);
        assert_eq!(pins[0].name, "in");
        assert_eq!(pins[0].pin_type, "input");
        assert_eq!(pins[0].at.unwrap().x, 10.0);
        assert_eq!(pins[0].at.unwrap().rotation, 180.0);
        assert_eq!(pins[1].name, "out");
        assert_eq!(pins[1].pin_type, "output");
        assert_eq!(pins[1].at.unwrap().x, 40.0);
        assert_eq!(pins[1].at.unwrap().rotation, 0.0);
    }
}
