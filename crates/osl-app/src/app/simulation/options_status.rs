//! Run status summary and recent runs display for the profile editor.

use crate::app::NekoSpiceApp;
use crate::app::localization::UiText;
use super::profile_editor_widgets::section_header;
use crate::app::status_strip::severity_color;
use crate::app::theme::{StudioTheme, StudioThemeMode};
use eframe::egui;
use osl_kicad::KicadDiagnosticSeverity;
use osl_core::RunStatus;

/// Draw run status summary showing current state and last result.
pub(crate) fn draw_run_status_summary(app: &NekoSpiceApp, ui: &mut egui::Ui, mode: StudioThemeMode) {
    let palette = StudioTheme::palette(mode);
    StudioTheme::panel_frame_for(mode).show(ui, |ui| {
        section_header(ui, mode, app.text(UiText::RunStatus));
        ui.add_space(4.0);

        if app.simulation_panel.active_task.is_some() {
            ui.horizontal(|ui| {
                ui.colored_label(palette.accent, "●");
                ui.label(StudioTheme::muted_for(mode, "Status:"));
                ui.label(egui::RichText::new("Running").strong().color(palette.accent));
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
                            ui.label(egui::RichText::new(format!(
                                "{} -- {} ms", status_text, run.metadata.duration_ms
                            )).color(palette.text));
                            ui.label(StudioTheme::muted_for(mode, "Last simulation run"));
                        });
                    });
                });
        } else {
            ui.label(StudioTheme::muted_for(mode, "No recent runs."));
        }
    });
}
