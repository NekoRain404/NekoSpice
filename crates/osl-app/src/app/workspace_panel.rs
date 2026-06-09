use super::NekoSpiceApp;
use super::localization::UiText;
use super::navigation::StudioWorkspace;
use super::theme::{StudioTheme, StudioThemeMode};
use eframe::egui;

impl NekoSpiceApp {
    pub(super) fn draw_right_workspace_panel(&mut self, ui: &mut egui::Ui) {
        match self.active_workspace {
            StudioWorkspace::Schematic => self.draw_schematic_workspace_panel(ui),
            StudioWorkspace::Library => self.draw_library_browser(ui),
            StudioWorkspace::Simulation => self.draw_simulation_panel(ui),
            StudioWorkspace::Reports => self.draw_reports_workspace_panel(ui),
            StudioWorkspace::Settings => self.draw_settings_workspace_panel(ui),
        }
    }

    fn draw_schematic_workspace_panel(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        ui.heading(self.text(UiText::SchematicTools));
        ui.label(StudioTheme::muted_for(
            mode,
            self.text(UiText::SchematicToolsCaption),
        ));
        ui.add_space(8.0);
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            self.draw_schematic_tool_controls(ui);
        });
        ui.add_space(8.0);
        self.draw_document_diagnostics_panel(ui, 220.0);
    }

    fn draw_reports_workspace_panel(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        ui.heading(self.text(UiText::ReportsResults));
        ui.label(StudioTheme::muted_for(
            mode,
            self.text(UiText::ReportsCaption),
        ));
        ui.add_space(8.0);
        self.draw_simulation_panel(ui);
    }

    fn draw_settings_workspace_panel(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        ui.heading(self.text(UiText::Settings));
        ui.label(StudioTheme::muted_for(
            mode,
            self.text(UiText::StudioSubtitle),
        ));
        ui.add_space(8.0);
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::Appearance),
            ));
            ui.label(StudioTheme::muted_for(
                mode,
                format!(
                    "{}: {}",
                    self.text(UiText::CurrentTheme),
                    self.theme_mode_label(mode)
                ),
            ));
            ui.horizontal_wrapped(|ui| {
                for candidate in StudioThemeMode::ALL {
                    let label = self.theme_mode_label(candidate);
                    if ui
                        .selectable_value(&mut self.preferences.theme_mode, candidate, label)
                        .changed()
                    {
                        self.status_message =
                            Some(format!("{}: {}", self.text(UiText::Theme), label));
                    }
                }
            });
        });

        ui.add_space(8.0);
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::Language),
            ));
            ui.label(StudioTheme::muted_for(
                mode,
                format!(
                    "{}: {}",
                    self.text(UiText::CurrentLanguage),
                    self.locale().native_name()
                ),
            ));
            ui.horizontal_wrapped(|ui| {
                for locale in super::localization::StudioLocale::ALL {
                    if ui
                        .selectable_value(
                            &mut self.preferences.locale,
                            locale,
                            locale.native_name(),
                        )
                        .changed()
                    {
                        self.status_message = Some(format!(
                            "{}: {}",
                            self.text(UiText::Language),
                            locale.native_name()
                        ));
                    }
                }
            });
        });
    }
}
