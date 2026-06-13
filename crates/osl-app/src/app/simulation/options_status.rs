//! Run status summary and recent runs display for the profile editor.

use super::profile_editor_widgets::section_header;
use crate::app::NekoSpiceApp;
use crate::app::localization::UiText;
use crate::app::status_strip::severity_color;
use crate::app::theme::{StudioTheme, StudioThemeMode};
use eframe::egui;
use osl_core::RunStatus;
use osl_kicad::KicadDiagnosticSeverity;

/// Draw run status summary showing current state and last result.
pub(crate) fn draw_run_status_summary(
    app: &NekoSpiceApp,
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
) {
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
                ui.colored_label(severity_color(mode, KicadDiagnosticSeverity::Error), "●");
                ui.label(StudioTheme::muted_for(mode, "Status:"));
                ui.label(egui::RichText::new("Error").strong().color(palette.danger));
            });
            ui.label(StudioTheme::muted_for(mode, error));
            ui.add_space(4.0);
            ui.label(StudioTheme::section_title_for(mode, "Suggested Actions"));
            ui.add_space(2.0);
            for suggestion in error_suggestions(error) {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("→").color(palette.accent).size(11.0));
                    ui.label(StudioTheme::muted_for(mode, suggestion));
                });
            }
        } else {
            ui.label(StudioTheme::muted_for(mode, "No simulation run yet."));
        }
    });
}

/// Draw recent runs list with pass/fail indicator.
pub(crate) fn draw_recent_runs(app: &NekoSpiceApp, ui: &mut egui::Ui, mode: StudioThemeMode) {
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
                                    "{} -- {} ms",
                                    status_text, run.metadata.duration_ms
                                ))
                                .color(palette.text),
                            );
                            ui.label(StudioTheme::muted_for(mode, "Last simulation run"));
                        });
                    });
                });
        } else {
            ui.label(StudioTheme::muted_for(mode, "No recent runs."));
        }
    });
}

/// Suggest actionable steps based on common simulation errors.
fn error_suggestions(error: &str) -> Vec<&'static str> {
    let lower = error.to_lowercase();
    let mut suggestions = Vec::new();

    if lower.contains("no such file") || lower.contains("not found") {
        suggestions.push("Check that ngspice/Xyce is installed and in PATH");
        suggestions.push("Verify the executable path in Settings");
    }
    if lower.contains("convergence") || lower.contains("singular matrix") {
        suggestions.push("Try the 'Convergence Aid' preset");
        suggestions.push("Increase SRCSTEPS or GMINSTEPS in solver settings");
        suggestions.push("Check for floating nodes or missing ground connections");
    }
    if lower.contains("netlist") || lower.contains("parse") {
        suggestions.push("Check the netlist preview for syntax errors");
        suggestions.push("Verify all components have valid SPICE models");
    }
    if lower.contains("timeout") || lower.contains("iteration") {
        suggestions.push("Increase ITL4 or ITL5 iteration limits");
        suggestions.push("Try the 'Fast' preset for quicker convergence");
    }
    if lower.contains("model") || lower.contains("subcircuit") {
        suggestions.push("Import vendor models via the Library workspace");
        suggestions.push("Check that .include/.lib paths are correct in the schematic");
    }
    if suggestions.is_empty() {
        suggestions.push("Check the raw log for detailed error information");
        suggestions.push("Try the 'Convergence Aid' preset and re-run");
    }
    suggestions
}
