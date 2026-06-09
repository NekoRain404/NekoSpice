use super::NekoSpiceApp;
use super::localization::UiText;
use super::reports_workspace_widgets::{artifact_row, report_row};
use super::theme::StudioTheme;
use crate::report_summary::GuiReportSummaryState;
use crate::waveform_summary::GuiWaveformSummaryState;
use eframe::egui;

const MEASUREMENT_LIMIT: usize = 8;
const ARTIFACT_LIMIT: usize = 10;

impl NekoSpiceApp {
    pub(super) fn draw_report_measurements_section(&self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::Measurements),
            ));
            let Some(run) = &self.simulation_panel.last_run else {
                ui.label(StudioTheme::muted_for(mode, self.text(UiText::NoRecentRun)));
                return;
            };
            let GuiWaveformSummaryState::Ready(summary) = &run.waveform else {
                ui.label(StudioTheme::muted_for(mode, self.text(UiText::NoWaveform)));
                return;
            };
            egui::Grid::new("reports_measurements_table")
                .num_columns(5)
                .spacing(egui::Vec2::new(12.0, 4.0))
                .striped(true)
                .show(ui, |ui| {
                    ui.strong(self.text(UiText::Label));
                    ui.strong("Last");
                    ui.strong("Min");
                    ui.strong("Max");
                    ui.strong("P-P");
                    ui.end_row();
                    for variable in summary.variables.iter().take(MEASUREMENT_LIMIT) {
                        ui.label(&variable.name);
                        ui.monospace(format_compact_f64(variable.last));
                        ui.monospace(format_compact_f64(variable.min));
                        ui.monospace(format_compact_f64(variable.max));
                        ui.monospace(format_compact_f64(variable.peak_to_peak));
                        ui.end_row();
                    }
                });
        });
    }

    pub(super) fn draw_report_artifacts_section(&self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::Artifacts),
            ));
            let Some(run) = &self.simulation_panel.last_run else {
                ui.label(StudioTheme::muted_for(mode, self.text(UiText::NoRecentRun)));
                return;
            };
            for artifact in run.metadata.artifacts.iter().take(ARTIFACT_LIMIT) {
                artifact_row(
                    ui,
                    &artifact.kind,
                    &artifact.path,
                    artifact.path == "report.html",
                );
            }
        });
    }

    pub(super) fn draw_report_preview_section(&self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::ReportPreview),
            ));
            let Some(run) = &self.simulation_panel.last_run else {
                ui.label(StudioTheme::muted_for(mode, self.text(UiText::NoRecentRun)));
                return;
            };
            match &run.report {
                GuiReportSummaryState::Ready(report) => {
                    report_row(ui, mode, "HTML", &report.report_file);
                    report_row(
                        ui,
                        mode,
                        "Source",
                        report.source_file.as_deref().unwrap_or("-"),
                    );
                    report_row(ui, mode, "Kind", report.source_kind.unwrap_or("report"));
                    report_row(ui, mode, "Size", &format_bytes(report.size_bytes));
                    ui.separator();
                    ui.label(StudioTheme::muted_for(
                        mode,
                        run.output_dir.display().to_string(),
                    ));
                }
                GuiReportSummaryState::Missing(message) => {
                    ui.label(StudioTheme::muted_for(mode, self.text(UiText::Missing)));
                    ui.monospace(message);
                }
            }
        });
    }

    pub(super) fn draw_report_export_section(&self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::ExportReport),
            ));
            ui.horizontal_wrapped(|ui| {
                let _ = ui.button("PDF");
                let _ = ui.button("HTML");
                let _ = ui.button("DOCX");
            });
            ui.label(StudioTheme::muted_for(
                mode,
                self.text(UiText::ReportPreview),
            ));
        });
    }
}

fn format_compact_f64(value: f64) -> String {
    if !value.is_finite() {
        return value.to_string();
    }
    let absolute = value.abs();
    if value == 0.0 {
        "0".to_string()
    } else if !(1.0e-3..1.0e4).contains(&absolute) {
        format!("{value:.3e}")
    } else {
        format!("{value:.4}")
    }
}

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;

    if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}
