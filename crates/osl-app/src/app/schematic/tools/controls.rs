use super::state::SchematicTool;
use crate::app::NekoSpiceApp;
use eframe::egui;

impl NekoSpiceApp {
    pub(in crate::app) fn draw_schematic_tool_controls(&mut self, ui: &mut egui::Ui) {
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
            SchematicTool::Sheet => {
                ui.horizontal(|ui| {
                    ui.label("Name");
                    ui.text_edit_singleline(&mut self.schematic_tools.sheet_name);
                });
                ui.horizontal(|ui| {
                    ui.label("File");
                    ui.text_edit_singleline(&mut self.schematic_tools.sheet_file);
                });
                ui.horizontal(|ui| {
                    ui.label("W");
                    ui.add(egui::DragValue::new(
                        &mut self.schematic_tools.sheet_size.width,
                    ));
                    ui.label("H");
                    ui.add(egui::DragValue::new(
                        &mut self.schematic_tools.sheet_size.height,
                    ));
                });
                ui.horizontal(|ui| {
                    ui.label("Pins");
                    ui.text_edit_singleline(&mut self.schematic_tools.sheet_pin_names[0]);
                    ui.text_edit_singleline(&mut self.schematic_tools.sheet_pin_names[1]);
                });
            }
            SchematicTool::Select | SchematicTool::Junction | SchematicTool::NoConnect => {}
        }
    }

    pub(in crate::app) fn select_schematic_tool(&mut self) {
        self.schematic_tools.set_active(SchematicTool::Select);
    }

    fn activate_schematic_tool(&mut self, tool: SchematicTool) {
        self.schematic_tools.set_active(tool);
        if tool != SchematicTool::Select {
            self.placement = None;
        }
        self.status_message = Some(format!("Tool: {}", tool.label()));
    }

    /// Activate a tool directly from the context menu or tool palette.
    /// This is a public entry point for external UI elements to switch tools.
    pub(in crate::app) fn activate_schematic_tool_direct(&mut self, tool: SchematicTool) {
        self.activate_schematic_tool(tool);
    }
}
