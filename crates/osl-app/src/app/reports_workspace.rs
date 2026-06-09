use super::NekoSpiceApp;
use super::localization::UiText;
use super::reports_workspace_widgets::report_metric_card;
use super::theme::StudioTheme;
use eframe::egui;

impl NekoSpiceApp {
    pub(super) fn draw_reports_center_workspace(&mut self, ui: &mut egui::Ui) {
        self.poll_simulation_task();
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            self.draw_reports_workspace_header(ui);
            ui.add_space(8.0);
            self.draw_report_summary_metrics(ui);
            ui.add_space(8.0);
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_width((ui.available_width() * 0.56).max(380.0));
                    self.draw_report_measurements_section(ui);
                    ui.add_space(8.0);
                    self.draw_report_artifacts_section(ui);
                });
                ui.add_space(10.0);
                ui.vertical(|ui| {
                    ui.set_width(ui.available_width().max(260.0));
                    self.draw_report_preview_section(ui);
                    ui.add_space(8.0);
                    self.draw_report_export_section(ui);
                });
            });
        });
    }

    fn draw_reports_workspace_header(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.heading(self.text(UiText::ReportsResults));
                ui.label(StudioTheme::muted_for(
                    mode,
                    self.text(UiText::ReportsCaption),
                ));
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                let running = self.simulation_panel.active_task.is_some();
                if ui
                    .add_enabled(
                        self.document.is_some() && !running,
                        egui::Button::new(self.text(UiText::RunSimulation)),
                    )
                    .clicked()
                {
                    self.run_simulation_from_panel();
                }
            });
        });
    }

    fn draw_report_summary_metrics(&self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let run = self.simulation_panel.last_run.as_ref();
        ui.columns(2, |columns| {
            report_metric_card(
                &mut columns[0],
                mode,
                self.text(UiText::TotalMeasurements),
                &measurement_count_text(run),
                self.text(UiText::Measurements),
            );
            report_metric_card(
                &mut columns[1],
                mode,
                self.text(UiText::Artifacts),
                &run.map(|run| run.metadata.artifacts.len().to_string())
                    .unwrap_or_else(|| "0".to_string()),
                self.text(UiText::RunOutput),
            );
        });
        ui.add_space(8.0);
        ui.columns(2, |columns| {
            report_metric_card(
                &mut columns[0],
                mode,
                self.text(UiText::ReportPreview),
                report_state_text(run),
                "HTML",
            );
            report_metric_card(
                &mut columns[1],
                mode,
                self.text(UiText::LastRun),
                &run.map(|run| format!("{} ms", run.metadata.duration_ms))
                    .unwrap_or_else(|| self.text(UiText::NoRecentRun).to_string()),
                self.text(UiText::Backend),
            );
        });
    }
}

fn measurement_count_text(run: Option<&crate::simulation::GuiSimulationRun>) -> String {
    run.and_then(|run| match &run.waveform {
        crate::waveform_summary::GuiWaveformSummaryState::Ready(summary) => {
            Some(summary.variables.len().to_string())
        }
        _ => None,
    })
    .unwrap_or_else(|| "0".to_string())
}

fn report_state_text(run: Option<&crate::simulation::GuiSimulationRun>) -> &'static str {
    match run.map(|run| &run.report) {
        Some(crate::report_summary::GuiReportSummaryState::Ready(_)) => "Ready",
        Some(crate::report_summary::GuiReportSummaryState::Missing(_)) => "Missing",
        None => "No run",
    }
}
