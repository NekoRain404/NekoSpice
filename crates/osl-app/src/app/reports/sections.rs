use crate::app::NekoSpiceApp;
use crate::app::localization::UiText;
use super::preview::{draw_curve, draw_grid};
use super::widgets::{artifact_row, export_toggle, formula_token, report_row};
use crate::app::theme::StudioTheme;
use crate::report_summary::GuiReportSummaryState;
use crate::waveform_summary::GuiWaveformSummaryState;
use eframe::egui;

const MEASUREMENT_LIMIT: usize = 8;
const ARTIFACT_LIMIT: usize = 10;

impl NekoSpiceApp {
    /// draw report measurements section。
    pub(crate) fn draw_report_measurements_section(&self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::Measurements),
            ));
            let Some(run) = &self.simulation_panel.last_run else {
                self.draw_reference_measurement_rows(ui);
                return;
            };
            let GuiWaveformSummaryState::Ready(summary) = &run.waveform else {
                self.draw_reference_measurement_rows(ui);
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

    /// draw report plot annotation section。
    pub(crate) fn draw_report_plot_annotation_section(&self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let palette = StudioTheme::palette(mode);
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::Plots),
            ));
            let (rect, _) = ui.allocate_exact_size(
                egui::vec2(ui.available_width().max(260.0), 220.0),
                egui::Sense::hover(),
            );
            let painter = ui.painter_at(rect);
            painter.rect_filled(rect, egui::CornerRadius::same(4), palette.canvas);
            painter.rect_stroke(
                rect,
                egui::CornerRadius::same(4),
                egui::Stroke::new(1.0, palette.border),
                egui::StrokeKind::Inside,
            );
            draw_grid(&painter, rect, palette.border);
            draw_curve(&painter, rect, palette.accent, 0.0);
            draw_curve(&painter, rect, palette.warning, 0.8);
            painter.text(
                rect.left_top() + egui::vec2(12.0, 10.0),
                egui::Align2::LEFT_TOP,
                "Bode Plot / Loop Gain",
                egui::FontId::proportional(13.0),
                palette.text,
            );
            painter.text(
                rect.center_top() + egui::vec2(24.0, 72.0),
                egui::Align2::CENTER_CENTER,
                "UGF: 2.14 MHz",
                egui::FontId::proportional(12.0),
                palette.text,
            );
        });
    }

    /// draw report formula editor section。
    pub(crate) fn draw_report_formula_editor_section(&self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::FormulaEditor),
            ));
            ui.monospace("phase_margin = 180 + phase_at(gain(dB(v(out)/v(in))), freq(ugf))");
            ui.separator();
            ui.horizontal_wrapped(|ui| {
                for token in [
                    "avg()", "max()", "min()", "pp()", "rms()", "db()", "phase()",
                ] {
                    formula_token(ui, mode, token);
                }
            });
        });
    }

    /// draw report details section。
    pub(crate) fn draw_report_details_section(&self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::RunContext),
            ));
            report_row(ui, mode, self.text(UiText::Backend), "ngspice 42");
            report_row(ui, mode, self.text(UiText::Solver), "adaptive gear");
            report_row(ui, mode, self.text(UiText::TemperatureSweep), "27 C");
            report_row(
                ui,
                mode,
                self.text(UiText::ReportTitle),
                "Precision OpAmp Report",
            );
        });
    }

    /// draw report artifacts section。
    pub(crate) fn draw_report_artifacts_section(&self, ui: &mut egui::Ui) {
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

    /// draw report preview section。
    pub(crate) fn draw_report_preview_section(&self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::ReportPreview),
            ));
            match self
                .simulation_panel
                .last_run
                .as_ref()
                .map(|run| &run.report)
            {
                Some(GuiReportSummaryState::Ready(report)) => {
                    report_row(ui, mode, "HTML", &report.report_file);
                    report_row(
                        ui,
                        mode,
                        "Source",
                        report.source_file.as_deref().unwrap_or("-"),
                    );
                    report_row(ui, mode, "Kind", report.source_kind.unwrap_or("report"));
                    report_row(ui, mode, "Size", &format_bytes(report.size_bytes));
                }
                Some(GuiReportSummaryState::Missing(message)) => {
                    ui.label(StudioTheme::muted_for(mode, self.text(UiText::Missing)));
                    ui.monospace(message);
                }
                None => {
                    report_row(
                        ui,
                        mode,
                        self.text(UiText::Templates),
                        "Engineering Report v2",
                    );
                    report_row(ui, mode, self.text(UiText::PassRate), "100%");
                    report_row(ui, mode, self.text(UiText::TotalMeasurements), "28");
                }
            }
            ui.separator();
            self.draw_report_preview_mock_page(ui);
        });
    }

    /// draw report export section。
    pub(crate) fn draw_report_export_section(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::ExportReport),
            ));
            ui.horizontal_wrapped(|ui| {
                if ui.button("HTML").on_hover_text("Export simulation report as HTML").clicked() {
                    self.export_report_html();
                }
                if ui.button("CSV").on_hover_text("Export measurement data as CSV").clicked() {
                    self.export_measurements_csv();
                }
                if ui.button("Markdown").on_hover_text("Export report as Markdown").clicked() {
                    self.export_report_markdown();
                }
                if ui.button("Netlist").on_hover_text("Export SPICE netlist to file").clicked() {
                    self.export_netlist_dialog();
                }
            });
            report_row(
                ui,
                mode,
                self.text(UiText::ReportTitle),
                "Precision OpAmp Performance",
            );
            report_row(ui, mode, self.text(UiText::PageSize), "A4");
            ui.separator();
            export_toggle(ui, mode, self.text(UiText::CoverPage), true);
            export_toggle(ui, mode, self.text(UiText::StatisticalSummary), true);
            export_toggle(ui, mode, self.text(UiText::Measurements), true);
            export_toggle(ui, mode, self.text(UiText::Plots), true);
            export_toggle(ui, mode, self.text(UiText::Appendix), false);
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
