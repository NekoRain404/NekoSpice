//! Optimization workspace center sections — targets, parametric sweep, and Monte Carlo tabs.
//!
//! All data is editable via text fields. When no targets/params are defined,
//! the panels show "add" buttons and empty-state prompts.

use crate::app::NekoSpiceApp;
use crate::app::localization::UiText;
use super::state::{MonteCarloMeasurement, MonteCarloParam, OptimizationTarget, SweepParam};
use super::widgets::{
    definition_row, measurement_row, metric_card, mini_donut, parameter_row, progress_bar,
    result_row, status_chip, sweep_row,
};
use crate::app::theme::StudioTheme;
use eframe::egui;

impl NekoSpiceApp {
    /// Draw the optimization sub-tab selector.
    pub(crate) fn draw_optimization_tabs(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            for tab in super::state::OptimizationTab::ALL {
                let label = self.text(tab.text_key());
                ui.selectable_value(&mut self.optimization_workspace.active_tab, tab, label);
            }
        });
    }

    /// Dispatch to the correct sub-panel based on active tab.
    pub(crate) fn draw_optimization_main_panel(&mut self, ui: &mut egui::Ui) {
        match self.optimization_workspace.active_tab {
            super::state::OptimizationTab::Targets => self.draw_targets_panel(ui),
            super::state::OptimizationTab::Sweep => self.draw_sweep_panel(ui),
            super::state::OptimizationTab::MonteCarlo => self.draw_monte_carlo_panel(ui),
        }
    }

    // ── Targets panel ────────────────────────────────────────────────

    /// Draw the optimization targets panel with editable rows.
    pub(crate) fn draw_targets_panel(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(mode, self.text(UiText::Objective)));

            let has_targets = !self.optimization_workspace.targets.is_empty();
            if has_targets {
                // Header row
                ui.horizontal(|ui| {
                    ui.label(StudioTheme::muted_for(mode, "Name"));
                    ui.label(StudioTheme::muted_for(mode, "Goal"));
                    ui.label(StudioTheme::muted_for(mode, "Constraint"));
                });
                ui.separator();

                // Editable target rows
                let mut remove_idx = None;
                for (i, target) in self.optimization_workspace.targets.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(&mut target.name);
                        egui::ComboBox::from_id_salt(format!("target_goal_{i}"))
                            .selected_text(&target.goal)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut target.goal, "minimize".to_string(), "minimize");
                                ui.selectable_value(&mut target.goal, "maximize".to_string(), "maximize");
                            });
                        ui.text_edit_singleline(&mut target.constraint);
                        if ui.small_button("X").on_hover_text("Remove target").clicked() {
                            remove_idx = Some(i);
                        }
                    });
                }
                if let Some(idx) = remove_idx {
                    self.optimization_workspace.targets.remove(idx);
                }
            } else {
                ui.label(StudioTheme::muted_for(mode, "No optimization targets defined. Add one below."));
            }

            ui.add_space(4.0);
            if ui.button("+ Add Target").clicked() {
                self.optimization_workspace.targets.push(OptimizationTarget::default());
            }

            ui.add_space(8.0);
            ui.label(StudioTheme::section_title_for(mode, self.text(UiText::CandidateResults)));

            let run_count = self.simulation_panel.last_run.as_ref()
                .map(|_| 1)
                .unwrap_or(0);
            if run_count > 0 {
                result_row(ui, mode, "Best", "from last run", "current");
            } else {
                ui.label(StudioTheme::muted_for(mode, "Run a simulation to see results"));
            }
        });
    }

    // ── Sweep panel ──────────────────────────────────────────────────

    /// Draw the parametric sweep panel with editable sweep parameters.
    pub(crate) fn draw_sweep_panel(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(mode, self.text(UiText::ParametricSweep)));

            // Show current .step directive if configured
            match &self.simulation_panel.step_sweep {
                crate::app::simulation::state::StepSweep::None => {
                    ui.label(StudioTheme::muted_for(mode, "No .step directive configured"));
                }
                sweep => {
                    let text = format!("{:?}", sweep);
                    ui.label(StudioTheme::accent_for(mode, &text));
                }
            }

            ui.add_space(6.0);

            let has_params = !self.optimization_workspace.sweep_params.is_empty();
            if has_params {
                egui::Grid::new("optimization_sweep_grid")
                    .num_columns(5)
                    .spacing(egui::Vec2::new(8.0, 6.0))
                    .striped(true)
                    .show(ui, |ui| {
                        ui.strong(self.text(UiText::Parameter));
                        ui.strong("Start");
                        ui.strong("Stop");
                        ui.strong(self.text(UiText::Samples));
                        ui.strong("");
                        ui.end_row();

                        let mut remove_idx = None;
                        for (i, param) in self.optimization_workspace.sweep_params.iter_mut().enumerate() {
                            ui.text_edit_singleline(&mut param.name);
                            ui.text_edit_singleline(&mut param.start);
                            ui.text_edit_singleline(&mut param.stop);
                            ui.text_edit_singleline(&mut param.count);
                            if ui.small_button("X").on_hover_text("Remove").clicked() {
                                remove_idx = Some(i);
                            }
                            ui.end_row();
                        }
                        if let Some(idx) = remove_idx {
                            self.optimization_workspace.sweep_params.remove(idx);
                        }
                    });
            } else {
                ui.label(StudioTheme::muted_for(mode, "No sweep parameters defined. Add one below."));
            }

            ui.add_space(4.0);
            if ui.button("+ Add Parameter").clicked() {
                self.optimization_workspace.sweep_params.push(SweepParam::default());
            }
        });
    }

    // ── Monte Carlo panel ────────────────────────────────────────────

    /// Draw the Monte Carlo panel with editable parameters and measurements.
    pub(crate) fn draw_monte_carlo_panel(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(mode, self.text(UiText::MonteCarlo)));
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

    /// Draw optimization summary panel with real or empty state.
    pub(crate) fn draw_optimization_summary_panel(&self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let ws = &self.optimization_workspace;
        let total = ws.mc_completed;
        let passed = ws.mc_passed;
        let yield_ratio = if total > 0 { passed as f32 / total as f32 } else { 0.0 };

        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(mode, self.text(UiText::StatisticalSummary)));

            if total > 0 {
                ui.horizontal(|ui| {
                    mini_donut(ui, mode, yield_ratio);
                    ui.vertical(|ui| {
                        let yield_pct = format!("{:.1}%", yield_ratio * 100.0);
                        status_chip(ui, "PASS", StudioTheme::palette(mode).success);
                        parameter_row(ui, mode, self.text(UiText::Yield),
                            &format!("{} / {}", passed, total), &yield_pct);
                        parameter_row(ui, mode, self.text(UiText::Completed),
                            &total.to_string(), &format!("{}%", 100));
                    });
                });
            } else {
                ui.label(StudioTheme::muted_for(mode, "Run Monte Carlo analysis to see results"));
            }
        });
    }

    // ── Monte Carlo metric cards (uses real run state) ───────────────

    fn draw_monte_carlo_metrics(&self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let ws = &self.optimization_workspace;
        let total: usize = ws.mc_sample_count.parse().unwrap_or(0);
        let completed = ws.mc_completed;
        let passed = ws.mc_passed;
        let failed = completed.saturating_sub(passed);
        let status = self.simulation_panel.last_run.as_ref()
            .map(|r| r.metadata.status.as_str().to_string())
            .unwrap_or_else(|| "Queued".to_string());

        let cols = if ui.available_width() >= 720.0 { 4 } else { 2 };
        ui.columns(cols, |columns| {
            metric_card(&mut columns[0], mode, self.text(UiText::Samples),
                &format!("{}", total), if total > 0 { "configured" } else { "set count below" });
            let comp_pct = if completed > 0 { format!("{:.1}%", completed as f32 / total.max(1) as f32 * 100.0) } else { "0%".to_string() };
            metric_card(&mut columns[1], mode, self.text(UiText::Completed),
                &format!("{}", completed), &comp_pct);
            let pf_pct = if completed > 0 { format!("{:.1}%", passed as f32 / completed as f32 * 100.0) } else { "-".to_string() };
            metric_card(&mut columns[2], mode, self.text(UiText::PassFail),
                &format!("{} / {}", passed, failed), &pf_pct);
            metric_card(&mut columns[3], mode, self.text(UiText::AnalysisStatus),
                &status, "ngspice");
        });
    }

    // ── Editable parameter definitions ────────────────────────────────

    fn draw_parameter_definitions(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        ui.label(StudioTheme::section_title_for(mode, self.text(UiText::ParameterDefinitions)));

        // Sample count editor
        ui.horizontal(|ui| {
            ui.label(StudioTheme::muted_for(mode, "Samples:"));
            ui.add(egui::TextEdit::singleline(&mut self.optimization_workspace.mc_sample_count)
                .desired_width(80.0));
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
                                ui.selectable_value(&mut param.distribution, "Gaussian".to_string(), "Gaussian");
                                ui.selectable_value(&mut param.distribution, "Uniform".to_string(), "Uniform");
                                ui.selectable_value(&mut param.distribution, "LogNormal".to_string(), "LogNormal");
                            });
                        ui.text_edit_singleline(&mut param.tolerance);
                        if ui.small_button("X").clicked() { remove_idx = Some(i); }
                        ui.end_row();
                    }
                    if let Some(idx) = remove_idx {
                        self.optimization_workspace.mc_params.remove(idx);
                    }
                });
        } else {
            ui.label(StudioTheme::muted_for(mode, "No parameters defined. Add one below."));
        }

        if ui.button("+ Add Parameter").clicked() {
            self.optimization_workspace.mc_params.push(MonteCarloParam::default());
        }
    }

    // ── Editable response measurements ────────────────────────────────

    fn draw_response_measurements(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        ui.label(StudioTheme::section_title_for(mode, self.text(UiText::ResponseMeasurements)));

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
                    for (i, m) in self.optimization_workspace.mc_measurements.iter_mut().enumerate() {
                        ui.text_edit_singleline(&mut m.name);
                        ui.text_edit_singleline(&mut m.kind);
                        ui.text_edit_singleline(&mut m.spec);
                        if ui.small_button("X").clicked() { remove_idx = Some(i); }
                        ui.end_row();
                    }
                    if let Some(idx) = remove_idx {
                        self.optimization_workspace.mc_measurements.remove(idx);
                    }
                });
        } else {
            ui.label(StudioTheme::muted_for(mode, "No measurements defined. Add one below."));
        }

        if ui.button("+ Add Measurement").clicked() {
            self.optimization_workspace.mc_measurements.push(MonteCarloMeasurement::default());
        }
    }
}
