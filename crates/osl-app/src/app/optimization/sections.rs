use crate::app::NekoSpiceApp;
use crate::app::localization::UiText;
use super::state::OptimizationTab;
use super::widgets::{
    definition_row, measurement_row, metric_card, mini_donut, parameter_row, progress_bar,
    result_row, status_chip, sweep_row,
};
use crate::app::theme::StudioTheme;
use eframe::egui;

impl NekoSpiceApp {
    /// draw optimization tabs。
    pub(crate) fn draw_optimization_tabs(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            for tab in OptimizationTab::ALL {
                let label = self.text(tab.text_key());
                ui.selectable_value(&mut self.optimization_workspace.active_tab, tab, label);
            }
        });
    }

    /// draw optimization main panel。
    pub(crate) fn draw_optimization_main_panel(&self, ui: &mut egui::Ui) {
        match self.optimization_workspace.active_tab {
            OptimizationTab::Targets => self.draw_targets_panel(ui),
            OptimizationTab::Sweep => self.draw_sweep_panel(ui),
            OptimizationTab::MonteCarlo => self.draw_monte_carlo_panel(ui),
        }
    }

    /// draw targets panel。
    pub(crate) fn draw_targets_panel(&self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::Objective),
            ));
            parameter_row(ui, mode, "Vout ripple", "minimize", "< 80 mV");
            parameter_row(ui, mode, "Efficiency", "maximize", "> 90%");
            parameter_row(ui, mode, "I(L1) RMS", "minimize", "< 0.8 A");
            ui.separator();
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::CandidateResults),
            ));
            result_row(ui, mode, "A", "R1=1.0k, C1=1u", "+12.4%");
            result_row(ui, mode, "B", "R1=1.2k, C1=820n", "+9.1%");
            result_row(ui, mode, "C", "R1=820, C1=1.5u", "+6.8%");
        });
    }

    /// draw sweep panel。
    pub(crate) fn draw_sweep_panel(&self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::ParametricSweep),
            ));
            egui::Grid::new("optimization_sweep_grid")
                .num_columns(4)
                .spacing(egui::Vec2::new(14.0, 6.0))
                .striped(true)
                .show(ui, |ui| {
                    ui.strong(self.text(UiText::Parameter));
                    ui.strong(self.text(UiText::Range));
                    ui.strong(self.text(UiText::Samples));
                    ui.strong(self.text(UiText::StatusConsole));
                    ui.end_row();
                    sweep_row(ui, "R1", "680 .. 1.5k", "16", "queued");
                    sweep_row(ui, "C1", "470n .. 2.2u", "12", "queued");
                    sweep_row(ui, "TEMP", "-20 .. 85 C", "8", "ready");
                });
        });
    }

    /// draw monte carlo panel。
    pub(crate) fn draw_monte_carlo_panel(&self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::MonteCarlo),
            ));
            self.draw_monte_carlo_metrics(ui);
            ui.add_space(8.0);
            if ui.available_width() >= 720.0 {
                ui.columns(2, |columns| {
                    self.draw_parameter_definitions(&mut columns[0]);
                    self.draw_response_measurements(&mut columns[1]);
                });
            } else {
                self.draw_parameter_definitions(ui);
                ui.add_space(8.0);
                self.draw_response_measurements(ui);
            }
            ui.separator();
            self.draw_distribution_preview(ui);
        });
    }

    /// draw optimization summary panel。
    pub(crate) fn draw_optimization_summary_panel(&self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::StatisticalSummary),
            ));
            ui.horizontal(|ui| {
                mini_donut(ui, mode, 0.933);
                ui.vertical(|ui| {
                    status_chip(ui, "PASS", StudioTheme::palette(mode).success);
                    parameter_row(ui, mode, self.text(UiText::Yield), "933 / 1,000", "93.3%");
                    parameter_row(ui, mode, "Worst", "Signal Level", "3.42 sigma");
                    parameter_row(ui, mode, "Confidence", "95%", "+/-1.09%");
                });
            });
            ui.separator();
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::JobMonitor),
            ));
            progress_bar(ui, mode, "MC_1000_Run3", 0.842);
            progress_bar(ui, mode, "Param_Corner", 1.0);
            progress_bar(ui, mode, "Sweep_Vin", 0.0);
        });
    }

    fn draw_monte_carlo_metrics(&self, ui: &mut egui::Ui) {
        if ui.available_width() >= 720.0 {
            ui.columns(4, |columns| {
                self.draw_monte_carlo_metric_cards(&mut columns[0], 0);
                self.draw_monte_carlo_metric_cards(&mut columns[1], 1);
                self.draw_monte_carlo_metric_cards(&mut columns[2], 2);
                self.draw_monte_carlo_metric_cards(&mut columns[3], 3);
            });
        } else {
            ui.columns(2, |columns| {
                self.draw_monte_carlo_metric_cards(&mut columns[0], 0);
                self.draw_monte_carlo_metric_cards(&mut columns[1], 1);
            });
            ui.add_space(6.0);
            ui.columns(2, |columns| {
                self.draw_monte_carlo_metric_cards(&mut columns[0], 2);
                self.draw_monte_carlo_metric_cards(&mut columns[1], 3);
            });
        }
        ui.add_space(2.0);
    }

    fn draw_monte_carlo_metric_cards(&self, ui: &mut egui::Ui, index: usize) {
        let mode = self.theme_mode();
        match index {
            0 => metric_card(ui, mode, self.text(UiText::Samples), "1,000", "planned"),
            1 => metric_card(ui, mode, self.text(UiText::Completed), "842", "84.2%"),
            2 => metric_card(ui, mode, self.text(UiText::PassFail), "933 / 67", "93.3%"),
            _ => metric_card(
                ui,
                mode,
                self.text(UiText::AnalysisStatus),
                &self.simulation_panel.last_run.as_ref()
                    .map(|r| r.metadata.status.as_str().to_string())
                    .unwrap_or_else(|| "Queued".to_string()),
                "ngspice",
            ),
        }
    }

    fn draw_parameter_definitions(&self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        ui.label(StudioTheme::section_title_for(
            mode,
            self.text(UiText::ParameterDefinitions),
        ));
        egui::Grid::new("monte_carlo_parameter_definitions")
            .num_columns(5)
            .spacing(egui::vec2(12.0, 6.0))
            .striped(true)
            .show(ui, |ui| {
                ui.strong(self.text(UiText::Parameter));
                ui.strong("Nominal");
                ui.strong(self.text(UiText::Distributions));
                ui.strong(self.text(UiText::Tolerance));
                ui.strong("Sens.");
                ui.end_row();
                definition_row(ui, mode, "R1", "10k", "Gaussian", "1%", "High");
                definition_row(ui, mode, "R2", "100k", "Gaussian", "1%", "High");
                definition_row(ui, mode, "C1", "10n", "Gaussian", "5%", "Med");
                definition_row(ui, mode, "VOS", "1m", "Gaussian", "0.5m", "High");
            });
    }

    fn draw_response_measurements(&self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        ui.label(StudioTheme::section_title_for(
            mode,
            self.text(UiText::ResponseMeasurements),
        ));
        egui::Grid::new("monte_carlo_response_measurements")
            .num_columns(4)
            .spacing(egui::vec2(12.0, 6.0))
            .striped(true)
            .show(ui, |ui| {
                ui.strong("Measurement");
                ui.strong("Type");
                ui.strong("Spec");
                ui.strong("Goal");
                ui.end_row();
                measurement_row(ui, "V(out)_DC", "DC", "1.200 V +/- 2%", "up");
                measurement_row(ui, "UGF", "AC", "> 5.0 MHz", "up");
                measurement_row(ui, "Phase Margin", "AC", "> 60 deg", "up");
            });
    }

    fn draw_distribution_preview(&self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        ui.label(StudioTheme::section_title_for(
            mode,
            self.text(UiText::Distributions),
        ));
        ui.columns(3, |columns| {
            metric_card(
                &mut columns[0],
                mode,
                "V(out)_DC",
                "1.2001 V",
                "yield 96.1%",
            );
            metric_card(&mut columns[1], mode, "UGF", "10.21 MHz", "yield 94.7%");
            metric_card(
                &mut columns[2],
                mode,
                "Phase Margin",
                "67.8 deg",
                "yield 95.0%",
            );
        });
    }
}
