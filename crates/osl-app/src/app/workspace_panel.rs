use super::NekoSpiceApp;
use super::localization::UiText;
use super::navigation::StudioWorkspace;
use super::theme::{StudioTheme, StudioThemeMode};
use eframe::egui;

impl NekoSpiceApp {
    pub(super) fn draw_left_context_panel(&mut self, ui: &mut egui::Ui) {
        match self.active_workspace {
            StudioWorkspace::Home => self.draw_home_project_context(ui),
            StudioWorkspace::Schematic
            | StudioWorkspace::Library
            | StudioWorkspace::Simulation
            | StudioWorkspace::Optimization
            | StudioWorkspace::Review
            | StudioWorkspace::Waveforms
            | StudioWorkspace::Reports
            | StudioWorkspace::Settings => self.draw_project_sidebar(ui),
        }
    }

    pub(super) fn draw_right_workspace_panel(&mut self, ui: &mut egui::Ui) {
        match self.active_workspace {
            StudioWorkspace::Home => self.draw_home_insights_panel(ui),
            StudioWorkspace::Schematic => self.draw_schematic_inspector_panel(ui),
            StudioWorkspace::Library => self.draw_library_validation_panel(ui),
            StudioWorkspace::Simulation => self.draw_simulation_panel(ui),
            StudioWorkspace::Optimization => self.draw_optimization_workspace_panel(ui),
            StudioWorkspace::Review => self.draw_review_workspace_panel(ui),
            StudioWorkspace::Waveforms => self.draw_waveform_workspace_panel(ui),
            StudioWorkspace::Reports => self.draw_reports_workspace_panel(ui),
            StudioWorkspace::Settings => self.draw_settings_workspace_panel(ui),
        }
    }

    fn draw_reports_workspace_panel(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        ui.heading(self.text(UiText::ReportsResults));
        ui.label(StudioTheme::muted_for(
            mode,
            self.text(UiText::ReportsCaption),
        ));
        ui.add_space(8.0);
        egui::ScrollArea::vertical()
            .id_salt("reports_right_panel_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                self.draw_report_preview_section(ui);
                ui.add_space(8.0);
                self.draw_report_export_section(ui);
            });
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
