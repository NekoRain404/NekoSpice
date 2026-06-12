//! Simulation run controller — builds profiles from UI state, launches
//! backend tasks, and polls for completion. Separated from the drawing
//! code to keep the control flow easy to follow.

use crate::app::NekoSpiceApp;
use crate::simulation::{GuiSimulationJob, GuiSimulationTask};
use osl_sim::{SimulationProfile, SpiceMethod, ProfileParamEntry};
use osl_kicad::KicadSimulationDirectiveKind;
use std::path::Path;
use std::time::Duration;
use eframe::egui;

impl NekoSpiceApp {
    /// Build a `SimulationProfile` from the current UI state.
    ///
    /// Reads the analysis kind/body from the directive editor, solver
    /// options from the profile editor, and component/model parameter
    /// overrides — everything the user configured in the simulation panels.
    pub(crate) fn build_simulation_profile(&self) -> SimulationProfile {
        let options = &self.simulation_profile_editor.options;
        let analysis_kind = match self.simulation_panel.directive_kind {
            KicadSimulationDirectiveKind::Tran => ".tran".to_string(),
            KicadSimulationDirectiveKind::Ac => ".ac".to_string(),
            KicadSimulationDirectiveKind::Dc => ".dc".to_string(),
            KicadSimulationDirectiveKind::Op => ".op".to_string(),
            _ => ".tran".to_string(),
        };
        let analysis_body = self.simulation_panel.directive_body.clone();
        SimulationProfile {
            analysis_kind,
            analysis_body,
            temperature: options.temperature.clone(),
            max_iterations: options.max_iterations.clone(),
            min_timestep: options.min_timestep.clone(),
            method: SpiceMethod::from_str_loose(&options.method),
            reltol: options.reltol.clone(),
            abstol: options.abstol.clone(),
            vntol: options.vntol.clone(),
            component_params: self
                .simulation_profile_editor
                .component_params
                .iter()
                .filter(|(name, value, _)| !name.trim().is_empty() && !value.trim().is_empty())
                .map(|(name, value, unit)| ProfileParamEntry {
                    name: name.clone(),
                    value: value.clone(),
                    unit: unit.clone(),
                })
                .collect(),
            model_params: self
                .simulation_profile_editor
                .model_params
                .iter()
                .filter(|(name, value, _)| !name.trim().is_empty() && !value.trim().is_empty())
                .map(|(name, value, unit)| ProfileParamEntry {
                    name: name.clone(),
                    value: value.clone(),
                    unit: unit.clone(),
                })
                .collect(),
        }
    }

    /// Launch a simulation from the current panel state.
    ///
    /// Builds a profile from the editor, validates the netlist, and
    /// spawns the selected backend (ngspice or Xyce) in a background thread.
    /// Results are polled via `poll_simulation_task()`.
    pub(crate) fn run_simulation_from_panel(&mut self) {
        if self.document.is_none() {
            self.status_message = Some("No editable schematic loaded".to_string());
            return;
        };
        // Auto-save the current directive from UI to the schematic
        // so the user doesn't have to click "Set Directive" before Run
        self.apply_simulation_directive_edit();
        // Re-borrow self.document after the mutable borrow above is released
        let Some(document) = &self.document else {
            self.status_message = Some("No editable schematic loaded".to_string());
            return;
        };
        let profile = self.build_simulation_profile();
        let runs_root = Path::new("runs").join("gui");
        match GuiSimulationJob::from_document(document, &runs_root, &profile) {
            Ok(job) => {
                let issues = job.validate();
                if !issues.is_empty() {
                    self.status_message =
                        Some(format!("Netlist issues: {}", issues.join("; ")));
                }
                self.simulation_panel.last_run = None;
                self.simulation_panel.last_error = None;
                let ngspice = self.preferences.ngspice_path.clone();
                    let xyce = self.preferences.xyce_path.clone();
                    self.simulation_panel.active_task =
                        Some(GuiSimulationTask::spawn_with_backend(
                            job,
                            self.simulation_panel.backend.label(),
                            &ngspice,
                            &xyce,
                        ));
                self.status_message =
                    Some(format!("Simulation started ({})", self.simulation_panel.backend.label()));
            }
            Err(error) => {
                self.status_message = Some(error.clone());
                self.simulation_panel.last_error = Some(error);
            }
        }
    }

    /// Poll the active simulation task for completion and update state.
    /// On success: stores the run result and syncs waveform signals.
    /// On failure: reads the parsed error log and displays it.
    pub(in crate::app) fn poll_simulation_task(&mut self) {
        let Some(task) = &self.simulation_panel.active_task else {
            return;
        };
        let Some(result) = task.try_finish() else {
            return;
        };
        self.simulation_panel.active_task = None;
        match result {
            Ok(run) => {
                // If simulation failed, read the parsed error log
                if run.metadata.status == osl_core::RunStatus::Failed {
                    let log_path = run.output_dir.join("ngspice.log");
                    let fallback = run.output_dir.join("xyce.log");
                    let actual = if log_path.is_file() { log_path } else { fallback };
                    if actual.is_file() {
                        if let Ok(text) = std::fs::read_to_string(&actual) {
                            self.status_message = Some(text.clone());
                            self.simulation_panel.last_error = Some(text);
                        } else {
                            self.status_message = Some(format!(
                                "Simulation {} in {} ms (exit {:?})",
                                run.metadata.status.as_str(),
                                run.metadata.duration_ms,
                                run.metadata.exit_code,
                            ));
                            self.simulation_panel.last_error = None;
                        }
                    } else {
                        self.status_message = Some(format!(
                            "Simulation {} in {} ms",
                            run.metadata.status.as_str(),
                            run.metadata.duration_ms
                        ));
                        self.simulation_panel.last_error = None;
                    }
                } else {
                    self.status_message = Some(format!(
                        "Simulation {} in {} ms",
                        run.metadata.status.as_str(),
                        run.metadata.duration_ms
                    ));
                    self.simulation_panel.last_error = None;
                }
                self.sync_selected_waveform_signal(&run.waveform);
                self.simulation_panel.last_run = Some(run);
            }
            Err(error) => {
                self.status_message = Some(error.clone());
                self.simulation_panel.last_run = None;
                self.simulation_panel.last_error = Some(error);
            }
        }
    }

    /// Keep the UI repainting while simulation is in progress.
    pub(in crate::app) fn request_simulation_repaint(&self, ctx: &egui::Context) {
        if self.simulation_panel.active_task.is_some() {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }
}
