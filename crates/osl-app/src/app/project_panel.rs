use super::widgets::metric_row;
use super::{EditNudgeDirection, NekoSpiceApp, theme::StudioTheme};
use eframe::egui;
use std::path::PathBuf;

impl NekoSpiceApp {
    pub(super) fn draw_project_sidebar(&mut self, ui: &mut egui::Ui) {
        ui.label(StudioTheme::section_title("Active Project"));
        let path_response = ui.text_edit_singleline(&mut self.schematic_path);
        let load_requested = ui.button("Open Schematic").clicked()
            || (path_response.lost_focus()
                && ui.input(|input| input.key_pressed(egui::Key::Enter)));
        if load_requested {
            self.load_schematic(PathBuf::from(self.schematic_path.trim()));
        }
        ui.add_space(8.0);

        if let Some(error) = &self.load_error {
            ui.colored_label(StudioTheme::DANGER, error);
            return;
        }

        let Some(scene) = &self.scene else {
            ui.label("No schematic loaded");
            return;
        };

        StudioTheme::panel_frame().show(ui, |ui| {
            ui.label(StudioTheme::section_title("Schematic Health"));
            metric_row(ui, "Symbols", &scene.symbols.len().to_string());
            metric_row(ui, "Wires", &scene.wires.len().to_string());
            metric_row(ui, "Buses", &scene.buses.len().to_string());
            metric_row(ui, "Labels", &scene.labels.len().to_string());
            metric_row(ui, "Sheets", &scene.sheets.len().to_string());
            metric_row(ui, "Graphics", &scene.graphics.len().to_string());
            metric_row(ui, "Zoom", &format!("{:.1} px/mm", self.viewport.zoom));
        });

        ui.add_space(8.0);
        StudioTheme::panel_frame().show(ui, |ui| {
            ui.label(StudioTheme::section_title("Selection"));
            if let Some(hit) = &self.selected_hit {
                metric_row(ui, "Kind", &hit.kind);
                ui.label(&hit.label);
                if let Some(uuid) = &hit.uuid {
                    ui.monospace(uuid);
                }
            } else {
                ui.label(StudioTheme::muted("No selected item"));
            }
            self.draw_selection_property_editor(ui);
        });

        ui.add_space(8.0);
        StudioTheme::panel_frame().show(ui, |ui| {
            ui.label(StudioTheme::section_title("Edit Commands"));
            let can_edit = self.document.is_some();
            let can_delete = self
                .selected_hit
                .as_ref()
                .and_then(|hit| hit.uuid.as_ref())
                .is_some();
            if ui
                .add_enabled(can_edit && can_delete, egui::Button::new("Delete Selected"))
                .clicked()
            {
                self.delete_selected();
            }
            ui.horizontal_wrapped(|ui| {
                for (label, direction) in [
                    ("Left", EditNudgeDirection::Left),
                    ("Right", EditNudgeDirection::Right),
                    ("Up", EditNudgeDirection::Up),
                    ("Down", EditNudgeDirection::Down),
                ] {
                    if ui
                        .add_enabled(can_edit && can_delete, egui::Button::new(label))
                        .clicked()
                    {
                        self.nudge_selected(direction);
                    }
                }
            });
        });
    }
}
