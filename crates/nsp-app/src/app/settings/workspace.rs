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
            ui.add_space(4.0);

            // ngspice path with validation indicator
            ui.label(StudioTheme::muted_for(mode, "Solver Paths"));
            ui.add_space(2.0);
            solver_path_row(ui, mode, &mut self.preferences.ngspice_path, "ngspice");
            ui.add_space(4.0);
            solver_path_row(ui, mode, &mut self.preferences.xyce_path, "Xyce");
            if ui.small_button("Save Paths").clicked() {
                self.preferences.save_to_disk();
                self.status_message = Some("Solver paths saved".to_string());
            }
            ui.add_space(8.0);

            settings_row(ui, mode, self.text(UiText::Backend), "CLI isolated");
            settings_row(ui, mode, self.text(UiText::Threads), "auto");
            settings_row(ui, mode, self.text(UiText::Graphics), "egui + wgpu");
        });
    }

    /// Simulation defaults section — editable solver options with auto-save.
    fn draw_settings_simulation_defaults(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let mut changed = false;
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(mode, "Simulation Defaults"));
            ui.add_space(4.0);

            let preset = self.simulation_profile_editor.active_preset.clone();
            if preset != "default" {
                ui.horizontal(|ui| {
                    ui.label(StudioTheme::muted_for(mode, "Active Preset"));
                    ui.label(
                        egui::RichText::new(&preset)
                            .strong()
                            .color(self.theme_palette().accent),
                    );
                });
            }

            // Backend selector
            ui.horizontal(|ui| {
                ui.label(StudioTheme::muted_for(mode, "Backend"));
                egui::ComboBox::from_id_salt("settings_backend")
                    .selected_text(self.simulation_panel.backend.label())
                    .show_ui(ui, |ui| {
                        for &kind in &super::super::simulation::state::SimulationBackendKind::ALL {
                            if ui
                                .selectable_value(
                                    &mut self.simulation_panel.backend,
                                    kind,
                                    kind.label(),
                                )
                                .changed()
                            {
                                changed = true;
                            }
                        }
                    });
            });
            ui.add_space(4.0);

            // Analysis type selector
            ui.horizontal(|ui| {
                ui.label(StudioTheme::muted_for(mode, "Analysis"));
                for kind in [
                    nsp_schema::NspSimulationDirectiveKind::Tran,
                    nsp_schema::NspSimulationDirectiveKind::Ac,
                    nsp_schema::NspSimulationDirectiveKind::Dc,
                    nsp_schema::NspSimulationDirectiveKind::Op,
                    nsp_schema::NspSimulationDirectiveKind::Noise,
                ] {
                    let active = self.simulation_panel.directive_kind == kind;
                    let btn = if active {
                        egui::Button::new(
                            egui::RichText::new(kind.to_string())
                                .strong()
                                .color(self.theme_palette().text),
                        )
                        .fill(self.theme_palette().accent_soft)
                        .stroke(egui::Stroke::new(1.0, self.theme_palette().accent))
                    } else {
                        egui::Button::new(
                            egui::RichText::new(kind.to_string())
                                .color(self.theme_palette().text_muted),
                        )
                        .fill(self.theme_palette().panel_soft)
                    };
                    if ui.add(btn).clicked() && !active {
                        self.simulation_panel.directive_kind = kind;
                        self.simulation_panel.analysis_params =
                            super::super::simulation::state::AnalysisParams::for_kind(kind);
                        changed = true;
                    }
                }
            });
            ui.add_space(4.0);

            // Editable solver parameters
            egui::Grid::new("settings_sim_defaults")
                .num_columns(2)
                .spacing([12.0, 4.0])
                .show(ui, |ui| {
                    changed |= settings_edit_row(
                        ui,
                        mode,
                        "Temperature (°C)",
                        &mut self.simulation_profile_editor.options.temperature,
                    );
                    changed |= settings_edit_row(
                        ui,
                        mode,
                        "Method",
                        &mut self.simulation_profile_editor.options.method,
                    );
                    changed |= settings_edit_row(
                        ui,
                        mode,
                        "RELTOL",
                        &mut self.simulation_profile_editor.options.reltol,
                    );
                    changed |= settings_edit_row(
                        ui,
                        mode,
                        "ABSTOL",
                        &mut self.simulation_profile_editor.options.abstol,
                    );
                    changed |= settings_edit_row(
                        ui,
                        mode,
                        "VNTOL",
                        &mut self.simulation_profile_editor.options.vntol,
                    );
                    changed |= settings_edit_row(
                        ui,
                        mode,
                        "GMIN",
                        &mut self.simulation_profile_editor.options.gmin,
                    );
                    changed |= settings_edit_row(
                        ui,
                        mode,
                        "ITL1",
                        &mut self.simulation_profile_editor.options.itl1,
                    );
                    changed |= settings_edit_row(
                        ui,
                        mode,
                        "ITL4",
                        &mut self.simulation_profile_editor.options.itl4,
                    );
                });
        });

        if changed {
            self.save_simulation_settings();
        }
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

/// Editable settings row that returns whether the value was changed.
fn settings_edit_row(
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
    label: &str,
    value: &mut String,
) -> bool {
    ui.label(StudioTheme::muted_for(mode, label));
    let resp = ui.add(
        egui::TextEdit::singleline(value)
            .desired_width(120.0)
            .font(egui::TextStyle::Monospace),
    );
    let changed = resp.changed();
    ui.end_row();
    changed
}

/// Draw a solver path row with a validation indicator (green check or red X).
fn solver_path_row(ui: &mut egui::Ui, mode: StudioThemeMode, path: &mut String, name: &str) {
    let palette = StudioTheme::palette(mode);
    // Check if the solver is available
    let available = std::process::Command::new("which")
        .arg(path.trim())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    ui.horizontal(|ui| {
        // Validation indicator
        let (icon, color) = if available {
            ("✓", palette.success)
        } else {
            ("✗", palette.danger)
        };
        ui.label(egui::RichText::new(icon).color(color).strong());

        ui.label(StudioTheme::muted_for(mode, name));
        let resp = ui.add(
            egui::TextEdit::singleline(path)
                .desired_width(160.0)
                .font(egui::TextStyle::Monospace)
                .hint_text(format!("path to {}", name)),
        );
        if resp.changed() {
            // Auto-save on change
        }

        if available {
            ui.label(StudioTheme::muted_for(mode, "found"));
        } else {
            ui.label(
                egui::RichText::new("not found")
                    .color(palette.danger)
                    .size(11.0),
            );
        }
    });
}
