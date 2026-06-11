//! Right column of the simulation profile editor:
//! - Simulation Options (temperature, iterations, tolerances)
//! - Run Status summary
//! - Recent runs list

use super::NekoSpiceApp;
use super::localization::UiText;
use super::simulation_profile_editor_widgets::section_header;
use super::status_strip::severity_color;
use super::theme::{StudioTheme, StudioThemeMode};
use eframe::egui;
use osl_kicad::KicadDiagnosticSeverity;
use osl_core::RunStatus;

/// Draw the full right-column options panel.
pub(super) fn draw_profile_options(app: &mut NekoSpiceApp, ui: &mut egui::Ui) {
    let mode = app.theme_mode();
    draw_simulation_options(app, ui, mode);
    ui.add_space(8.0);
    draw_run_status_summary(app, ui, mode);
    ui.add_space(8.0);
    draw_recent_runs(app, ui, mode);
}

/// Simulation Options: temperature, max iterations, min timestep, tolerances.
fn draw_simulation_options(app: &mut NekoSpiceApp, ui: &mut egui::Ui, mode: StudioThemeMode) {
    StudioTheme::panel_frame_for(mode).show(ui, |ui| {
        section_header(ui, mode, app.text(UiText::SimulationOptions));
        ui.add_space(4.0);

        

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

        ui.add_space(6.0);
        ui.label(StudioTheme::section_title_for(mode, "Tolerances"));
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

        // Current status indicator
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

/// Recent runs list: shows the last few simulation runs.
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
