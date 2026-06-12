//! Right column of the simulation profile editor — three sections:
//!
//! 1. **Simulation Options** — temperature, max iterations, min timestep,
//!    SPICE integration method, and solver tolerances (RELTOL, ABSTOL, VNTOL).
//! 2. **Run Status** — current run state, duration, exit code, and error messages.
//! 3. **Recent Runs** — last simulation run summary with pass/fail indicator.

use crate::app::NekoSpiceApp;
use crate::app::localization::UiText;
use super::profile_editor_widgets::section_header;
use crate::app::status_strip::severity_color;
use crate::app::theme::{StudioTheme, StudioThemeMode};
use eframe::egui;
use osl_kicad::KicadDiagnosticSeverity;
use osl_core::RunStatus;

/// SPICE integration method options displayed in the profile editor.
/// These correspond to ngspice `.options method=...` settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SpiceMethod {
    Gear,
    Trap,
}

impl SpiceMethod {
    /// Display label for the method selector button.
    fn label(self) -> &'static str {
        match self {
            Self::Gear => "Gear",
            Self::Trap => "Trap",
        }
    }
}

/// Draw the complete right-column options panel.
pub(crate) fn draw_profile_options(app: &mut NekoSpiceApp, ui: &mut egui::Ui) {
    let mode = app.theme_mode();
    draw_simulation_options(app, ui, mode);
    ui.add_space(8.0);
    draw_run_status_summary(app, ui, mode);
    ui.add_space(8.0);
    draw_recent_runs(app, ui, mode);
}

/// Simulation Options panel: temperature, max iterations, min timestep,
/// SPICE method selector, and solver tolerances.
fn draw_simulation_options(app: &mut NekoSpiceApp, ui: &mut egui::Ui, mode: StudioThemeMode) {
    let palette = StudioTheme::palette(mode);
    StudioTheme::panel_frame_for(mode).show(ui, |ui| {
        section_header(ui, mode, app.text(UiText::SimulationOptions));
        ui.add_space(4.0);

        // Primary simulation parameters grid
        egui::Grid::new("sim_options_grid")
            .num_columns(2)
            .spacing([8.0, 6.0])
            .show(ui, |ui| {
                ui.label(StudioTheme::muted_for(mode, "Temperature (°C)"));
                ui.add(egui::TextEdit::singleline(
                    &mut app.simulation_profile_editor.options.temperature,
                )
                .desired_width(100.0));
                ui.end_row();

                ui.label(StudioTheme::muted_for(mode, "Max Iterations"));
                ui.add(egui::TextEdit::singleline(
                    &mut app.simulation_profile_editor.options.max_iterations,
                )
                .desired_width(100.0));
                ui.end_row();

                ui.label(StudioTheme::muted_for(mode, "Min Timestep (s)"));
                ui.add(egui::TextEdit::singleline(
                    &mut app.simulation_profile_editor.options.min_timestep,
                )
                .desired_width(100.0));
                ui.end_row();
            });

        // SPICE integration method selector
        ui.add_space(6.0);
        ui.label(StudioTheme::section_title_for(mode, "Integration Method"));
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            for method in [SpiceMethod::Gear, SpiceMethod::Trap] {
                let active = app.simulation_profile_editor.options.method == method.label();
                let btn = if active {
                    egui::Button::new(
                        egui::RichText::new(method.label())
                            .strong()
                            .color(palette.text),
                    )
                    .fill(palette.accent_soft)
                    .stroke(egui::Stroke::new(1.0, palette.accent))
                } else {
                    egui::Button::new(
                        egui::RichText::new(method.label())
                            .color(palette.text_muted),
                    )
                    .fill(palette.panel_soft)
                    .stroke(egui::Stroke::new(1.0, palette.border))
                };
                if ui.add(btn).clicked() {
                    app.simulation_profile_editor.options.method = method.label().to_string();
                }
            }
        });

        // Solver tolerances section
        ui.add_space(6.0);
        ui.label(StudioTheme::section_title_for(mode, "Solver Tolerances"));
        ui.add_space(4.0);

        egui::Grid::new("tolerances_grid")
            .num_columns(2)
            .spacing([8.0, 6.0])
            .show(ui, |ui| {
                ui.label(StudioTheme::muted_for(mode, "RELTOL"));
                ui.add(egui::TextEdit::singleline(
                    &mut app.simulation_profile_editor.options.reltol,
                )
                .desired_width(100.0));
                ui.end_row();

                ui.label(StudioTheme::muted_for(mode, "ABSTOL"));
                ui.add(egui::TextEdit::singleline(
                    &mut app.simulation_profile_editor.options.abstol,
                )
                .desired_width(100.0));
                ui.end_row();

                ui.label(StudioTheme::muted_for(mode, "VNTOL"));
                ui.add(egui::TextEdit::singleline(
                    &mut app.simulation_profile_editor.options.vntol,
                )
                .desired_width(100.0));
                ui.end_row();
            });
    });
}

/// Run status summary: shows current run state and last result.
fn draw_run_status_summary(app: &NekoSpiceApp, ui: &mut egui::Ui, mode: StudioThemeMode) {
    let palette = StudioTheme::palette(mode);
    StudioTheme::panel_frame_for(mode).show(ui, |ui| {
        section_header(ui, mode, app.text(UiText::RunStatus));
        ui.add_space(4.0);

        if app.simulation_panel.active_task.is_some() {
            // Simulation is currently running — show animated status
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
            // Show last run result with color-coded pass/fail
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
            // Show error state
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
            // Rendered as a card with status indicator
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
