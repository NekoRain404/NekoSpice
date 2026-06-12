//! Simulation right panel — manages simulation directives, run execution,
//! and displays results (netlist preview, run output, diagnostics, waveforms).
//!
//! This panel is used in the right sidebar of the schematic workspace and
//! provides the primary interface for running ngspice simulations.

use crate::app::NekoSpiceApp;
use crate::app::localization::UiText;
use super::artifacts_panel::draw_simulation_artifacts_panel;
use super::report_panel::draw_simulation_report_panel;
use super::waveform_panel::draw_simulation_waveform_panel;
use crate::app::status_strip::severity_color;
use crate::app::theme::StudioTheme;
use crate::simulation::{GuiSimulationRun, GuiSimulationTask};
use crate::waveform_summary::GuiWaveformSummaryState;
use eframe::egui;
use osl_core::RunStatus;
use osl_kicad::{KicadDiagnosticSeverity, KicadSimulationDirective, KicadSimulationDirectiveKind};
use osl_sim::{SimulationProfile, SpiceMethod, ProfileParamEntry};
use std::path::Path;
use std::time::Duration;

/// Available simulation backend engines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// 仿真后端类型：`Ngspice` 或 `Xyce`。
pub(crate) enum SimulationBackendKind {
    Ngspice,
    Xyce,
}

impl SimulationBackendKind {
    pub(crate) const ALL: [Self; 2] = [Self::Ngspice, Self::Xyce];

    /// label。
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Ngspice => "ngspice",
            Self::Xyce => "Xyce",
        }
    }

    /// label zh。
    pub(crate) fn label_zh(self) -> &'static str {
        match self {
            Self::Ngspice => "ngspice",
            Self::Xyce => "Xyce",
        }
    }
}

/// Number of netlist lines to display in the preview panel.
const NETLIST_PREVIEW_LINES: usize = 18;

/// Persistent state for the simulation right panel.
///
/// Tracks the current directive kind/body, whether to show the netlist preview,
/// the last completed run, any error, the currently running task, and the
/// selected waveform signal for display.
#[derive(Debug)]
/// 仿真面板状态。
///
/// 持有当前仿真指令、运行结果、错误信息和后端选择。
/// 管理 ngspice/Xyce 仿真的完整生命周期。
pub(crate) struct SimulationPanelState {
    /// Currently selected analysis directive kind (.tran, .ac, .dc, .op).
    pub(crate) directive_kind: KicadSimulationDirectiveKind,
    /// Directive body text (e.g., "1u 1m" for .tran).
    pub(crate) directive_body: String,
    /// Whether to show the netlist preview section.
    pub(crate) show_netlist: bool,
    /// Last completed simulation run result.
    pub(crate) last_run: Option<GuiSimulationRun>,
    /// Error message from the last failed run.
    pub(crate) last_error: Option<String>,
    /// Currently running simulation task (if any).
    pub(crate) active_task: Option<GuiSimulationTask>,
    /// Currently selected waveform signal for display in previews.
    pub(crate) selected_waveform_signal: Option<String>,
    /// Currently selected simulation backend engine.
    pub(crate) backend: SimulationBackendKind,
}

impl Default for SimulationPanelState {
    fn default() -> Self {
        Self {
            directive_kind: KicadSimulationDirectiveKind::Tran,
            directive_body: "1u 1m".to_string(),
            show_netlist: true,
            last_run: None,
            last_error: None,
            active_task: None,
            selected_waveform_signal: None,
            backend: SimulationBackendKind::Ngspice,
        }
    }
}

impl NekoSpiceApp {
    /// Draw the full simulation right panel: directive editor, run button,
    /// netlist preview, diagnostics, run results, and waveform summary.
    pub(crate) fn draw_simulation_panel(&mut self, ui: &mut egui::Ui) {
        self.poll_simulation_task();
        ui.heading(self.text(UiText::SimulationWorkspace));
        self.draw_simulation_directive_editor(ui);

        let is_running = self.simulation_panel.active_task.is_some();
        if is_running {
            // Keep the UI repainting while simulation is in progress
            ui.ctx().request_repaint_after(Duration::from_millis(100));
        }
ui.horizontal(|ui| {
            // Backend engine selector
            egui::ComboBox::from_id_salt("simulation_backend")
                .selected_text(self.simulation_panel.backend.label())
                .show_ui(ui, |ui| {
                    for &kind in &SimulationBackendKind::ALL {
                        let label = match self.locale() {
                            crate::app::localization::StudioLocale::SimplifiedChinese => kind.label_zh(),
                            _ => kind.label(),
                        };
                        if ui.selectable_value(
                            &mut self.simulation_panel.backend,
                            kind,
                            label,
                        ).clicked() {
                            // Backend changed
                        }
                    }
                });
            ui.separator();
            if ui
                .add_enabled(
                    self.document.is_some() && !is_running,
                    egui::Button::new(self.text(UiText::RunSimulation)),
                )
                .clicked()
            {
                self.run_simulation_from_panel();
            }
            if is_running {
                ui.label(self.text(UiText::Running));
            }
        });
        self.draw_simulation_run_status(ui);

        let Some(document) = &self.document else {
            ui.label(self.text(UiText::NoEditableSchematicLoaded));
            return;
        };

        ui.separator();
        draw_simulation_directives(ui, &document.simulation_directives());

        ui.separator();
        self.draw_document_diagnostics_panel(ui, 150.0);

        ui.separator();
        ui.checkbox(&mut self.simulation_panel.show_netlist, "Netlist preview");
        if self.simulation_panel.show_netlist {
            let profile = self.build_simulation_profile();
            let preview = document.spice_netlist_preview().map(|netlist| {
                osl_sim::inject_profile_directives(&netlist, &profile)
            });
            match preview {
                Ok(netlist) => {
                    egui::ScrollArea::vertical()
                        .id_salt("simulation_netlist_preview")
                        .max_height(220.0)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            for line in netlist.lines() {
                                ui.monospace(line);
                            }
                        });
                }
                Err(error) => {
                    ui.colored_label(
                        severity_color(self.theme_mode(), KicadDiagnosticSeverity::Error),
                        error,
                    );
                }
            }
        }
    }

    /// Directive editor: analysis type selector (.tran/.ac/.dc/.op) and body text input.
    pub(in crate::app) fn draw_simulation_directive_editor(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            for (kind, label) in [
                (KicadSimulationDirectiveKind::Tran, ".tran"),
                (KicadSimulationDirectiveKind::Ac, ".ac"),
                (KicadSimulationDirectiveKind::Dc, ".dc"),
                (KicadSimulationDirectiveKind::Op, ".op"),
            ] {
                ui.selectable_value(&mut self.simulation_panel.directive_kind, kind, label);
            }
        });
        ui.horizontal(|ui| {
            ui.label(self.text(UiText::Body));
            ui.text_edit_singleline(&mut self.simulation_panel.directive_body);
        });
        if ui
            .add_enabled(
                self.document.is_some(),
                egui::Button::new(self.text(UiText::SetDirective)),
            )
            .clicked()
        {
            self.apply_simulation_directive_edit();
        }
    }

    /// Apply the current directive editor state to the loaded document.
    pub(in crate::app) fn apply_simulation_directive_edit(&mut self) {
        let Some(document) = &mut self.document else {
            self.status_message = Some("No editable schematic loaded".to_string());
            return;
        };
        // Snapshot before edit for undo support
        self.history.push(document.snapshot());
        let kind = self.simulation_panel.directive_kind;
        let body = self.simulation_panel.directive_body.clone();
        match document.set_simulation_directive(kind, body, None) {
            Ok(summary) => {
                self.scene = Some(document.scene());
                self.load_error = None;
                self.history.clear_redo();
                self.status_message =
                    Some(format!("Edited {} {}", summary.operation, summary.target));
            }
            Err(error) => {
                self.status_message = Some(error);
            }
        }
    }

    /// Build a SimulationProfile from the current profile editor state.
    ///
    /// Converts the GUI editor's editable fields (temperature, tolerances,
    /// method, component/model params) into a SimulationProfile that will
    /// be injected as SPICE directives in the netlist.
    pub(crate) fn build_simulation_profile(&self) -> SimulationProfile {
        let options = &self.simulation_profile_editor.options;
        // Get analysis kind and body from the UI directive editor
        let analysis_kind = match self.simulation_panel.directive_kind {
            osl_kicad::KicadSimulationDirectiveKind::Tran => ".tran".to_string(),
            osl_kicad::KicadSimulationDirectiveKind::Ac => ".ac".to_string(),
            osl_kicad::KicadSimulationDirectiveKind::Dc => ".dc".to_string(),
            osl_kicad::KicadSimulationDirectiveKind::Op => ".op".to_string(),
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
    /// Builds a simulation profile from the editor state, validates the
    /// netlist, and spawns the backend (ngspice or Xyce) in a background
    /// thread. Results are polled via `poll_simulation_task()`.
    pub(crate) fn run_simulation_from_panel(&mut self) {
        let Some(document) = &self.document else {
            self.status_message = Some("No editable schematic loaded".to_string());
            return;
        };
        // Build profile from editor state
        let profile = self.build_simulation_profile();
        let runs_root = Path::new("runs").join("gui");
        match crate::simulation::GuiSimulationJob::from_document(document, &runs_root, &profile) {
            Ok(job) => {
                // Validate netlist before running
                let issues = job.validate();
                if !issues.is_empty() {
                    self.status_message = Some(format!("Netlist issues: {}", issues.join("; ")));
                }
                self.simulation_panel.last_run = None;
                self.simulation_panel.last_error = None;
                self.simulation_panel.active_task = Some(GuiSimulationTask::spawn_with_backend(job, self.simulation_panel.backend.label()));
                self.status_message = Some(format!("Simulation started ({})", self.simulation_panel.backend.label()));
            }
            Err(error) => {
                self.status_message = Some(error.clone());
                self.simulation_panel.last_error = Some(error);
            }
        }
    }

    /// Poll the active simulation task for completion and update state.
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
                // If simulation failed, try to read the parsed error log
                if run.metadata.status == osl_core::RunStatus::Failed {
                    let error_file = run.output_dir.join("simulation-error.txt");
                    if let Ok(error_text) = std::fs::read_to_string(&error_file) {
                        self.status_message = Some(error_text.clone());
                        self.simulation_panel.last_error = Some(error_text);
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

    /// Draw the run status section: current task state, errors, and last run info.
    pub(in crate::app) fn draw_simulation_run_status(&mut self, ui: &mut egui::Ui) {
        if self.simulation_panel.active_task.is_some() {
            ui.label(self.text(UiText::Running));
        }
        // Show ngspice/stderr log if available
        if let Some(run) = &self.simulation_panel.last_run {
            let log_path = run.output_dir.join("ngspice.log");
            let fallback_log = run.output_dir.join("xyce.log");
            let actual_log = if log_path.is_file() { log_path } else { fallback_log };
            if actual_log.is_file() {
                if let Ok(log_content) = std::fs::read_to_string(&actual_log) {
                    ui.separator();
                    ui.label(StudioTheme::muted_for(self.theme_mode(), "Simulation Log"));
                    egui::ScrollArea::vertical()
                        .id_salt("ngspice_log_viewer")
                        .max_height(100.0)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.monospace(&log_content);
                        });
                }
            }
        }
        if let Some(error) = &self.simulation_panel.last_error {
            ui.colored_label(
                severity_color(self.theme_mode(), KicadDiagnosticSeverity::Error),
                error,
            );
        }
        if let Some(run) = &self.simulation_panel.last_run {
            let color = match run.metadata.status {
                RunStatus::Passed => self.theme_palette().success,
                RunStatus::Failed => {
                    severity_color(self.theme_mode(), KicadDiagnosticSeverity::Error)
                }
            };
            ui.colored_label(
                color,
                format!(
                    "{}: {} ms, exit {:?}",
                    run.metadata.status.as_str(),
                    run.metadata.duration_ms,
                    run.metadata.exit_code
                ),
            );
            ui.monospace(run.output_dir.display().to_string());
            draw_simulation_report_panel(ui, &run.report);
            draw_simulation_artifacts_panel(ui, run);
            draw_simulation_waveform_panel(
                ui,
                self.theme_mode(),
                &run.waveform,
                &mut self.simulation_panel.selected_waveform_signal,
            );
        }
    }

    /// Sync the selected waveform signal with the latest run's available signals.
    /// Keeps the current selection if it's still valid, otherwise picks the default.
    pub(in crate::app) fn sync_selected_waveform_signal(
        &mut self,
        waveform: &GuiWaveformSummaryState,
    ) {
        let GuiWaveformSummaryState::Ready(summary) = waveform else {
            self.simulation_panel.selected_waveform_signal = None;
            return;
        };
        let keep_current = self
            .simulation_panel
            .selected_waveform_signal
            .as_deref()
            .is_some_and(|signal| summary.has_preview_signal(signal));
        if !keep_current {
            self.simulation_panel.selected_waveform_signal =
                summary.default_signal_name().map(ToOwned::to_owned);
        }
    }
}

/// Draw the list of simulation directives from the loaded schematic.
fn draw_simulation_directives(ui: &mut egui::Ui, directives: &[KicadSimulationDirective]) {
    ui.label(format!("{} directives", directives.len()));
    for directive in directives {
        ui.horizontal(|ui| {
            ui.monospace(directive.kind.to_string());
            ui.label(&directive.text);
        });
    }
}
