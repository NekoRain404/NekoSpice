//! Structured analysis parameters and step sweep configuration.
//!
//! Each SPICE analysis type (`.tran`, `.ac`, `.dc`, `.op`, `.noise`,
//! `.disto`, `.sens`) is represented as a variant of [`AnalysisParams`]
//! with typed fields instead of a raw body string. This lets the UI
//! provide field-level validation, sensible defaults, and tooltips.
//!
//! [`StepSweep`] models the `.step` directive variants (linear,
//! decade, octave, and list sweeps).

use nsp_schema::NspSimulationDirectiveKind;

// ── Analysis Parameters ───────────────────────────────────────────────

/// Structured analysis parameters for each SPICE analysis type.
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
        sweep_type: String,
        npoints: String,
        fstart: String,
        fstop: String,
    },
    /// `.dc source vstart vstop vincr`
    Dc {
        source: String,
        vstart: String,
        vstop: String,
        vincr: String,
    },
    /// `.op` — no parameters
    Op,
    /// `.noise V(output) V(input) type npoints fstart fstop`
    Noise {
        output: String,
        input_source: String,
        sweep_type: String,
        npoints: String,
        fstart: String,
        fstop: String,
    },
    /// `.disto fstart fstop [fstep [maxharmonic]]`
    Disto {
        fstart: String,
        fstop: String,
        fstep: String,
        maxharmonic: String,
    },
    /// `.sens output_variable`
    Sens { output: String },
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
    /// Create default parameters for the given analysis kind.
    pub(crate) fn for_kind(kind: NspSimulationDirectiveKind) -> Self {
        match kind {
            NspSimulationDirectiveKind::Tran => Self::default(),
            NspSimulationDirectiveKind::Ac => Self::Ac {
                sweep_type: "dec".to_string(),
                npoints: "100".to_string(),
                fstart: "1".to_string(),
                fstop: "10Meg".to_string(),
            },
            NspSimulationDirectiveKind::Dc => Self::Dc {
                source: "V1".to_string(),
                vstart: "0".to_string(),
                vstop: "5".to_string(),
                vincr: "0.1".to_string(),
            },
            NspSimulationDirectiveKind::Op => Self::Op,
            NspSimulationDirectiveKind::Noise => Self::Noise {
                output: "V(out)".to_string(),
                input_source: "V(src)".to_string(),
                sweep_type: "dec".to_string(),
                npoints: "100".to_string(),
                fstart: "1".to_string(),
                fstop: "100Meg".to_string(),
            },
            NspSimulationDirectiveKind::Disto => Self::Disto {
                fstart: "1".to_string(),
                fstop: "100k".to_string(),
                fstep: "0".to_string(),
                maxharmonic: "3".to_string(),
            },
            NspSimulationDirectiveKind::Sens => Self::Sens {
                output: "V(out)".to_string(),
            },
            _ => Self::default(),
        }
    }

    /// Parse a raw directive body string into structured fields.
    ///
    /// Called when loading a schematic that already contains simulation
    /// directives, so the UI fields are pre-filled with the actual values.
    pub(crate) fn parse_body(&mut self, body: &str) {
        let parts: Vec<&str> = body.split_whitespace().collect();
        match self {
            Self::Tran {
                tstep,
                tstop,
                tstart,
                tmax,
                uic,
            } => {
                if let Some(v) = parts.first() {
                    *tstep = v.to_string();
                }
                if let Some(v) = parts.get(1) {
                    *tstop = v.to_string();
                }
                if let Some(v) = parts.get(2) {
                    *tstart = v.to_string();
                }
                if let Some(v) = parts.get(3)
                    && *v != "UIC"
                {
                    *tmax = v.to_string();
                }
                *uic = parts.contains(&"UIC");
            }
            Self::Ac {
                sweep_type,
                npoints,
                fstart,
                fstop,
            } => {
                if let Some(v) = parts.first() {
                    *sweep_type = v.to_string();
                }
                if let Some(v) = parts.get(1) {
                    *npoints = v.to_string();
                }
                if let Some(v) = parts.get(2) {
                    *fstart = v.to_string();
                }
                if let Some(v) = parts.get(3) {
                    *fstop = v.to_string();
                }
            }
            Self::Dc {
                source,
                vstart,
                vstop,
                vincr,
            } => {
                if let Some(v) = parts.first() {
                    *source = v.to_string();
                }
                if let Some(v) = parts.get(1) {
                    *vstart = v.to_string();
                }
                if let Some(v) = parts.get(2) {
                    *vstop = v.to_string();
                }
                if let Some(v) = parts.get(3) {
                    *vincr = v.to_string();
                }
            }
            Self::Noise {
                output,
                input_source,
                sweep_type,
                npoints,
                fstart,
                fstop,
            } => {
                if let Some(v) = parts.first() {
                    *output = v.to_string();
                }
                if let Some(v) = parts.get(1) {
                    *input_source = v.to_string();
                }
                if let Some(v) = parts.get(2) {
                    *sweep_type = v.to_string();
                }
                if let Some(v) = parts.get(3) {
                    *npoints = v.to_string();
                }
                if let Some(v) = parts.get(4) {
                    *fstart = v.to_string();
                }
                if let Some(v) = parts.get(5) {
                    *fstop = v.to_string();
                }
            }
            Self::Disto {
                fstart,
                fstop,
                fstep,
                maxharmonic,
            } => {
                if let Some(v) = parts.first() {
                    *fstart = v.to_string();
                }
                if let Some(v) = parts.get(1) {
                    *fstop = v.to_string();
                }
                if let Some(v) = parts.get(2) {
                    *fstep = v.to_string();
                }
                if let Some(v) = parts.get(3) {
                    *maxharmonic = v.to_string();
                }
            }
            Self::Sens { output } => {
                if !body.is_empty() {
                    *output = body.to_string();
                }
            }
            Self::Op => {}
        }
    }

    /// Build the SPICE directive body string from structured fields.
    pub(crate) fn to_body(&self) -> String {
        match self {
            Self::Tran {
                tstep,
                tstop,
                tstart,
                tmax,
                uic,
            } => {
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
            Self::Ac {
                sweep_type,
                npoints,
                fstart,
                fstop,
            } => {
                format!("{} {} {} {}", sweep_type, npoints, fstart, fstop)
            }
            Self::Dc {
                source,
                vstart,
                vstop,
                vincr,
            } => {
                format!("{} {} {} {}", source, vstart, vstop, vincr)
            }
            Self::Op => String::new(),
            Self::Noise {
                output,
                input_source,
                sweep_type,
                npoints,
                fstart,
                fstop,
            } => {
                format!(
                    "{} {} {} {} {} {}",
                    output.trim(),
                    input_source.trim(),
                    sweep_type,
                    npoints,
                    fstart,
                    fstop
                )
            }
            Self::Disto {
                fstart,
                fstop,
                fstep,
                maxharmonic,
            } => {
                if fstep.trim().is_empty() || fstep == "0" {
                    format!("{} {}", fstart, fstop)
                } else if maxharmonic.trim().is_empty() || maxharmonic == "3" {
                    format!("{} {} {}", fstart, fstop, fstep)
                } else {
                    format!("{} {} {} {}", fstart, fstop, fstep, maxharmonic)
                }
            }
            Self::Sens { output } => output.trim().to_string(),
        }
    }
}

pub(crate) use super::step_sweep::StepSweep;
