//! Run comparison — allows selecting two historical simulation runs
//! and comparing their settings, duration, and waveform data side by side.
//!
//! This helps users understand how parameter changes affect simulation
//! results without switching between different views.

use crate::app::NekoSpiceApp;
use crate::app::theme::StudioTheme;
use eframe::egui;

/// State for the run comparison feature.
#[derive(Debug, Default)]
pub(crate) struct RunCompareState {
    /// Index of the first run to compare (in history, 0 = most recent).
    pub run_a: Option<usize>,
    /// Index of the second run to compare.
    pub run_b: Option<usize>,
    /// Whether the comparison panel is visible.
    pub visible: bool,
}

impl NekoSpiceApp {
    /// Draw the run comparison panel in the profile editor right column.
    ///
    /// Shows a dropdown to select two runs, then displays a side-by-side
    /// comparison of their key settings and results.
    pub(crate) fn draw_run_compare_panel(
        &mut self,
        ui: &mut egui::Ui,
        mode: crate::app::theme::StudioThemeMode,
    ) {
        let palette = StudioTheme::palette(mode);
        let history_count = self.simulation_history.len();

        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(mode, "Compare Runs"));
            ui.add_space(4.0);

            if history_count < 2 {
                ui.label(StudioTheme::muted_for(
                    mode,
                    "Need at least 2 runs to compare.",
                ));
                return;
            }

            // Toggle visibility
            let mut visible = self.run_compare.visible;
            ui.checkbox(&mut visible, "Enable run comparison");
            self.run_compare.visible = visible;

            if !visible {
                return;
            }

            ui.add_space(4.0);

            // Run A selector
            ui.label(StudioTheme::muted_for(mode, "Run A (baseline)"));
            let run_a = self.run_compare.run_a.unwrap_or(0).min(history_count - 1);
            egui::ComboBox::from_id_salt("compare_run_a")
                .selected_text(format_run_label(&self.simulation_history, run_a))
                .show_ui(ui, |ui| {
                    for i in 0..history_count {
                        let label = format_run_label(&self.simulation_history, i);
                        let mut selected = Some(run_a);
                        if ui
                            .selectable_value(&mut selected, Some(i), &label)
                            .changed()
                        {
                            self.run_compare.run_a = selected;
                        }
                    }
                });

            ui.add_space(4.0);

            // Run B selector
            ui.label(StudioTheme::muted_for(mode, "Run B (compare)"));
            let default_b = if run_a == 0 { 1 } else { 0 };
            let run_b = self
                .run_compare
                .run_b
                .unwrap_or(default_b)
                .min(history_count - 1);
            egui::ComboBox::from_id_salt("compare_run_b")
                .selected_text(format_run_label(&self.simulation_history, run_b))
                .show_ui(ui, |ui| {
                    for i in 0..history_count {
                        let label = format_run_label(&self.simulation_history, i);
                        let mut selected = Some(run_b);
                        if ui
                            .selectable_value(&mut selected, Some(i), &label)
                            .changed()
                        {
                            self.run_compare.run_b = selected;
                        }
                    }
                });

            ui.add_space(8.0);

            // Side-by-side comparison
            if let (Some(a), Some(b)) = (self.run_compare.run_a, self.run_compare.run_b)
                && a != b
                && a < history_count
                && b < history_count
            {
                let entries = self.simulation_history.entries();
                let ea = &entries[a];
                let eb = &entries[b];

                ui.separator();
                ui.add_space(4.0);

                egui::Grid::new("run_compare_grid")
                    .num_columns(3)
                    .spacing([8.0, 4.0])
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label(StudioTheme::muted_for(mode, "Property"));
                        ui.label(StudioTheme::muted_for(mode, "Run A"));
                        ui.label(StudioTheme::muted_for(mode, "Run B"));
                        ui.end_row();

                        compare_row(
                            ui,
                            mode,
                            "Analysis",
                            &ea.analysis_label(),
                            &eb.analysis_label(),
                            palette,
                        );
                        compare_row(ui, mode, "Backend", &ea.backend, &eb.backend, palette);
                        compare_row(
                            ui,
                            mode,
                            "Settings",
                            &ea.settings_summary,
                            &eb.settings_summary,
                            palette,
                        );
                        compare_row(
                            ui,
                            mode,
                            "Duration",
                            &format!("{} ms", ea.duration_ms),
                            &format!("{} ms", eb.duration_ms),
                            palette,
                        );
                        compare_status_row(
                            ui,
                            mode,
                            "Status",
                            ea.status_label(),
                            eb.status_label(),
                            ea,
                            eb,
                            &palette,
                        );
                        compare_row(
                            ui,
                            mode,
                            "Time",
                            &ea.time_label(),
                            &eb.time_label(),
                            palette,
                        );
                    });
            }
        });
    }
}

/// Format a history entry label for the comparison dropdown.
fn format_run_label(
    history: &crate::app::simulation::history::SimulationHistory,
    index: usize,
) -> String {
    let entry = &history.entries()[index];
    format!(
        "#{} {} ({}, {}ms)",
        index + 1,
        entry.analysis_label(),
        entry.status_label(),
        entry.duration_ms,
    )
}

/// Draw a comparison row with matching/different highlighting.
fn compare_row(
    ui: &mut egui::Ui,
    mode: crate::app::theme::StudioThemeMode,
    label: &str,
    val_a: &str,
    val_b: &str,
    palette: crate::app::theme::StudioPalette,
) {
    ui.label(StudioTheme::muted_for(mode, label));
    let color_a = palette.text;
    let color_b = if val_a == val_b {
        palette.text_muted
    } else {
        palette.warning
    };
    ui.label(egui::RichText::new(val_a).monospace().color(color_a));
    ui.label(egui::RichText::new(val_b).monospace().color(color_b));
    ui.end_row();
}

/// Draw a status comparison row with colored indicators.
#[allow(clippy::too_many_arguments)]
fn compare_status_row(
    ui: &mut egui::Ui,
    mode: crate::app::theme::StudioThemeMode,
    label: &str,
    status_a: &str,
    status_b: &str,
    entry_a: &crate::app::simulation::history::SimulationHistoryEntry,
    entry_b: &crate::app::simulation::history::SimulationHistoryEntry,
    palette: &crate::app::theme::StudioPalette,
) {
    ui.label(StudioTheme::muted_for(mode, label));
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("●")
                .color(entry_a.status_color(palette))
                .size(10.0),
        );
        ui.label(status_a);
    });
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("●")
                .color(entry_b.status_color(palette))
                .size(10.0),
        );
        ui.label(status_b);
    });
    ui.end_row();
}
