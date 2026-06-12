//! Settings workspace — application configuration center.
//!
//! Organized sections:
//! - Theme gallery (left/top)
//! - Workspace paths (document, library)
//! - Runtime / Solver paths (ngspice, Xyce)
//! - Simulation defaults summary
//! - Localization

use crate::app::NekoSpiceApp;
use crate::app::localization::{StudioLocale, UiText};
use crate::app::theme::{StudioTheme, StudioThemeMode};
use eframe::egui::{self, RichText};

impl NekoSpiceApp {
    /// Draw settings center workspace.
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
                    self.draw_settings_simulation_defaults(ui);
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
                        ui.add_space(10.0);
                        self.draw_settings_simulation_defaults(ui);
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

    fn draw_settings_runtime_section(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::System),
            ));
            // Editable ngspice path (auto-save on change)
            ui.horizontal(|ui| {
                ui.label(StudioTheme::muted_for(mode, "ngspice"));
                if ui.text_edit_singleline(&mut self.preferences.ngspice_path).changed() {
                    self.preferences.save_to_disk();
                }
            });
            // Editable Xyce path (auto-save on change)
            ui.horizontal(|ui| {
                ui.label(StudioTheme::muted_for(mode, "Xyce"));
                if ui.text_edit_singleline(&mut self.preferences.xyce_path).changed() {
                    self.preferences.save_to_disk();
                }
            });
            settings_row(ui, mode, self.text(UiText::Backend), "CLI isolated");
            settings_row(ui, mode, self.text(UiText::Threads), "auto");
            settings_row(ui, mode, self.text(UiText::Graphics), "egui + wgpu");
        });
    }

    /// Simulation defaults section — shows current solver options at a glance.
    fn draw_settings_simulation_defaults(&self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let opts = &self.simulation_profile_editor.options;
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(mode, "Simulation Defaults"));
            ui.add_space(4.0);

            let preset = &self.simulation_profile_editor.active_preset;
            if preset != "default" {
                ui.horizontal(|ui| {
                    ui.label(StudioTheme::muted_for(mode, "Active Preset"));
                    ui.label(egui::RichText::new(preset).strong().color(self.theme_palette().accent));
                });
            }

            egui::Grid::new("settings_sim_defaults")
                .num_columns(2)
                .spacing([12.0, 4.0])
                .show(ui, |ui| {
                    settings_grid_row(ui, mode, "Temperature", &format!("{} °C", opts.temperature));
                    settings_grid_row(ui, mode, "Method", &opts.method);
                    settings_grid_row(ui, mode, "RELTOL", &opts.reltol);
                    settings_grid_row(ui, mode, "ABSTOL", &opts.abstol);
                    settings_grid_row(ui, mode, "VNTOL", &opts.vntol);
                    settings_grid_row(ui, mode, "GMIN", &opts.gmin);
                    settings_grid_row(ui, mode, "ITL1", &opts.itl1);
                    settings_grid_row(ui, mode, "ITL4", &opts.itl4);
                });

            ui.add_space(4.0);
            ui.label(StudioTheme::muted_for(
                mode,
                "Edit simulation options in the Simulation workspace Profile Editor.",
            ));
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

fn settings_grid_row(ui: &mut egui::Ui, mode: StudioThemeMode, label: &str, value: &str) {
    ui.label(StudioTheme::muted_for(mode, label));
    ui.label(egui::RichText::new(value).monospace());
    ui.end_row();
}
