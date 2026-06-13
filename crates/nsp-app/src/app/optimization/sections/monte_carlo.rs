//! 蒙特卡洛分析面板。配置参数分布、容差和响应测量指标，
//! 支持 Gaussian/Uniform/LogNormal 分布类型选择。

use super::super::state::{MonteCarloMeasurement, MonteCarloParam};
use super::super::widgets::metric_card;
use crate::app::NekoSpiceApp;
use crate::app::localization::UiText;
use crate::app::theme::StudioTheme;
use eframe::egui;

impl NekoSpiceApp {
    /// Draw the Monte Carlo analysis panel.
    pub(crate) fn draw_monte_carlo_panel(&mut self, ui: &mut egui::Ui) {
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
        });
    }

    /// Draw the Monte Carlo summary metrics cards.
    fn draw_monte_carlo_metrics(&self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let ws = &self.optimization_workspace;
        let total: usize = ws.mc_sample_count.parse().unwrap_or(0);
        let completed = ws.mc_completed;
        let passed = ws.mc_passed;
        let failed = completed.saturating_sub(passed);
        let status = self
            .simulation_panel
            .last_run
            .as_ref()
            .map(|r| r.metadata.status.as_str().to_string())
            .unwrap_or_else(|| "Queued".to_string());

        let cols = if ui.available_width() >= 720.0 { 4 } else { 2 };
        ui.columns(cols, |columns| {
            metric_card(
                &mut columns[0],
                mode,
                self.text(UiText::Samples),
                &format!("{}", total),
                if total > 0 {
                    "configured"
                } else {
                    "set count below"
                },
            );
            let comp_pct = if completed > 0 {
                format!("{:.1}%", completed as f32 / total.max(1) as f32 * 100.0)
            } else {
                "0%".to_string()
            };
            metric_card(
                &mut columns[1],
                mode,
                self.text(UiText::Completed),
                &format!("{}", completed),
                &comp_pct,
            );
            let pf_pct = if completed > 0 {
                format!("{:.1}%", passed as f32 / completed as f32 * 100.0)
            } else {
                "-".to_string()
            };
            metric_card(
                &mut columns[2],
                mode,
                self.text(UiText::PassFail),
                &format!("{} / {}", passed, failed),
                &pf_pct,
            );
            metric_card(
                &mut columns[3],
                mode,
                self.text(UiText::AnalysisStatus),
                &status,
                "ngspice",
            );
        });
    }

    /// Draw editable parameter definitions with distribution type and tolerance.
    fn draw_parameter_definitions(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        ui.label(StudioTheme::section_title_for(
            mode,
            self.text(UiText::ParameterDefinitions),
        ));

        ui.horizontal(|ui| {
            ui.label(StudioTheme::muted_for(mode, "Samples:"));
            ui.add(
                egui::TextEdit::singleline(&mut self.optimization_workspace.mc_sample_count)
                    .desired_width(80.0),
            );
        });
        ui.add_space(4.0);

        let has_params = !self.optimization_workspace.mc_params.is_empty();
        if has_params {
            egui::Grid::new("monte_carlo_parameter_definitions")
                .num_columns(5)
                .spacing(egui::vec2(8.0, 6.0))
                .striped(true)
                .show(ui, |ui| {
                    ui.strong(self.text(UiText::Parameter));
                    ui.strong("Nominal");
                    ui.strong(self.text(UiText::Distributions));
                    ui.strong(self.text(UiText::Tolerance));
                    ui.strong("");
                    ui.end_row();

                    let mut remove_idx = None;
                    for (i, param) in self.optimization_workspace.mc_params.iter_mut().enumerate() {
                        ui.text_edit_singleline(&mut param.name);
                        ui.text_edit_singleline(&mut param.nominal);
                        egui::ComboBox::from_id_salt(format!("mc_dist_{i}"))
                            .selected_text(&param.distribution)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut param.distribution,
                                    "Gaussian".to_string(),
                                    "Gaussian",
                                );
                                ui.selectable_value(
                                    &mut param.distribution,
                                    "Uniform".to_string(),
                                    "Uniform",
                                );
                                ui.selectable_value(
                                    &mut param.distribution,
                                    "LogNormal".to_string(),
                                    "LogNormal",
                                );
                            });
                        ui.text_edit_singleline(&mut param.tolerance);
                        if ui.small_button("X").clicked() {
                            remove_idx = Some(i);
                        }
                        ui.end_row();
                    }
                    if let Some(idx) = remove_idx {
                        self.optimization_workspace.mc_params.remove(idx);
                    }
                });
        } else {
            ui.label(StudioTheme::muted_for(
                mode,
                "No parameters defined. Add one below.",
            ));
        }

        if ui.button("+ Add Parameter").clicked() {
            self.optimization_workspace
                .mc_params
                .push(MonteCarloParam::default());
        }
    }

    /// Draw editable response measurements with measurement type and spec.
    fn draw_response_measurements(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        ui.label(StudioTheme::section_title_for(
            mode,
            self.text(UiText::ResponseMeasurements),
        ));

        let has_measurements = !self.optimization_workspace.mc_measurements.is_empty();
        if has_measurements {
            egui::Grid::new("monte_carlo_response_measurements")
                .num_columns(5)
                .spacing(egui::vec2(8.0, 6.0))
                .striped(true)
                .show(ui, |ui| {
                    ui.strong("Measurement");
                    ui.strong("Type");
                    ui.strong("Spec");
                    ui.strong("");
                    ui.end_row();

                    let mut remove_idx = None;
                    for (i, m) in self
                        .optimization_workspace
                        .mc_measurements
                        .iter_mut()
                        .enumerate()
                    {
                        ui.text_edit_singleline(&mut m.name);
                        ui.text_edit_singleline(&mut m.kind);
                        ui.text_edit_singleline(&mut m.spec);
                        if ui.small_button("X").clicked() {
                            remove_idx = Some(i);
                        }
                        ui.end_row();
                    }
                    if let Some(idx) = remove_idx {
                        self.optimization_workspace.mc_measurements.remove(idx);
                    }
                });
        } else {
            ui.label(StudioTheme::muted_for(
                mode,
                "No measurements defined. Add one below.",
            ));
        }

        if ui.button("+ Add Measurement").clicked() {
            self.optimization_workspace
                .mc_measurements
                .push(MonteCarloMeasurement::default());
        }
    }
}
