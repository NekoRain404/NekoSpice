//! Waveform measurements and run statistics sections.
//!
//! Split from workspace_sections.rs to keep files under 300 lines.
//! Provides measurement table and run statistics panel for the waveform workspace.

use super::workspace_widgets::{MeasurementTableLabels, measurement_table, run_stat_row};
use crate::app::NekoSpiceApp;
use crate::app::localization::UiText;
use crate::app::theme::StudioTheme;
use eframe::egui;

/// Maximum number of measurement rows to display before collapsing.
const MEASUREMENT_LIMIT: usize = 8;

impl NekoSpiceApp {
    /// Draw the measurements table showing signal statistics (last, avg, rms, P-P).
    pub(crate) fn draw_waveform_measurements_section(&self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::Measurements),
            ));
            let Some(summary) = self.current_waveform_summary() else {
                ui.label(StudioTheme::muted_for(mode, self.text(UiText::NoWaveform)));
                return;
            };
            let labels = MeasurementTableLabels {
                signal: self.text(UiText::Signal),
                last: self.text(UiText::LastValue),
                average: self.text(UiText::Average),
                rms: self.text(UiText::Rms),
                peak_to_peak: self.text(UiText::PeakToPeak),
                samples: self.text(UiText::Samples),
                more_variables: self.text(UiText::MoreVariables),
            };
            measurement_table(ui, &labels, &summary.variables, MEASUREMENT_LIMIT);
        });
    }

    /// Draw the run statistics panel showing backend, duration, and artifact counts.
    pub(crate) fn draw_waveform_run_statistics_section(&self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::RunStatistics),
            ));
            let Some(run) = &self.simulation_panel.last_run else {
                ui.label(StudioTheme::muted_for(mode, self.text(UiText::NoRecentRun)));
                return;
            };
            let point_count = self
                .current_waveform_summary()
                .map(|summary| summary.point_count.to_string())
                .unwrap_or_else(|| "0".to_string());
            let variable_count = self
                .current_waveform_summary()
                .map(|summary| summary.variable_count.to_string())
                .unwrap_or_else(|| "0".to_string());
            run_stat_row(
                ui,
                mode,
                self.text(UiText::StatusConsole),
                run.metadata.status.as_str(),
            );
            run_stat_row(ui, mode, self.text(UiText::Backend), &run.metadata.backend);
            run_stat_row(
                ui,
                mode,
                self.text(UiText::LastRun),
                &format!("{} ms", run.metadata.duration_ms),
            );
            run_stat_row(ui, mode, self.text(UiText::Points), &point_count);
            run_stat_row(ui, mode, self.text(UiText::Signals), &variable_count);
            run_stat_row(
                ui,
                mode,
                self.text(UiText::Artifacts),
                &run.metadata.artifacts.len().to_string(),
            );
        });
    }
}
