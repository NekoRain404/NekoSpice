//! Simulation panel state definitions.
//!
//! Contains the backend engine selector and the persistent state for
//! the simulation right panel. Structured analysis parameters and step
//! sweep configuration live in [`super::analysis`].

pub(crate) use super::analysis::{AnalysisParams, StepSweep};

use crate::simulation::{GuiSimulationRun, GuiSimulationTask};
use osl_kicad::KicadSimulationDirectiveKind;
use std::time::Instant;

// ── Backend Engine ────────────────────────────────────────────────────

/// Available simulation backend engines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SimulationBackendKind {
    Ngspice,
    Xyce,
}

impl SimulationBackendKind {
    pub(crate) const ALL: [Self; 2] = [Self::Ngspice, Self::Xyce];

    /// English label for display.
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Ngspice => "ngspice",
            Self::Xyce => "Xyce",
        }
    }

    /// Chinese label for display.
    pub(crate) fn label_zh(self) -> &'static str {
        match self {
            Self::Ngspice => "ngspice",
            Self::Xyce => "Xyce",
        }
    }
}

// ── Panel State ───────────────────────────────────────────────────────

/// Persistent state for the simulation right panel.
///
/// Tracks analysis parameters, run results, task tracking, and the
/// selected backend engine.
#[derive(Debug)]
pub(crate) struct SimulationPanelState {
    /// Currently selected analysis directive kind.
    pub(crate) directive_kind: KicadSimulationDirectiveKind,
    /// Structured analysis parameters (replaces raw body text).
    pub(crate) analysis_params: AnalysisParams,
    /// Whether to show the netlist preview section.
    #[allow(dead_code)]
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
    /// Netlist validation warnings from the last run attempt.
    pub(crate) netlist_warnings: Vec<String>,
    /// `.step` parameter sweep configuration.
    pub(crate) step_sweep: StepSweep,
    /// When the current simulation run started (for elapsed time display).
    pub(crate) run_start_time: Option<Instant>,
    /// Auto-run simulation when schematic changes (sweep parameter or temperature).
    pub(crate) auto_run_enabled: bool,
}

impl Default for SimulationPanelState {
    fn default() -> Self {
        Self {
            directive_kind: KicadSimulationDirectiveKind::Tran,
            analysis_params: AnalysisParams::default(),
            show_netlist: true,
            last_run: None,
            last_error: None,
            active_task: None,
            selected_waveform_signal: None,
            backend: SimulationBackendKind::Ngspice,
            netlist_warnings: Vec::new(),
            step_sweep: StepSweep::None,
            run_start_time: None,
            auto_run_enabled: false,
        }
    }
}

impl SimulationPanelState {
    /// Load persisted simulation settings from disk.
    /// Restores backend and directive_kind to their last-saved values.
    #[allow(dead_code)]
    pub(crate) fn from_disk() -> Self {
        let (_opts, _preset, backend_str, directive_str, _toggles) =
            crate::app::preferences::StudioPreferences::load_simulation_settings();
        let backend = match backend_str.as_str() {
            "Xyce" | "xyce" => SimulationBackendKind::Xyce,
            _ => SimulationBackendKind::Ngspice,
        };
        let directive_kind: KicadSimulationDirectiveKind = directive_str
            .parse()
            .unwrap_or(KicadSimulationDirectiveKind::Tran);
        let analysis_params = AnalysisParams::for_kind(directive_kind);
        Self {
            directive_kind,
            analysis_params,
            backend,
            ..Self::default()
        }
    }
}
