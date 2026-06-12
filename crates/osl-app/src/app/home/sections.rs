//! Home workspace dashboard sections.
//!
//! Contains the project list, solver health, recent measurements, and
//! recommendations panels. Template and quick-action grids have been
//! split into `templates.rs` and `quick_actions.rs`.

use crate::app::NekoSpiceApp;
use super::widgets::{measurement_row, project_row, queue_row, recommendation_row, section_header, section_header_clickable};
use crate::app::localization::UiText;
use crate::app::navigation::StudioWorkspace;
use crate::app::theme::StudioTheme;
use crate::app::widgets::metric_row;
use eframe::egui;
use osl_core::RunStatus;

impl NekoSpiceApp {
    /// Draw the recent projects panel showing loaded schematic and library.
    pub(crate) fn draw_recent_projects_panel(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            if section_header_clickable(ui, mode, self.text(UiText::RecentProjects), self.text(UiText::ViewAll)) {
                self.active_workspace = StudioWorkspace::Schematic;
            }
            // Currently loaded schematic
            let snapshot = self.studio_status_snapshot();
            project_row(ui, mode, &snapshot.project_name, &snapshot.source_path, &snapshot.document_state);
            // Symbol library
            if !self.library_table_path.is_empty() {
                let lib_name = std::path::Path::new(&self.library_table_path)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| self.library_table_path.clone());
                let status = if self.library.is_some() { self.text(UiText::Ready) } else { self.text(UiText::Missing) };
                project_row(ui, mode, &lib_name, &self.library_table_path, status);
            }
        });
    }

    /// Draw the simulation queue panel showing running/completed jobs.
    pub(crate) fn draw_simulation_queue_panel(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            section_header(ui, mode, self.text(UiText::SimulationQueue), self.text(UiText::ViewAll));
            if self.simulation_panel.active_task.is_some() {
                queue_row(ui, mode, "1", self.simulation_panel.backend.label(), &self.schematic_path, self.text(UiText::Running));
            } else if let Some(run) = &self.simulation_panel.last_run {
                let status = match run.metadata.status {
                    RunStatus::Passed => self.text(UiText::Completed),
                    RunStatus::Failed => self.text(UiText::WaveformError),
                };
                queue_row(ui, mode, "1", self.simulation_panel.backend.label(), &self.schematic_path, status);
            } else {
                ui.label(StudioTheme::muted_for(mode, self.text(UiText::NoRecentRun)));
            }
        });
    }

    /// Draw the solver health / system diagnostics panel.
    pub(crate) fn draw_solver_health_panel(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            section_header(ui, mode, self.text(UiText::SolverHealth), self.text(UiText::Diagnostics));
            let threads = std::thread::available_parallelism().map_or(1, |n| n.get());
            if self.simulation_panel.active_task.is_some() {
                ui.horizontal(|ui| {
                    ui.colored_label(self.theme_palette().accent, "●");
                    ui.label(self.text(UiText::Running));
                });
            } else {
                metric_row(ui, mode, self.text(UiText::HealthReady), "");
            }
            metric_row(ui, mode, self.text(UiText::Threads), &format!("{threads} threads"));
            metric_row(ui, mode, self.text(UiText::Renderer), "wgpu");
            metric_row(ui, mode, self.text(UiText::Backend), "CLI");
            if self.simulation_panel.active_task.is_none() {
                ui.add_space(4.0);
                metric_row(ui, mode, self.text(UiText::SystemOperational), "");
            }
        });
    }

    /// Draw recent measurements from the last simulation run.
    /// Shows real waveform variable summaries when available, otherwise
    /// displays analysis parameters and run metadata.
    pub(crate) fn draw_recent_measurements_panel(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            section_header(ui, mode, self.text(UiText::RecentMeasurements), self.text(UiText::ViewAll));
            if let Some(run) = &self.simulation_panel.last_run {
                let status = match run.metadata.status {
                    RunStatus::Passed => self.text(UiText::Saved),
                    RunStatus::Failed => self.text(UiText::WaveformError),
                };
                metric_row(ui, mode, "Status", status);
                metric_row(ui, mode, "Duration", &format!("{} ms", run.metadata.duration_ms));
                metric_row(ui, mode, "Analysis", &self.simulation_panel.directive_kind.to_string());
                ui.add_space(4.0);
                ui.separator();
                ui.add_space(4.0);
                // Show real waveform variable summaries if available
                if let crate::waveform_summary::GuiWaveformSummaryState::Ready(summary) = &run.waveform {
                    for variable in summary.variables.iter().take(4) {
                        let value_str = if !variable.unit.is_empty() {
                            format!("{} {}", super::super::waveform::preview_primitives::format_compact_f64(variable.last), variable.unit)
                        } else {
                            super::super::waveform::preview_primitives::format_compact_f64(variable.last)
                        };
                        let hint = format!("avg={}, rms={}, pp={}",
                            super::super::waveform::preview_primitives::format_compact_f64(variable.avg),
                            super::super::waveform::preview_primitives::format_compact_f64(variable.rms),
                            super::super::waveform::preview_primitives::format_compact_f64(variable.peak_to_peak),
                        );
                        measurement_row(ui, mode, &variable.name, &value_str, &hint);
                    }
                    if summary.variables.len() > 4 {
                        ui.label(StudioTheme::muted_for(mode,
                            format!("+{} more signals", summary.variables.len() - 4)));
                    }
                } else {
                    // No waveform data — show analysis params as summary
                    let body = self.simulation_panel.analysis_params.to_body();
                    if !body.trim().is_empty() {
                        metric_row(ui, mode, "Parameters", body.trim());
                    }
                }
            } else {
                ui.label(StudioTheme::muted_for(mode, self.text(UiText::NoRecentRun)));
            }
        });
    }

    /// Draw recommendations panel with actionable suggestions.
    /// Each recommendation navigates to the relevant workspace when clicked.
    pub(crate) fn draw_recommendations_panel(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            section_header(ui, mode, self.text(UiText::RecommendedForYou), self.text(UiText::ViewAll));
            if self.document.is_none() {
                if recommendation_row(ui, mode, "Open a schematic to get started", self.text(UiText::Open)) {
                    self.active_workspace = StudioWorkspace::Schematic;
                }
            } else {
                // Check if simulation has been run
                let has_run = self.simulation_panel.last_run.is_some();
                let has_warnings = !self.simulation_panel.netlist_warnings.is_empty();
                let has_error = self.simulation_panel.last_error.is_some();
                let running = self.simulation_panel.active_task.is_some();

                if !has_run && !running {
                    if recommendation_row(ui, mode, "Run a simulation to validate your design", self.text(UiText::Run)) {
                        self.active_workspace = StudioWorkspace::Simulation;
                    }
                } else if has_error {
                    if recommendation_row(ui, mode, "Fix simulation errors — check log output", self.text(UiText::ViewAll)) {
                        self.active_workspace = StudioWorkspace::Simulation;
                    }
                } else if has_warnings {
                    if recommendation_row(ui, mode, &format!("Review {} simulation warnings", self.simulation_panel.netlist_warnings.len()), self.text(UiText::ViewAll)) {
                        self.active_workspace = StudioWorkspace::Simulation;
                    }
                } else if has_run {
                    if recommendation_row(ui, mode, "Analyze waveform results", self.text(UiText::ViewAll)) {
                        self.active_workspace = StudioWorkspace::Waveforms;
                    }
                }

                // ERC errors from schematic
                let report = self.document.as_ref().map(|d| d.check_report());
                if let Some(r) = report {
                    if r.error_count() > 0 {
                        if recommendation_row(ui, mode, &format!("Fix {} ERC errors in schematic", r.error_count()), self.text(UiText::ViewAll)) {
                            self.active_workspace = StudioWorkspace::Schematic;
                        }
                    }
                }

                // Temperature sweep suggestion
                if recommendation_row(ui, mode, self.text(UiText::TemperatureSweep), self.text(UiText::Run)) {
                    self.active_workspace = StudioWorkspace::Simulation;
                    self.simulation_panel.directive_kind = osl_kicad::KicadSimulationDirectiveKind::Tran;
                    self.simulation_panel.step_sweep = super::super::simulation::state::StepSweep::Parametric {
                        param_name: "TEMP".to_string(),
                        sweep_mode: "lin".to_string(),
                        start: "-40".to_string(),
                        stop: "125".to_string(),
                        step: "5".to_string(),
                    };
                }
            }
        });
    }
}
