use crate::app::NekoSpiceApp;
use crate::app::localization::{StudioLocale, UiText};
use crate::app::theme::{StudioTheme, StudioThemeMode};
use eframe::egui::{self, RichText};

impl NekoSpiceApp {
    /// draw settings center workspace。
    pub(crate) fn draw_settings_center_workspace(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.heading(self.text(UiText::Settings));
            ui.label(StudioTheme::muted_for(
                mode,
                self.text(UiText::StudioSubtitle),
            ));
            ui.add_space(10.0);
            let spacing = 10.0;
            let width = ui.available_width();
            if width < 700.0 {
                ui.vertical(|ui| {
                    self.draw_settings_theme_gallery(ui);
                    ui.add_space(10.0);
                    self.draw_settings_workspace_section(ui);
                    ui.add_space(10.0);
                    self.draw_settings_runtime_section(ui);
                    ui.add_space(10.0);
                    self.draw_settings_localization_section(ui);
                });
            } else {
                let left_width = ((width - spacing) * 0.60).max(360.0);
                let right_width = (width - left_width - spacing).max(260.0);
                ui.horizontal_top(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width(left_width);
                        self.draw_settings_theme_gallery(ui);
                        ui.add_space(10.0);
                        self.draw_settings_workspace_section(ui);
                    });
                    ui.add_space(spacing);
                    ui.vertical(|ui| {
                        ui.set_width(right_width);
                        self.draw_settings_runtime_section(ui);
                        ui.add_space(10.0);
                        self.draw_settings_localization_section(ui);
                    });
                });
            }
        });
    }

    fn draw_settings_workspace_section(&self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::Workspace),
            ));
            settings_row(ui, mode, self.text(UiText::Document), &self.schematic_path);
            settings_row(
                ui,
                mode,
                self.text(UiText::Libraries),
                &self.library_table_path,
            );
            settings_row(
                ui,
                mode,
                self.text(UiText::Renderer),
                "wgpu / hardware accelerated",
            );
        });
    }

    fn draw_settings_runtime_section(&self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::System),
            ));
            settings_row(ui, mode, self.text(UiText::Solver), "ngspice");
            settings_row(ui, mode, self.text(UiText::Backend), "CLI isolated");
            settings_row(ui, mode, self.text(UiText::Threads), "auto");
            settings_row(ui, mode, self.text(UiText::Graphics), "egui + wgpu");
        });
    }

    fn draw_settings_localization_section(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
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
                for locale in StudioLocale::ALL {
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

fn settings_row(ui: &mut egui::Ui, mode: StudioThemeMode, label: &str, value: &str) {
    let palette = StudioTheme::palette(mode);
    ui.horizontal(|ui| {
        ui.label(StudioTheme::muted_for(mode, label));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(value).color(palette.text));
        });
    });
}
