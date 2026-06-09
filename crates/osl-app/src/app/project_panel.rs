use super::localization::UiText;
use super::widgets::metric_row;
use super::{NekoSpiceApp, theme::StudioTheme};
use eframe::egui;
use std::path::PathBuf;

impl NekoSpiceApp {
    pub(super) fn draw_project_sidebar(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let palette = self.theme_palette();

        ui.label(StudioTheme::section_title_for(
            mode,
            self.text(UiText::ActiveProject),
        ));
        let path_response = ui.text_edit_singleline(&mut self.schematic_path);
        let load_requested = ui.button(self.text(UiText::OpenSchematic)).clicked()
            || (path_response.lost_focus()
                && ui.input(|input| input.key_pressed(egui::Key::Enter)));
        if load_requested {
            self.load_schematic(PathBuf::from(self.schematic_path.trim()));
        }
        ui.add_space(8.0);

        if let Some(error) = &self.load_error {
            ui.colored_label(palette.danger, error);
            return;
        }

        let Some(scene) = &self.scene else {
            ui.label(self.text(UiText::NoSchematicLoaded));
            return;
        };

        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::SchematicHealth),
            ));
            metric_row(
                ui,
                mode,
                self.text(UiText::Symbols),
                &scene.symbols.len().to_string(),
            );
            metric_row(
                ui,
                mode,
                self.text(UiText::Wires),
                &scene.wires.len().to_string(),
            );
            metric_row(
                ui,
                mode,
                self.text(UiText::Buses),
                &scene.buses.len().to_string(),
            );
            metric_row(
                ui,
                mode,
                self.text(UiText::Labels),
                &scene.labels.len().to_string(),
            );
            metric_row(
                ui,
                mode,
                self.text(UiText::Sheets),
                &scene.sheets.len().to_string(),
            );
            metric_row(
                ui,
                mode,
                self.text(UiText::Graphics),
                &scene.graphics.len().to_string(),
            );
            metric_row(
                ui,
                mode,
                self.text(UiText::Zoom),
                &format!("{:.1} px/mm", self.viewport.zoom),
            );
        });

        ui.add_space(8.0);
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::Selection),
            ));
            if let Some(hit) = &self.selected_hit {
                metric_row(ui, mode, self.text(UiText::Kind), &hit.kind);
                ui.label(&hit.label);
                if let Some(uuid) = &hit.uuid {
                    ui.monospace(uuid);
                }
            } else {
                ui.label(StudioTheme::muted_for(
                    mode,
                    self.text(UiText::NoSelectedItem),
                ));
            }
        });
    }
}
