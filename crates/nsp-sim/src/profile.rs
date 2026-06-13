//! Simulation profile — carries all user-configured settings from the profile
//! editor and generates the corresponding SPICE directives for netlist injection.
//!
//! This module bridges the gap between the GUI profile editor (temperature,
//! tolerances, method, component/model overrides) and the actual netlist
//! that gets sent to ngspice/Xyce.
//!
//! # SPICE Options Reference
//!
//! The following SPICE `.options` parameters are supported:
//!
//! | Parameter  | Default   | Description                              |
//! |------------|-----------|------------------------------------------|
//! | `TEMP`     | 27        | Operating temperature (°C)               |
//! | `TNOM`     | 27        | Nominal temperature for models (°C)      |
//! | `METHOD`   | trap      | Integration method (trap/gear)           |
//! | `RELTOL`   | 0.001     | Relative convergence tolerance           |
//! | `ABSTOL`   | 1e-12     | Absolute current tolerance (A)           |
//! | `VNTOL`    | 1e-6      | Absolute voltage tolerance (V)           |
//! | `GMIN`     | 1e-12     | Minimum conductance (S)                  |
//! | `CHGTOL`   | 1e-14     | Charge tolerance (C)                     |
//! | `PIVTOL`   | 1e-13     | Pivot tolerance for LU decomposition     |
//! | `PIVREL`   | 1e-3      | Relative pivot tolerance                 |
//! | `ITL1`     | 100       | DC operating point iteration limit       |
//! | `ITL2`     | 50        | DC transfer curve iteration limit        |
//! | `ITL4`     | 10        | Transient timestep iteration limit       |
//! | `ITL5`     | 5000      | Transient total iteration limit (0=off)  |
//! | `SRCSTEPS` | 0         | Source stepping iterations (0=auto)      |
//! | `GMINSTEPS`=| 0        | GMIN stepping iterations (0=auto)        |
//! | `NUMDGT`   | 6         | Number of significant digits in output   |

/// A single component or model parameter override from the profile editor.
/// Each row in the UI's component/model params table maps to one of these.
#[derive(Debug, Clone, PartialEq)]
pub struct ProfileParamEntry {
    /// Component reference or model name (e.g. "R1", "C1", "opamp_model").
    pub name: String,
    /// Parameter value (e.g. "10k", "100n", "0.001").
    pub value: String,
    /// Unit hint for display (e.g. "ohm", "F", "V"). Not sent to SPICE.
    pub unit: String,
}

impl ProfileParamEntry {
    /// Returns true if this entry has a non-empty name and value.
    pub fn is_valid(&self) -> bool {
        !self.name.trim().is_empty() && !self.value.trim().is_empty()
    }
}

/// SPICE integration method for transient analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpiceMethod {
    /// Trapezoidal integration (default, best accuracy for most circuits).
    Trap,
    /// Gear integration (more stable for stiff systems, less accurate).
    Gear,
}

impl SpiceMethod {
    /// Parse from string, defaulting to Trap on unknown input.
    pub fn from_str_loose(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "gear" => Self::Gear,
            _ => Self::Trap,
        }
    }

    /// String representation for SPICE `.options` directive.
    pub fn as_spice_str(self) -> &'static str {
        match self {
            Self::Trap => "trap",
            Self::Gear => "gear",
        }
    }
}

/// Complete simulation profile carrying all user-configurable settings.
///
/// This struct is populated by the GUI profile editor and used to generate
/// SPICE directives (`.options`, `.param`, `.temperature`) that get injected
/// into the netlist before it's sent to the solver backend.
///
/// The struct is organized into logical groups:
/// - **Analysis**: `.tran`, `.ac`, `.dc`, `.op` directive configuration
/// - **Environment**: temperature, nominal temperature
/// - **Transient**: integration method, iteration limits, timestep control
/// - **Convergence**: tolerances and minimum conductance
/// - **Initial Conditions**: `.ic` and `.nodeset` entries
/// - **Output**: digit precision control
/// - **Overrides**: component/model parameter `.param` entries
/// - **Presets**: named configuration presets
#[derive(Debug, Clone)]
pub struct SimulationProfile {
    // ── Analysis ───────────────────────────────────────────────────────
    /// Analysis type selected in the UI (.tran, .ac, .dc, .op).
    pub analysis_kind: String,
    /// Analysis body text from the UI (e.g. "1u 1m" for .tran).
    pub analysis_body: String,

    // ── Environment ────────────────────────────────────────────────────
    /// Simulation temperature in degrees Celsius.
    pub temperature: String,
    /// Nominal temperature for model evaluation (°C). Default: 27.
    pub tnom: String,

    // ── Transient Solver ───────────────────────────────────────────────
    /// SPICE integration method (Trap or Gear).
    pub method: SpiceMethod,
    /// DC operating point iteration limit (ITL1).
    pub itl1: String,
    /// DC transfer curve iteration limit (ITL2).
    pub itl2: String,
    /// Transient timestep iteration limit (ITL4).
    pub itl4: String,
    /// Transient total iteration limit (ITL5, 0 = no limit).
    pub itl5: String,
    /// Minimum allowed timestep in seconds (TRTOL). 0 = auto.
    pub min_timestep: String,
    /// Source stepping iterations (0 = auto).
    pub srcsteps: String,
    /// GMIN stepping iterations (0 = auto).
    pub gminsteps: String,

    // ── Convergence ────────────────────────────────────────────────────
    /// Relative convergence tolerance (RELTOL).
    pub reltol: String,
    /// Absolute current convergence tolerance (ABSTOL).
    pub abstol: String,
    /// Absolute voltage convergence tolerance (VNTOL).
    pub vntol: String,
    /// Minimum conductance to ground (GMIN).
    pub gmin: String,
    /// Charge convergence tolerance (CHGTOL).
    pub chgtol: String,
    /// Absolute pivot tolerance for LU decomposition (PIVTOL).
    pub pivtol: String,
    /// Relative pivot tolerance (PIVREL).
    pub pivrel: String,

    // ── Output Control ─────────────────────────────────────────────────
    /// Number of significant digits in output (NUMDGT).
    pub numdgt: String,

    // ── Initial Conditions ─────────────────────────────────────────────
    /// `.ic` entries: (node_name, initial_voltage).
    /// Example: [("V(out)", "2.5"), ("V(in)", "0")]
    pub initial_conditions: Vec<(String, String)>,
    /// `.nodeset` entries: (node_name, initial_guess).
    /// Helps convergence by providing operating point hints.
    pub nodesets: Vec<(String, String)>,

    // ── Parameter Overrides ────────────────────────────────────────────
    /// Component parameter overrides from the profile editor.
    pub component_params: Vec<ProfileParamEntry>,
    /// Model parameter overrides from the profile editor.
    pub model_params: Vec<ProfileParamEntry>,

    // ── Sweep ─────────────────────────────────────────────────────────
    /// `.step` parameter sweep directive (optional).
    /// When set, generates a `.step` line before the analysis directive.
    pub step_directive: Option<String>,

    // ── Measurements ──────────────────────────────────────────────────
    /// `.measure` directives for post-simulation extraction.
    pub measure_directives: Vec<String>,

    // -- Vendor Models --
    /// SPICE subcircuit/model body text from vendor models (TI/ADI).
    /// Injected before .end so the solver can instantiate vendor components.
    pub vendor_model_bodies: Vec<String>,
}

impl Default for SimulationProfile {
    fn default() -> Self {
        Self {
            // Analysis
            analysis_kind: ".tran".to_string(),
            analysis_body: "1u 1m".to_string(),
            // Environment
            temperature: "27".to_string(),
            tnom: "27".to_string(),
            // Transient
            method: SpiceMethod::Trap,
            itl1: "100".to_string(),
            itl2: "50".to_string(),
            itl4: "10".to_string(),
            itl5: "5000".to_string(),
            min_timestep: "0".to_string(),
            srcsteps: "0".to_string(),
            gminsteps: "0".to_string(),
            // Convergence
            reltol: "0.001".to_string(),
            abstol: "1e-12".to_string(),
            vntol: "1e-6".to_string(),
            gmin: "1e-12".to_string(),
            chgtol: "1e-14".to_string(),
            pivtol: "1e-13".to_string(),
            pivrel: "1e-3".to_string(),
            // Output
            numdgt: "6".to_string(),
            // Initial conditions
            initial_conditions: Vec::new(),
            nodesets: Vec::new(),
            // Parameter overrides
            component_params: Vec::new(),
            model_params: Vec::new(),
            // Vendor models
            vendor_model_bodies: Vec::new(),
            step_directive: None,
            measure_directives: Vec::new(),
        }
    }
}

/// Named presets for common simulation configurations.
/// Returns a `SimulationProfile` pre-configured for the named preset.
pub fn simulation_preset(name: &str) -> SimulationProfile {
    let mut profile = SimulationProfile::default();
    match name {
        "fast" => {
            profile.reltol = "0.01".to_string();
            profile.abstol = "1e-9".to_string();
            profile.vntol = "1e-4".to_string();
            profile.itl4 = "20".to_string();
            profile.itl1 = "50".to_string();
            profile.min_timestep = "0".to_string();
        }
        "accurate" => {
            profile.reltol = "1e-5".to_string();
            profile.abstol = "1e-15".to_string();
            profile.vntol = "1e-8".to_string();
            profile.itl4 = "50".to_string();
            profile.itl1 = "200".to_string();
            profile.method = SpiceMethod::Gear;
        }
        "high-freq" => {
            profile.reltol = "1e-4".to_string();
            profile.abstol = "1e-12".to_string();
            profile.vntol = "1e-6".to_string();
            profile.itl4 = "50".to_string();
            profile.method = SpiceMethod::Trap;
        }
        "convergence-help" => {
            profile.gmin = "1e-10".to_string();
            profile.reltol = "0.01".to_string();
            profile.abstol = "1e-10".to_string();
            profile.vntol = "1e-4".to_string();
            profile.srcsteps = "100".to_string();
            profile.gminsteps = "10".to_string();
            profile.itl1 = "200".to_string();
            profile.itl4 = "100".to_string();
        }
        "power-electronics" => {
            // Switching converters, motor drives, power supplies
            profile.reltol = "0.005".to_string();
            profile.abstol = "1e-10".to_string();
            profile.vntol = "1e-5".to_string();
            profile.itl4 = "30".to_string();
            profile.itl1 = "150".to_string();
            profile.min_timestep = "1n".to_string();
            profile.method = SpiceMethod::Trap;
        }
        "low-power" => {
            // Ultra-low-power IoT, battery-operated, sensor circuits
            profile.reltol = "1e-4".to_string();
            profile.abstol = "1e-15".to_string();
            profile.vntol = "1e-9".to_string();
            profile.gmin = "1e-14".to_string();
            profile.chgtol = "1e-16".to_string();
            profile.itl4 = "40".to_string();
            profile.itl1 = "200".to_string();
            profile.method = SpiceMethod::Trap;
        }
        "precision" => {
            // Precision instrumentation, ADC/DAC, measurement frontends
            profile.reltol = "1e-6".to_string();
            profile.abstol = "1e-16".to_string();
            profile.vntol = "1e-9".to_string();
            profile.gmin = "1e-15".to_string();
            profile.chgtol = "1e-18".to_string();
            profile.itl4 = "80".to_string();
            profile.itl1 = "300".to_string();
            profile.numdgt = "8".to_string();
            profile.method = SpiceMethod::Gear;
        }
        "mixed-signal" => {
            // ADCs, PLLs, clock recovery — mix of fast and slow dynamics
            profile.reltol = "1e-4".to_string();
            profile.abstol = "1e-12".to_string();
            profile.vntol = "1e-6".to_string();
            profile.itl4 = "40".to_string();
            profile.itl5 = "10000".to_string();
            profile.srcsteps = "20".to_string();
            profile.gminsteps = "5".to_string();
            profile.method = SpiceMethod::Trap;
        }
        "rf" => {
            // RF circuits, mixers, oscillators — tight accuracy + high frequency
            profile.reltol = "1e-4".to_string();
            profile.abstol = "1e-13".to_string();
            profile.vntol = "1e-7".to_string();
            profile.itl4 = "60".to_string();
            profile.itl1 = "200".to_string();
            profile.method = SpiceMethod::Gear;
            profile.numdgt = "8".to_string();
        }
        _ => {} // "default" or unknown → return default profile
    }
    profile
}

/// Returns the list of available preset names for display in the UI.
pub fn available_presets() -> &'static [(&'static str, &'static str)] {
    &[
        ("default", "Default"),
        ("fast", "Fast"),
        ("accurate", "Accurate"),
        ("high-freq", "High Frequency"),
        ("convergence-help", "Convergence Aid"),
        ("power-electronics", "Power Electronics"),
        ("low-power", "Low Power"),
        ("precision", "Precision"),
        ("mixed-signal", "Mixed Signal"),
        ("rf", "RF"),
    ]
}

impl SimulationProfile {
    /// Generate SPICE directives from this profile's settings.
    ///
    /// Returns lines to be injected into the netlist (after the title line
    /// and before `.end`). Only non-default values are emitted to keep
    /// the netlist clean.
    pub fn generate_directives(&self) -> Vec<String> {
        let mut lines = Vec::new();

        // ── Analysis directive ─────────────────────────────────────────
        // .step directive (before analysis)
        if let Some(ref step) = self.step_directive
            && !step.trim().is_empty()
        {
            lines.push(step.trim().to_string());
        }
        // Analysis directive
        let analysis_line = if self.analysis_body.trim().is_empty() {
            self.analysis_kind.to_string()
        } else {
            format!("{} {}", self.analysis_kind, self.analysis_body.trim())
        };
        lines.push(analysis_line);

        // ── Temperature ────────────────────────────────────────────────
        if self.temperature != "27" {
            lines.push(format!(".temp {}", self.temperature));
        }

        // ── .options — solver settings ─────────────────────────────────
        if self.has_non_default_solver_options() {
            let mut opts = Vec::new();

            // Environment
            if self.tnom != "27" {
                opts.push(format!("tnom={}", self.tnom));
            }

            // Method
            if self.method != SpiceMethod::Trap {
                opts.push(format!("method={}", self.method.as_spice_str()));
            }

            // Iteration limits
            if self.itl1 != "100" {
                opts.push(format!("itl1={}", self.itl1));
            }
            if self.itl2 != "50" {
                opts.push(format!("itl2={}", self.itl2));
            }
            if self.itl4 != "10" {
                opts.push(format!("itl4={}", self.itl4));
            }
            if self.itl5 != "5000" {
                opts.push(format!("itl5={}", self.itl5));
            }

            // Timestep control
            if self.min_timestep != "0" {
                opts.push(format!("trtol={}", self.min_timestep));
            }

            // Convergence tolerances
            if self.reltol != "0.001" {
                opts.push(format!("reltol={}", self.reltol));
            }
            if self.abstol != "1e-12" {
                opts.push(format!("abstol={}", self.abstol));
            }
            if self.vntol != "1e-6" {
                opts.push(format!("vntol={}", self.vntol));
            }
            if self.gmin != "1e-12" {
                opts.push(format!("gmin={}", self.gmin));
            }
            if self.chgtol != "1e-14" {
                opts.push(format!("chgtol={}", self.chgtol));
            }
            if self.pivtol != "1e-13" {
                opts.push(format!("pivtol={}", self.pivtol));
            }
            if self.pivrel != "1e-3" {
                opts.push(format!("pivrel={}", self.pivrel));
            }

            // Advanced solver
            if self.srcsteps != "0" {
                opts.push(format!("srcsteps={}", self.srcsteps));
            }
            if self.gminsteps != "0" {
                opts.push(format!("gminsteps={}", self.gminsteps));
            }

            // Output
            if self.numdgt != "6" {
                opts.push(format!("numdgt={}", self.numdgt));
            }

            if !opts.is_empty() {
                lines.push(format!(".options {}", opts.join(" ")));
            }
        }

        // ── Measurements ─────────────────────────────────────────────────
        for measure in &self.measure_directives {
            if !measure.trim().is_empty() {
                lines.push(measure.trim().to_string());
            }
        }

        // ── Initial conditions ─────────────────────────────────────────
        if !self.initial_conditions.is_empty() {
            let entries: Vec<String> = self
                .initial_conditions
                .iter()
                .filter(|(n, v)| !n.trim().is_empty() && !v.trim().is_empty())
                .map(|(n, v)| format!("{}={}", n.trim(), v.trim()))
                .collect();
            if !entries.is_empty() {
                lines.push(format!(".ic {}", entries.join(" ")));
            }
        }

        // ── Nodesets ───────────────────────────────────────────────────
        if !self.nodesets.is_empty() {
            let entries: Vec<String> = self
                .nodesets
                .iter()
                .filter(|(n, v)| !n.trim().is_empty() && !v.trim().is_empty())
                .map(|(n, v)| format!("{}={}", n.trim(), v.trim()))
                .collect();
            if !entries.is_empty() {
                lines.push(format!(".nodeset {}", entries.join(" ")));
            }
        }

        // ── Component parameter overrides ──────────────────────────────
        for param in &self.component_params {
            if param.is_valid() {
                lines.push(format!(
                    ".param {}={}",
                    param.name.trim(),
                    param.value.trim()
                ));
            }
        }

        // ── Model parameter overrides ──────────────────────────────────
        for param in &self.model_params {
            if param.is_valid() {
                lines.push(format!(
                    ".param {}={}",
                    param.name.trim(),
                    param.value.trim()
                ));
            }
        }

        lines
    }

    /// Returns true if any solver option differs from SPICE defaults.
    fn has_non_default_solver_options(&self) -> bool {
        self.tnom != "27"
            || self.method != SpiceMethod::Trap
            || self.itl1 != "100"
            || self.itl2 != "50"
            || self.itl4 != "10"
            || self.itl5 != "5000"
            || self.min_timestep != "0"
            || self.srcsteps != "0"
            || self.gminsteps != "0"
            || self.reltol != "0.001"
            || self.abstol != "1e-12"
            || self.vntol != "1e-6"
            || self.gmin != "1e-12"
            || self.chgtol != "1e-14"
            || self.pivtol != "1e-13"
            || self.pivrel != "1e-3"
            || self.numdgt != "6"
    }

    /// Returns true if this profile has any non-trivial settings that
    /// would affect simulation results.
    pub fn has_custom_settings(&self) -> bool {
        self.temperature != "27"
            || self.has_non_default_solver_options()
            || !self.initial_conditions.is_empty()
            || !self.nodesets.is_empty()
            || self
                .component_params
                .iter()
                .any(ProfileParamEntry::is_valid)
            || self.model_params.iter().any(ProfileParamEntry::is_valid)
            || self.step_directive.is_some()
            || !self.measure_directives.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preset_fast_relaxes_tolerances() {
        let fast = simulation_preset("fast");
        assert_eq!(fast.reltol, "0.01");
        assert_eq!(fast.abstol, "1e-9");
    }

    #[test]
    fn preset_accurate_tightens_tolerances() {
        let acc = simulation_preset("accurate");
        assert_eq!(acc.reltol, "1e-5");
        assert_eq!(acc.method, SpiceMethod::Gear);
    }

    #[test]
    fn preset_convergence_help_sets_srcsteps() {
        let conv = simulation_preset("convergence-help");
        assert_eq!(conv.srcsteps, "100");
        assert_eq!(conv.gminsteps, "10");
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn generate_directives_includes_new_options() {
        let mut profile = SimulationProfile::default();
        profile.gmin = "1e-10".to_string();
        profile.itl4 = "50".to_string();
        profile.numdgt = "8".to_string();
        let directives = profile.generate_directives();
        let options_line = directives
            .iter()
            .find(|l| l.starts_with(".options"))
            .unwrap();
        assert!(options_line.contains("gmin=1e-10"));
        assert!(options_line.contains("itl4=50"));
        assert!(options_line.contains("numdgt=8"));
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn generate_directives_with_ic_and_nodeset() {
        let mut profile = SimulationProfile::default();
        profile.initial_conditions = vec![("V(out)".to_string(), "2.5".to_string())];
        profile.nodesets = vec![("V(out)".to_string(), "1.0".to_string())];
        let directives = profile.generate_directives();
        assert!(directives.iter().any(|l| l.contains(".ic V(out)=2.5")));
        assert!(directives.iter().any(|l| l.contains(".nodeset V(out)=1.0")));
    }

    #[test]
    fn default_profile_has_no_custom_settings() {
        let profile = SimulationProfile::default();
        assert!(!profile.has_custom_settings());
    }
}
