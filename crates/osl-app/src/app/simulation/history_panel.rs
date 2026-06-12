//! Simulation history panel — displays a table of past runs for comparison.
//!
//! Shows run timestamp, analysis type, backend, duration, and status.
//! Users can click a run to view its output directory or re-run with
//! the same settings.

use crate::app::NekoSpiceApp;

use super::profile_editor_widgets::section_header;
use crate::app::theme::{StudioTheme, StudioThemeMode};
use eframe::egui;

impl NekoSpiceApp {
    /// Draw the simulation history panel in the profile editor right column.
    pub(crate) fn draw_history_panel(&self, ui: &mut egui::Ui, mode: StudioThemeMode) {
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            let count = self.simulation_history.len();
            section_header(
                ui,
                mode,
                &format!("Run History ({})", count),
            );
            ui.add_space(4.0);

            if self.simulation_history.is_empty() {
                ui.label(StudioTheme::muted_for(
                    mode,
                    "No simulation runs yet. Run a simulation to populate history.",
                ));
                return;
            }

            let palette = StudioTheme::palette(mode);

            // History table
            egui::Grid::new("simulation_history_table")
                .num_columns(5)
                .spacing([6.0, 4.0])
                .striped(true)
                .show(ui, |ui| {
                    // Column headers
                    ui.label(StudioTheme::muted_for(mode, "Time"));
                    ui.label(StudioTheme::muted_for(mode, "Analysis"));
                    ui.label(StudioTheme::muted_for(mode, "Backend"));
                    ui.label(StudioTheme::muted_for(mode, "Duration"));
                    ui.label(StudioTheme::muted_for(mode, "Status"));
                    ui.end_row();

                    for entry in self.simulation_history.entries() {
                        // Time
                        ui.label(
                            egui::RichText::new(entry.time_label())
                                .monospace()
                                .size(11.0)
                                .color(palette.text),
                        );
                        // Analysis
                        ui.label(
                            egui::RichText::new(entry.analysis_label())
                                .monospace()
                                .size(11.0)
                                .color(palette.text),
                        );
                        // Backend
                        ui.label(
                            egui::RichText::new(&entry.backend)
                                .size(11.0)
                                .color(palette.text_muted),
                        );
                        // Duration
                        ui.label(
                            egui::RichText::new(format!("{} ms", entry.duration_ms))
                                .monospace()
                                .size(11.0)
                                .color(palette.text),
                        );
                        // Status with colored dot
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("●").color(entry.status_color(&palette)));
                            ui.label(
                                egui::RichText::new(entry.status_label())
                                    .size(11.0)
                                    .color(entry.status_color(&palette)),
                            );
                        });
                        ui.end_row();
                    }
                });
        });
    }
}
