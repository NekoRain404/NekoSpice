//! Simulation panel state definitions.
//!
//! Contains the backend engine selector, analysis parameters, and the
//! persistent state for the simulation right panel.
//!
//! The `AnalysisParams` enum provides structured editing for each SPICE
//! analysis type, replacing raw text body editing with proper fields.

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

/// Structured analysis parameters for each SPICE analysis type.
///
/// Instead of requiring the user to type raw SPICE syntax, each analysis
/// type exposes its parameters as named fields with sensible defaults.
#[derive(Debug, Clone)]
pub(crate) enum AnalysisParams {
    /// `.tran tstep tstop [tstart [tmax]] [UIC]`
    Tran {
        tstep: String,
        tstop: String,
        tstart: String,
        tmax: String,
        uic: bool,
    },
    /// `.ac type npoints fstart fstop`
    Ac {
        /// Sweep type: "dec", "lin", or "oct"
        sweep_type: String,
        npoints: String,
        fstart: String,
        fstop: String,
    },
    /// `.dc source vstart vstop vincr [source2 start2 stop2 incr2]`
    Dc {
        source: String,
        vstart: String,
        vstop: String,
        vincr: String,
    },
    /// `.op` — no parameters
    Op,
}

impl Default for AnalysisParams {
    fn default() -> Self {
        Self::Tran {
            tstep: "1u".to_string(),
            tstop: "1m".to_string(),
            tstart: "0".to_string(),
            tmax: "0".to_string(),
            uic: false,
        }
    }
}

impl AnalysisParams {
    /// Create default params for the given analysis kind.
    pub(crate) fn for_kind(kind: KicadSimulationDirectiveKind) -> Self {
        match kind {
            KicadSimulationDirectiveKind::Tran => Self::Tran {
                tstep: "1u".to_string(),
                tstop: "1m".to_string(),
                tstart: "0".to_string(),
                tmax: "0".to_string(),
                uic: false,
            },
            KicadSimulationDirectiveKind::Ac => Self::Ac {
                sweep_type: "dec".to_string(),
                npoints: "10".to_string(),
                fstart: "1".to_string(),
                fstop: "1Meg".to_string(),
            },
            KicadSimulationDirectiveKind::Dc => Self::Dc {
                source: "V1".to_string(),
                vstart: "0".to_string(),
                vstop: "5".to_string(),
                vincr: "0.1".to_string(),
            },
            KicadSimulationDirectiveKind::Op => Self::Op,
            _ => Self::default(),
        }
    }

    /// Build the SPICE directive body string from structured fields.
    pub(crate) fn to_body(&self) -> String {
        match self {
            Self::Tran { tstep, tstop, tstart, tmax, uic } => {
                let mut parts = vec![tstep.clone(), tstop.clone()];
                if !tstart.trim().is_empty() && tstart != "0" {
                    parts.push(tstart.clone());
                }
                if !tmax.trim().is_empty() && tmax != "0" {
                    parts.push(tmax.clone());
                }
                if *uic {
                    parts.push("UIC".to_string());
                }
                parts.join(" ")
            }
            Self::Ac { sweep_type, npoints, fstart, fstop } => {
                format!("{} {} {} {}", sweep_type, npoints, fstart, fstop)
            }
            Self::Dc { source, vstart, vstop, vincr } => {
                format!("{} {} {} {}", source, vstart, vstop, vincr)
            }
            Self::Op => String::new(),
        }
    }

}

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
        }
    }
}
