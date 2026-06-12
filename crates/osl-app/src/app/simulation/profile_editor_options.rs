//! Right column of the simulation profile editor — organized sections:
//!
//! 1. **Environment** — temperature, nominal temperature
//! 2. **Transient Solver** — method, iteration limits (ITL1/2/4/5), timestep control
//! 3. **Convergence** — tolerances (RELTOL/ABSTOL/VNTOL/GMIN/CHGTOL/PIVTOL/PIVREL)
//! 4. **Output Control** — digit precision
//! 5. **Initial Conditions** — .ic and .nodeset entries
//! 6. **Run Status** — current run state, duration, exit code
//! 7. **Recent Runs** — last simulation run summary

use crate::app::NekoSpiceApp;
use crate::app::localization::UiText;
use super::profile_editor_widgets::section_header;
use crate::app::status_strip::severity_color;
use crate::app::theme::{StudioTheme, StudioThemeMode};
use eframe::egui;
use osl_kicad::KicadDiagnosticSeverity;
use osl_core::RunStatus;

/// Draw the complete right-column options panel with all sections.
pub(crate) fn draw_profile_options(app: &mut NekoSpiceApp, ui: &mut egui::Ui) {
    let mode = app.theme_mode();
    let palette = StudioTheme::palette(mode);
    draw_environment_section(app, ui, mode);
    ui.add_space(8.0);
    egui::CollapsingHeader::new(egui::RichText::new("Transient Solver").color(palette.text))
        .id_salt("collapsible_transient")
        .default_open(false)
        .show(ui, |ui| {
            draw_transient_solver_section(app, ui, mode);
        });
    ui.add_space(8.0);
    egui::CollapsingHeader::new(egui::RichText::new("Convergence").color(palette.text))
        .id_salt("collapsible_convergence")
        .default_open(false)
        .show(ui, |ui| {
            draw_convergence_section(app, ui, mode);
        });
    ui.add_space(8.0);
    draw_output_section(app, ui, mode);
    ui.add_space(8.0);
    egui::CollapsingHeader::new(egui::RichText::new("Initial Conditions").color(palette.text))
        .id_salt("collapsible_ic")
        .default_open(false)
        .show(ui, |ui| {
            draw_initial_conditions_section(app, ui, mode);
        });
    ui.add_space(8.0);
    draw_run_status_summary(app, ui, mode);
    ui.add_space(8.0);
    draw_recent_runs(app, ui, mode);
}

// ── Environment ────────────────────────────────────────────────────────

/// Environment section: operating temperature and nominal model temperature.
fn draw_environment_section(app: &mut NekoSpiceApp, ui: &mut egui::Ui, mode: StudioThemeMode) {
    StudioTheme::panel_frame_for(mode).show(ui, |ui| {
        section_header(ui, mode, "Environment");
        ui.add_space(4.0);

        egui::Grid::new("env_grid")
            .num_columns(2)
            .spacing([8.0, 6.0])
            .show(ui, |ui| {
                labeled_field(ui, mode, "Operating Temp (°C)", &mut app.simulation_profile_editor.options.temperature, 100.0);
                labeled_field(ui, mode, "Nominal Temp (°C)", &mut app.simulation_profile_editor.options.tnom, 100.0);
            });
    });
}

// ── Transient Solver ───────────────────────────────────────────────────

/// Transient solver section: integration method, iteration limits, timestep control.
fn draw_transient_solver_section(app: &mut NekoSpiceApp, ui: &mut egui::Ui, mode: StudioThemeMode) {
    let palette = StudioTheme::palette(mode);
    StudioTheme::panel_frame_for(mode).show(ui, |ui| {
        section_header(ui, mode, "Transient Solver");
        ui.add_space(4.0);

        // Integration method selector
        ui.label(StudioTheme::muted_for(mode, "Integration Method"));
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            for method in ["Trap", "Gear"] {
                let active = app.simulation_profile_editor.options.method == method;
                let btn = if active {
                    egui::Button::new(
                        egui::RichText::new(method).strong().color(palette.text),
                    )
                    .fill(palette.accent_soft)
                    .stroke(egui::Stroke::new(1.0, palette.accent))
                } else {
                    egui::Button::new(
                        egui::RichText::new(method).color(palette.text_muted),
                    )
                    .fill(palette.panel_soft)
                    .stroke(egui::Stroke::new(1.0, palette.border))
                };
                if ui.add(btn).clicked() {
                    app.simulation_profile_editor.options.method = method.to_string();
                    app.save_simulation_settings();
                }
            }
        });

        ui.add_space(6.0);
        egui::Grid::new("transient_grid")
            .num_columns(2)
            .spacing([8.0, 6.0])
            .show(ui, |ui| {
                labeled_field(ui, mode, "ITL1 (DC iterations)", &mut app.simulation_profile_editor.options.itl1, 100.0);
                labeled_field(ui, mode, "ITL2 (DC sweep iters)", &mut app.simulation_profile_editor.options.itl2, 100.0);
                labeled_field(ui, mode, "ITL4 (tran iters/step)", &mut app.simulation_profile_editor.options.itl4, 100.0);
                labeled_field(ui, mode, "ITL5 (tran total iters)", &mut app.simulation_profile_editor.options.itl5, 100.0);
                labeled_field(ui, mode, "TRTOL (min timestep)", &mut app.simulation_profile_editor.options.min_timestep, 100.0);
                labeled_field(ui, mode, "SRCSTEPS", &mut app.simulation_profile_editor.options.srcsteps, 100.0);
                labeled_field(ui, mode, "GMINSTEPS", &mut app.simulation_profile_editor.options.gminsteps, 100.0);
            });

        // Reset to defaults
        ui.add_space(6.0);
        if ui.small_button("Reset Transient Defaults").clicked() {
            let defaults = super::profile_editor::SimOptions::default();
            app.simulation_profile_editor.options.method = defaults.method;
            app.simulation_profile_editor.options.itl1 = defaults.itl1;
            app.simulation_profile_editor.options.itl2 = defaults.itl2;
            app.simulation_profile_editor.options.itl4 = defaults.itl4;
            app.simulation_profile_editor.options.itl5 = defaults.itl5;
            app.simulation_profile_editor.options.min_timestep = defaults.min_timestep;
            app.simulation_profile_editor.options.srcsteps = defaults.srcsteps;
            app.simulation_profile_editor.options.gminsteps = defaults.gminsteps;
            app.save_simulation_settings();
        }
    });
}

// ── Convergence ────────────────────────────────────────────────────────

/// Convergence section: all tolerance and pivot settings.
fn draw_convergence_section(app: &mut NekoSpiceApp, ui: &mut egui::Ui, mode: StudioThemeMode) {
    StudioTheme::panel_frame_for(mode).show(ui, |ui| {
        section_header(ui, mode, "Convergence");
        ui.add_space(4.0);

        egui::Grid::new("convergence_grid")
            .num_columns(2)
            .spacing([8.0, 6.0])
            .show(ui, |ui| {
                labeled_field(ui, mode, "RELTOL", &mut app.simulation_profile_editor.options.reltol, 100.0);
                labeled_field(ui, mode, "ABSTOL (A)", &mut app.simulation_profile_editor.options.abstol, 100.0);
                labeled_field(ui, mode, "VNTOL (V)", &mut app.simulation_profile_editor.options.vntol, 100.0);
                labeled_field(ui, mode, "GMIN (S)", &mut app.simulation_profile_editor.options.gmin, 100.0);
                labeled_field(ui, mode, "CHGTOL (C)", &mut app.simulation_profile_editor.options.chgtol, 100.0);
                labeled_field(ui, mode, "PIVTOL", &mut app.simulation_profile_editor.options.pivtol, 100.0);
                labeled_field(ui, mode, "PIVREL", &mut app.simulation_profile_editor.options.pivrel, 100.0);
            });

        // Reset to defaults
        ui.add_space(6.0);
        if ui.small_button("Reset Convergence Defaults").clicked() {
            let defaults = super::profile_editor::SimOptions::default();
            app.simulation_profile_editor.options.reltol = defaults.reltol;
            app.simulation_profile_editor.options.abstol = defaults.abstol;
            app.simulation_profile_editor.options.vntol = defaults.vntol;
            app.simulation_profile_editor.options.gmin = defaults.gmin;
            app.simulation_profile_editor.options.chgtol = defaults.chgtol;
            app.simulation_profile_editor.options.pivtol = defaults.pivtol;
            app.simulation_profile_editor.options.pivrel = defaults.pivrel;
            app.save_simulation_settings();
        }
    });
}

// ── Output Control ─────────────────────────────────────────────────────

/// Output control section: digit precision in simulation output.
fn draw_output_section(app: &mut NekoSpiceApp, ui: &mut egui::Ui, mode: StudioThemeMode) {
    StudioTheme::panel_frame_for(mode).show(ui, |ui| {
        section_header(ui, mode, "Output Control");
        ui.add_space(4.0);

        egui::Grid::new("output_grid")
            .num_columns(2)
            .spacing([8.0, 6.0])
            .show(ui, |ui| {
                labeled_field(ui, mode, "NUMDGT (digits)", &mut app.simulation_profile_editor.options.numdgt, 100.0);
            });
    });
}

// ── Initial Conditions ─────────────────────────────────────────────────

/// Initial conditions section: .ic and .nodeset entries.
fn draw_initial_conditions_section(app: &mut NekoSpiceApp, ui: &mut egui::Ui, mode: StudioThemeMode) {
    StudioTheme::panel_frame_for(mode).show(ui, |ui| {
        section_header(ui, mode, "Initial Conditions");
        ui.add_space(4.0);

        // .ic entries
        ui.label(StudioTheme::muted_for(mode, ".ic — Node voltages"));
        ui.add_space(2.0);
        let mut remove_ic = None;
        for (i, (node, value)) in app.simulation_profile_editor.initial_conditions.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(node).desired_width(80.0).hint_text("node"));
                ui.add(egui::TextEdit::singleline(value).desired_width(80.0).hint_text("voltage"));
                if ui.small_button("×").clicked() {
                    remove_ic = Some(i);
                }
            });
        }
        if let Some(idx) = remove_ic {
            app.simulation_profile_editor.initial_conditions.remove(idx);
        }
        if ui.small_button("+ Add .ic").clicked() {
            app.simulation_profile_editor.initial_conditions.push((String::new(), String::new()));
        }

        ui.add_space(6.0);

        // .nodeset entries
        ui.label(StudioTheme::muted_for(mode, ".nodeset — Convergence hints"));
        ui.add_space(2.0);
        let mut remove_ns = None;
        for (i, (node, value)) in app.simulation_profile_editor.nodesets.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(node).desired_width(80.0).hint_text("node"));
                ui.add(egui::TextEdit::singleline(value).desired_width(80.0).hint_text("guess"));
                if ui.small_button("×").clicked() {
                    remove_ns = Some(i);
                }
            });
        }
        if let Some(idx) = remove_ns {
            app.simulation_profile_editor.nodesets.remove(idx);
        }
        if ui.small_button("+ Add .nodeset").clicked() {
            app.simulation_profile_editor.nodesets.push((String::new(), String::new()));
        }
    });
}

// ── Run Status ─────────────────────────────────────────────────────────

/// Run status summary: shows current run state and last result.
fn draw_run_status_summary(app: &NekoSpiceApp, ui: &mut egui::Ui, mode: StudioThemeMode) {
    let palette = StudioTheme::palette(mode);
    StudioTheme::panel_frame_for(mode).show(ui, |ui| {
        section_header(ui, mode, app.text(UiText::RunStatus));
        ui.add_space(4.0);

        if app.simulation_panel.active_task.is_some() {
            ui.horizontal(|ui| {
                ui.colored_label(palette.accent, "●");
                ui.label(StudioTheme::muted_for(mode, "Status:"));
                ui.label(
                    egui::RichText::new("Running")
                        .strong()
                        .color(palette.accent),
                );
            });
        } else if let Some(run) = &app.simulation_panel.last_run {
            let (color, label) = match run.metadata.status {
                RunStatus::Passed => (palette.success, "Passed"),
                RunStatus::Failed => (palette.danger, "Failed"),
            };
            ui.horizontal(|ui| {
                ui.colored_label(color, "●");
                ui.label(StudioTheme::muted_for(mode, "Status:"));
                ui.label(egui::RichText::new(label).strong().color(color));
            });
            ui.horizontal(|ui| {
                ui.label(StudioTheme::muted_for(mode, "Duration:"));
                ui.label(format!("{} ms", run.metadata.duration_ms));
            });
            ui.horizontal(|ui| {
                ui.label(StudioTheme::muted_for(mode, "Exit code:"));
                ui.label(format!("{:?}", run.metadata.exit_code));
            });
        } else if let Some(error) = &app.simulation_panel.last_error {
            ui.horizontal(|ui| {
                ui.colored_label(
                    severity_color(mode, KicadDiagnosticSeverity::Error),
                    "●",
                );
                ui.label(StudioTheme::muted_for(mode, "Status:"));
                ui.label(
                    egui::RichText::new("Error")
                        .strong()
                        .color(palette.danger),
                );
            });
            ui.label(StudioTheme::muted_for(mode, error));
        } else {
            ui.label(StudioTheme::muted_for(mode, "No simulation run yet."));
        }
    });
}

// ── Recent Runs ────────────────────────────────────────────────────────

/// Recent runs list: shows the last simulation run with pass/fail indicator.
fn draw_recent_runs(app: &NekoSpiceApp, ui: &mut egui::Ui, mode: StudioThemeMode) {
    let palette = StudioTheme::palette(mode);
    StudioTheme::panel_frame_for(mode).show(ui, |ui| {
        section_header(ui, mode, app.text(UiText::RecentRuns));
        ui.add_space(4.0);

        if let Some(run) = &app.simulation_panel.last_run {
            let (color, status_text) = match run.metadata.status {
                RunStatus::Passed => (palette.success, "Passed"),
                RunStatus::Failed => (palette.danger, "Failed"),
            };
            egui::Frame::new()
                .fill(palette.panel_soft)
                .corner_radius(4)
                .inner_margin(egui::Margin::same(8))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.colored_label(color, "●");
                        ui.vertical(|ui| {
                            ui.label(
                                egui::RichText::new(format!(
                                    "{} — {} ms",
                                    status_text, run.metadata.duration_ms
                                ))
                                .color(palette.text),
                            );
                            ui.label(
                                StudioTheme::muted_for(mode, "Last simulation run"),
                            );
                        });
                    });
                });
        } else {
            ui.label(StudioTheme::muted_for(mode, "No recent runs."));
        }
    });
}

// ── Helper ─────────────────────────────────────────────────────────────

/// Labeled text field with consistent sizing.
fn labeled_field(ui: &mut egui::Ui, mode: StudioThemeMode, label: &str, value: &mut String, width: f32) {
    ui.label(StudioTheme::muted_for(mode, label));
    ui.add(egui::TextEdit::singleline(value).desired_width(width));
    ui.end_row();
}
