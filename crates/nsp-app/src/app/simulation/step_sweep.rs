//! `.step` parameter sweep configuration.
//!
//! Supports parametric sweeps over named circuit parameters (R, C, V, etc.)
//! and dedicated temperature sweeps using the `TEMP` keyword.

/// `.step` parameter sweep configuration.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) enum StepSweep {
    /// No sweep active.
    #[default]
    None,
    /// Parametric sweep over a named parameter.
    Parametric {
        param_name: String,
        sweep_mode: String,
        start: String,
        stop: String,
        step: String,
    },
    /// Temperature sweep: `.step TEMP lin start stop step`
    Temperature {
        sweep_mode: String,
        start: String,
        stop: String,
        step: String,
    },
}

impl StepSweep {
    /// Build the `.step` directive body string.
    pub(crate) fn to_directive(&self) -> Option<String> {
        match self {
            Self::None => None,
            Self::Parametric {
                param_name,
                sweep_mode,
                start,
                stop,
                step,
            } => match sweep_mode.as_str() {
                "list" => Some(format!(".step param {} list {}", param_name, start)),
                "lin" => Some(format!(
                    ".step param {} lin {} {} {}",
                    param_name, start, stop, step
                )),
                "dec" => Some(format!(
                    ".step param {} dec {} {} {}",
                    param_name, step, start, stop
                )),
                "oct" => Some(format!(
                    ".step param {} oct {} {} {}",
                    param_name, step, start, stop
                )),
                _ => Some(format!(
                    ".step param {} {} {} {}",
                    param_name, start, stop, step
                )),
            },
            Self::Temperature {
                sweep_mode,
                start,
                stop,
                step,
            } => match sweep_mode.as_str() {
                "lin" => Some(format!(".step TEMP lin {} {} {}", start, stop, step)),
                "dec" => Some(format!(".step TEMP dec {} {} {}", step, start, stop)),
                "oct" => Some(format!(".step TEMP oct {} {} {}", step, start, stop)),
                _ => Some(format!(".step TEMP lin {} {} {}", start, stop, step)),
            },
        }
    }

    /// Parse a `.step` directive body into structured parameters.
    pub(crate) fn from_directive_body(body: &str) -> Self {
        let parts: Vec<&str> = body.split_whitespace().collect();
        // Handle .step TEMP ...
        if parts.len() >= 3 && parts.first() == Some(&"TEMP") {
            let sweep_mode = parts[1].to_string();
            return Self::Temperature {
                sweep_mode,
                start: parts.get(2).map(|s| s.to_string()).unwrap_or_default(),
                stop: parts.get(3).map(|s| s.to_string()).unwrap_or_default(),
                step: parts.get(4).map(|s| s.to_string()).unwrap_or_default(),
            };
        }
        if parts.len() < 3 || parts[0] != "param" {
            return Self::None;
        }
        let param_name = parts[1].to_string();
        let sweep_mode = parts[2].to_string();
        match sweep_mode.as_str() {
            "list" => {
                let values = parts[3..].join(" ");
                Self::Parametric {
                    param_name,
                    sweep_mode,
                    start: values,
                    stop: String::new(),
                    step: String::new(),
                }
            }
            "lin" | "dec" | "oct" => Self::Parametric {
                param_name,
                sweep_mode,
                start: parts.get(3).map(|s| s.to_string()).unwrap_or_default(),
                stop: parts.get(4).map(|s| s.to_string()).unwrap_or_default(),
                step: parts.get(5).map(|s| s.to_string()).unwrap_or_default(),
            },
            _ => Self::None,
        }
    }
}
