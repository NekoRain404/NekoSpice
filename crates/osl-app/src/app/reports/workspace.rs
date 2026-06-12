use crate::app::NekoSpiceApp;
use crate::app::localization::UiText;
use super::state::ReportsTab;
use super::widgets::report_metric_card;
use crate::app::theme::StudioTheme;
use eframe::egui;

impl NekoSpiceApp {
    /// draw reports center workspace。
    pub(crate) fn draw_reports_center_workspace(&mut self, ui: &mut egui::Ui) {
        self.poll_simulation_task();
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("reports_center_scroll")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    self.draw_reports_workspace_header(ui);
                    ui.add_space(10.0);
                    self.draw_reports_tabs(ui);
                    ui.add_space(10.0);
                    self.draw_report_summary_metrics(ui);
                    ui.add_space(10.0);
                    self.draw_reports_tab_body(ui);
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
                    .unwrap_or_else(|| "6".to_string()),
                self.text(UiText::RunOutput),
            );
        });
        ui.add_space(10.0);
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
                    .unwrap_or_else(|| "No run".to_string()),
                self.text(UiText::Backend),
            );
        });
    }

    fn draw_reports_tabs(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            for tab in ReportsTab::ALL {
                let label = self.text(tab.text_key());
                ui.selectable_value(&mut self.reports_workspace.active_tab, tab, label);
            }
        });
    }

    fn draw_reports_tab_body(&mut self, ui: &mut egui::Ui) {
        match self.reports_workspace.active_tab {
            ReportsTab::Overview | ReportsTab::Measurements => {
                self.draw_report_measurement_studio(ui)
            }
            ReportsTab::Plots => self.draw_report_plot_studio(ui),
            ReportsTab::Builder | ReportsTab::Templates | ReportsTab::ExportHistory => {
                self.draw_report_builder_studio(ui)
            }
        }
    }

    fn draw_report_measurement_studio(&self, ui: &mut egui::Ui) {
        if ui.available_width() >= 820.0 {
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_width((ui.available_width() * 0.58).max(420.0));
                    self.draw_report_measurements_section(ui);
                    ui.add_space(10.0);
                    self.draw_report_formula_editor_section(ui);
                });
                ui.add_space(10.0);
                ui.vertical(|ui| {
                    ui.set_width(ui.available_width().max(260.0));
                    self.draw_report_plot_annotation_section(ui);
                    ui.add_space(10.0);
                    self.draw_report_details_section(ui);
                });
            });
        } else {
            self.draw_report_measurements_section(ui);
            ui.add_space(10.0);
            self.draw_report_plot_annotation_section(ui);
            ui.add_space(10.0);
            self.draw_report_formula_editor_section(ui);
            ui.add_space(10.0);
            self.draw_report_details_section(ui);
        }
    }

    fn draw_report_plot_studio(&self, ui: &mut egui::Ui) {
        self.draw_report_plot_annotation_section(ui);
        ui.add_space(10.0);
        self.draw_report_measurements_section(ui);
    }

    fn draw_report_builder_studio(&mut self, ui: &mut egui::Ui) {
        if ui.available_width() >= 820.0 {
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_width((ui.available_width() * 0.58).max(420.0));
                    self.draw_report_artifacts_section(ui);
                    ui.add_space(10.0);
                    self.draw_report_export_section(ui);
                });
                ui.add_space(10.0);
                ui.vertical(|ui| {
                    ui.set_width(ui.available_width().max(260.0));
                    self.draw_report_preview_section(ui);
                });
            });
        } else {
            self.draw_report_artifacts_section(ui);
            ui.add_space(10.0);
            self.draw_report_export_section(ui);
            ui.add_space(10.0);
            self.draw_report_preview_section(ui);
        }
    }
}

fn measurement_count_text(run: Option<&crate::simulation::GuiSimulationRun>) -> String {
    run.and_then(|run| match &run.waveform {
        crate::waveform_summary::GuiWaveformSummaryState::Ready(summary) => {
            Some(summary.variables.len().to_string())
        }
        _ => None,
    })
    .unwrap_or_else(|| "28".to_string())
}

fn report_state_text(run: Option<&crate::simulation::GuiSimulationRun>) -> &'static str {
    match run.map(|run| &run.report) {
        Some(crate::report_summary::GuiReportSummaryState::Ready(_)) => "Ready",
        Some(crate::report_summary::GuiReportSummaryState::Missing(_)) => "Missing",
        None => "No run",
    }
}
