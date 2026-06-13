//! Waveform workspace side panels — cursor readout, run comparison,
//! and export controls. Extracted from workspace_sections.rs to keep
//! each file under 300 lines.

use super::preview::format_compact_f64;
use super::workspace_widgets::{cursor_row, waveform_summary_card};
use crate::app::NekoSpiceApp;
use crate::app::localization::UiText;
use crate::app::theme::StudioTheme;
use eframe::egui;

impl NekoSpiceApp {
    /// Cursor readout panel — shows A/B/Delta values for the selected signal.
    pub(crate) fn draw_waveform_cursor_panel(&self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::Cursors),
            ));
            let Some(summary) = self.current_waveform_summary() else {
                ui.label(StudioTheme::muted_for(mode, self.text(UiText::NoWaveform)));
                return;
            };
            let selected = self.simulation_panel.selected_waveform_signal.as_deref();
            let variable = selected.and_then(|signal| summary.variable_summary_for_signal(signal));
            if let Some(variable) = variable {
                cursor_row(
                    ui,
                    mode,
                    "A",
                    &variable.name,
                    &format_compact_f64(variable.first),
                );
                cursor_row(
                    ui,
                    mode,
                    "B",
                    &variable.name,
                    &format_compact_f64(variable.last),
                );
                cursor_row(
                    ui,
                    mode,
                    "Delta",
                    &variable.name,
                    &format_compact_f64(variable.peak_to_peak),
                );
            } else {
                ui.label(StudioTheme::muted_for(mode, self.text(UiText::NoSelection)));
            }
        });
    }

    /// Run comparison panel — shows the baseline run from the last simulation.
    pub(crate) fn draw_waveform_compare_panel(&self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::ComparisonManager),
            ));
            let Some(run) = &self.simulation_panel.last_run else {
                ui.label(StudioTheme::muted_for(mode, self.text(UiText::NoRecentRun)));
                return;
            };
            waveform_summary_card(
                ui,
                mode,
                self.text(UiText::BaselineRun),
                run.metadata.status.as_str(),
                &run.output_dir.display().to_string(),
            );
        });
    }

    /// Export panel — buttons for CSV, report, and raw waveform export.
    pub(crate) fn draw_waveform_export_panel(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::ExportShare),
            ));
            ui.vertical(|ui| {
                if ui.button(self.text(UiText::ExportWaveforms)).clicked() {
                    if let Some(run) = &self.simulation_panel.last_run {
                        self.status_message = Some(format!(
                            "Waveforms exported to: {}",
                            run.output_dir.display()
                        ));
                    } else {
                        self.status_message = Some("No simulation data to export".to_string());
                    }
                }
                if ui.button(self.text(UiText::ExportCsv)).clicked() {
                    self.export_waveform_csv();
                }
                if ui.button(self.text(UiText::ExportReport)).clicked() {
                    if let Some(run) = &self.simulation_panel.last_run {
                        let html = nsp_sim::run_report_html(&run.metadata);
                        let path = run.output_dir.join("report.html");
                        if let Err(e) = std::fs::write(&path, &html) {
                            self.status_message = Some(format!("Export failed: {}", e));
                        } else {
                            self.status_message = Some(format!("Report: {}", path.display()));
                        }
                    } else {
                        self.status_message = Some("No simulation data to export".to_string());
                    }
                }
            });
        });
    }

    /// Export current waveform data to CSV file using a native save dialog.
    pub(crate) fn export_waveform_csv(&mut self) {
        let Some(run) = &self.simulation_panel.last_run else {
            self.status_message = Some("No simulation data to export".to_string());
            return;
        };
        let raw_path = run.output_dir.join("waveform.raw");
        let Ok(waveform) = nsp_waveform::read_ngspice_raw(&raw_path) else {
            self.status_message = Some("Failed to read waveform data".to_string());
            return;
        };
        let Ok(csv_content) = waveform.to_csv() else {
            self.status_message = Some("Failed to convert waveform to CSV".to_string());
            return;
        };
        let dialog = rfd::FileDialog::new()
            .add_filter("CSV", &["csv"])
            .set_file_name("waveform.csv");
        if let Some(path) = dialog.save_file() {
            match std::fs::write(&path, &csv_content) {
                Ok(()) => {
                    self.status_message = Some(format!(
                        "CSV exported: {} ({} signals, {} points)",
                        path.display(),
                        waveform.variables().len(),
                        waveform.point_count(),
                    ))
                }
                Err(e) => self.status_message = Some(format!("Export failed: {e}")),
            }
        }
    }
}
