//! Simulation panel state definitions.
//!
//! Contains the backend engine selector and the persistent state for the
//! simulation right panel (directive kind/body, run results, task tracking).

use crate::simulation::{GuiSimulationRun, GuiSimulationTask};
use osl_kicad::KicadSimulationDirectiveKind;

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

/// Persistent state for the simulation right panel.
///
/// Tracks the current directive kind/body, whether to show the netlist preview,
/// the last completed run, any error, the currently running task, and the
/// selected waveform signal for display.
#[derive(Debug)]
pub(crate) struct SimulationPanelState {
    /// Currently selected analysis directive kind (.tran, .ac, .dc, .op).
    pub(crate) directive_kind: KicadSimulationDirectiveKind,
    /// Directive body text (e.g., "1u 1m" for .tran).
    pub(crate) directive_body: String,
    /// Whether to show the netlist preview section.
    #[allow(dead_code)] pub(crate) show_netlist: bool,
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
