use super::{EditNudgeDirection, NekoSpiceApp};
use eframe::egui::{self, Color32, Vec2};
use std::path::PathBuf;

impl NekoSpiceApp {
    fn draw_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("NekoSpice");
            ui.separator();
            ui.label("Schematic");
            let path_response = ui.text_edit_singleline(&mut self.schematic_path);
            let load_requested = ui.button("Open").clicked()
                || (path_response.lost_focus()
                    && ui.input(|input| input.key_pressed(egui::Key::Enter)));
            if load_requested {
                self.load_schematic(PathBuf::from(self.schematic_path.trim()));
            }
            if ui.button("Fit").clicked() {
                self.viewport
                    .fit_scene(self.scene.as_ref().and_then(|scene| scene.bounds));
            }
            let can_edit = self.document.is_some();
            if ui
                .add_enabled(can_edit, egui::Button::new("Save"))
                .clicked()
            {
                self.save_document();
            }
            let can_delete = self
                .selected_hit
                .as_ref()
                .and_then(|hit| hit.uuid.as_ref())
                .is_some();
            if ui
                .add_enabled(can_edit && can_delete, egui::Button::new("Delete"))
                .clicked()
            {
                self.delete_selected();
            }
            if can_edit && can_delete {
                ui.separator();
                if ui.button("Left").clicked() {
                    self.nudge_selected(EditNudgeDirection::Left);
                }
                if ui.button("Right").clicked() {
                    self.nudge_selected(EditNudgeDirection::Right);
                }
                if ui.button("Up").clicked() {
                    self.nudge_selected(EditNudgeDirection::Up);
                }
                if ui.button("Down").clicked() {
                    self.nudge_selected(EditNudgeDirection::Down);
                }
            }
            if let Some(message) = &self.status_message {
                ui.separator();
                ui.label(message);
            }
        });
    }

    fn draw_sidebar(&mut self, ui: &mut egui::Ui) {
        ui.heading("Project");
        ui.label("Renderer: wgpu");
        ui.separator();

        if let Some(error) = &self.load_error {
            ui.colored_label(Color32::from_rgb(190, 40, 40), error);
            return;
        }

        let Some(scene) = &self.scene else {
            ui.label("No schematic loaded");
            return;
        };

        ui.label(format!("Source: {}", scene.source));
        egui::Grid::new("project_stats")
            .num_columns(2)
            .spacing(Vec2::new(16.0, 4.0))
            .show(ui, |ui| {
                ui.label("Symbols");
                ui.label(scene.symbols.len().to_string());
                ui.end_row();
                ui.label("Wires");
                ui.label(scene.wires.len().to_string());
                ui.end_row();
                ui.label("Buses");
                ui.label(scene.buses.len().to_string());
                ui.end_row();
                ui.label("Labels");
                ui.label(scene.labels.len().to_string());
                ui.end_row();
                ui.label("Sheets");
                ui.label(scene.sheets.len().to_string());
                ui.end_row();
                ui.label("Graphics");
                ui.label(scene.graphics.len().to_string());
                ui.end_row();
            });
        ui.label(format!("Zoom: {:.1} px/mm", self.viewport.zoom));
        if let Some(document) = &self.document {
            ui.label(format!(
                "Dirty: {}",
                if document.is_dirty() { "yes" } else { "no" }
            ));
        }

        ui.separator();
        ui.heading("Selection");
        if let Some(hit) = &self.selected_hit {
            ui.label(format!("Kind: {}", hit.kind));
            ui.label(format!("Label: {}", hit.label));
            if let Some(uuid) = &hit.uuid {
                ui.monospace(uuid);
            }
        } else {
            ui.label("None");
        }
        self.draw_selection_property_editor(ui);

        ui.separator();
        self.draw_schematic_tool_controls(ui);
    }
}

impl eframe::App for NekoSpiceApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Panel::top("nekospice_toolbar")
            .exact_size(46.0)
            .show_inside(ui, |ui| {
                ui.add_space(4.0);
                self.draw_toolbar(ui);
            });
        egui::Panel::left("nekospice_project_panel")
            .default_size(280.0)
            .min_size(220.0)
            .max_size(380.0)
            .show_inside(ui, |ui| {
                ui.add_space(8.0);
                self.draw_sidebar(ui);
            });
        egui::Panel::right("nekospice_library_panel")
            .default_size(340.0)
            .min_size(260.0)
            .max_size(480.0)
            .show_inside(ui, |ui| {
                ui.add_space(8.0);
                self.draw_library_browser(ui);
            });
        egui::CentralPanel::default().show_inside(ui, |ui| {
            self.draw_canvas(ui);
        });
    }
}
