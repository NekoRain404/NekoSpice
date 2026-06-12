//! Simulation profile — carries all user-configured settings from the profile
//! editor and generates the corresponding SPICE directives for netlist injection.
//!
//! This module bridges the gap between the GUI profile editor (temperature,
//! tolerances, method, component/model overrides) and the actual netlist
//! that gets sent to ngspice/Xyce.

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
#[derive(Debug, Clone)]
pub struct SimulationProfile {
    /// Simulation temperature in degrees Celsius.
    pub temperature: String,
    /// Maximum number of Newton-Raphson iterations per timestep.
    pub max_iterations: String,
    /// Minimum allowed timestep in seconds ("0" = auto).
    pub min_timestep: String,
    /// SPICE integration method (Trap or Gear).
    pub method: SpiceMethod,
    /// Relative convergence tolerance (RELTOL).
    pub reltol: String,
    /// Absolute current convergence tolerance (ABSTOL).
    pub abstol: String,
    /// Absolute voltage convergence tolerance (VNTOL).
    pub vntol: String,
    /// Component parameter overrides from the profile editor.
    pub component_params: Vec<ProfileParamEntry>,
    /// Model parameter overrides from the profile editor.
    pub model_params: Vec<ProfileParamEntry>,
}

impl Default for SimulationProfile {
    fn default() -> Self {
        Self {
            temperature: "27".to_string(),
            max_iterations: "200".to_string(),
            min_timestep: "0".to_string(),
            method: SpiceMethod::Trap,
            reltol: "0.001".to_string(),
            abstol: "1e-12".to_string(),
            vntol: "1e-6".to_string(),
            component_params: Vec::new(),
            model_params: Vec::new(),
        }
    }
}

impl SimulationProfile {
    /// Generate SPICE directives from this profile's settings.
    ///
    /// Returns lines to be injected into the netlist (after the title line
    /// and before `.end`). Only non-default values are emitted to keep
    /// the netlist clean.
    pub fn generate_directives(&self) -> Vec<String> {
        let mut lines = Vec::new();

        // Temperature directive
        if self.temperature != "27" {
            lines.push(format!(".temp {}", self.temperature));
        }

        // Solver options — only emit if any differ from defaults
        if self.has_non_default_solver_options() {
            let mut opts = Vec::new();
            if self.max_iterations != "200" {
                opts.push(format!("itl1={}", self.max_iterations));
            }
            if self.min_timestep != "0" {
                opts.push(format!("trtol={}", self.min_timestep));
            }
            if self.method != SpiceMethod::Trap {
                opts.push(format!("method={}", self.method.as_spice_str()));
            }
            if self.reltol != "0.001" {
                opts.push(format!("reltol={}", self.reltol));
            }
            if self.abstol != "1e-12" {
                opts.push(format!("abstol={}", self.abstol));
            }
            if self.vntol != "1e-6" {
                opts.push(format!("vntol={}", self.vntol));
            }
            if !opts.is_empty() {
                lines.push(format!(".options {}", opts.join(" ")));
            }
        }

        // Component parameter overrides
        for param in &self.component_params {
            if param.is_valid() {
                lines.push(format!(".param {}={}", param.name.trim(), param.value.trim()));
            }
        }

        // Model parameter overrides
        for param in &self.model_params {
            if param.is_valid() {
                lines.push(format!(".param {}={}", param.name.trim(), param.value.trim()));
            }
        }

        lines
    }

    /// Returns true if any solver option differs from ngspice defaults.
    fn has_non_default_solver_options(&self) -> bool {
        self.max_iterations != "200"
            || self.min_timestep != "0"
            || self.method != SpiceMethod::Trap
            || self.reltol != "0.001"
            || self.abstol != "1e-12"
            || self.vntol != "1e-6"
    }

    /// Returns true if this profile has any non-trivial settings that
    /// would affect simulation results.
    pub fn has_custom_settings(&self) -> bool {
        self.temperature != "27"
            || self.has_non_default_solver_options()
            || self.component_params.iter().any(ProfileParamEntry::is_valid)
            || self.model_params.iter().any(ProfileParamEntry::is_valid)
    }
}


/// Inject profile directives into a SPICE netlist.
///
/// Inserts profile-generated directives (`.temp`, `.options`, `.param`) into
/// the netlist body, right before the `.end` directive. This ensures the
/// solver receives all user-configured settings without modifying the
/// schematic's own directives.
pub fn inject_profile_directives(netlist: &str, profile: &SimulationProfile) -> String {
    let directives = profile.generate_directives();
    if directives.is_empty() {
        return netlist.to_string();
    }

    // Insert profile directives before `.end`
    let mut output = Vec::new();
    let mut inserted = false;

    for line in netlist.lines() {
        if !inserted && line.trim().eq_ignore_ascii_case(".end") {
            // Insert a separator comment + all profile directives
            output.push("* --- NekoSpice simulation profile ---".to_string());
            for directive in &directives {
                output.push(directive.clone());
            }
            inserted = true;
        }
        output.push(line.to_string());
    }

    // If no `.end` found (shouldn't happen in valid netlist), append at end
    if !inserted {
        output.push("* --- NekoSpice simulation profile ---".to_string());
        output.extend(directives);
    }

    output.join("\n")
}

/// Validate that a netlist is runnable by ngspice/Xyce.
///
/// Checks for:
/// - Non-empty netlist
/// - Presence of `.end` directive
/// - At least one analysis directive (.tran, .ac, .dc, .op)
///
/// Returns a list of warning/error messages. Empty vec means valid.
pub fn validate_netlist_for_simulation(netlist: &str) -> Vec<String> {
    let mut issues = Vec::new();
    let trimmed = netlist.trim();

    if trimmed.is_empty() {
        issues.push("Netlist is empty".to_string());
        return issues;
    }

    let has_end = trimmed
        .lines()
        .any(|line| line.trim().eq_ignore_ascii_case(".end"));
    if !has_end {
        issues.push("Missing .end directive".to_string());
    }

    let has_analysis = trimmed.lines().any(|line| {
        let lower = line.trim().to_ascii_lowercase();
        lower.starts_with(".tran ")
            || lower.starts_with(".ac ")
            || lower.starts_with(".dc ")
            || lower == ".op"
    });
    if !has_analysis {
        issues.push(
            "No analysis directive found (.tran, .ac, .dc, .op)".to_string(),
        );
    }

    issues
}

/// Parse ngspice log file to extract errors and warnings.
///
/// Returns (errors, warnings, summary_line).
pub fn parse_ngspice_log(log_content: &str) -> (Vec<String>, Vec<String>, Option<String>) {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let mut summary_line = None;

    for line in log_content.lines() {
        let trimmed = line.trim();
        let lower = trimmed.to_ascii_lowercase();

        if lower.contains("error") || lower.contains("fatal") {
            errors.push(trimmed.to_string());
        } else if lower.contains("warning") {
            warnings.push(trimmed.to_string());
        }

        // Look for convergence/success/failure summary lines
        if lower.contains("simulation aborted")
            || lower.contains("simulation done")
            || lower.contains("tran analysis")
            || lower.contains("operating point")
            || lower.contains("convergence")
            || lower.contains("singular matrix")
            || lower.contains("doAnalyses")
        {
            summary_line = Some(trimmed.to_string());
        }
    }

    (errors, warnings, summary_line)
}

/// Format ngspice log issues into a user-friendly message.
pub fn format_simulation_log_summary(
    errors: &[String],
    warnings: &[String],
    summary: Option<&str>,
) -> String {
    let mut parts = Vec::new();

    if let Some(summary) = summary {
        parts.push(summary.to_string());
    }

    if !errors.is_empty() {
        parts.push(format!("{} error(s) found", errors.len()));
        // Include first 3 errors for brevity
        for error in errors.iter().take(3) {
            parts.push(format!("  → {}", error));
        }
    }

    if !warnings.is_empty() {
        parts.push(format!("{} warning(s)", warnings.len()));
    }

    if parts.is_empty() {
        "No issues found in log".to_string()
    } else {
        parts.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inject_directives_before_end() {
        let netlist = "* RC filter\nR1 in out 1k\nC1 out 0 100n\n.end\n";
        let mut profile = SimulationProfile::default();
        profile.temperature = "125".to_string();
        let result = inject_profile_directives(netlist, &profile);
        assert!(result.contains(".temp 125"));
        assert!(result.contains(".end"), "netlist should end with .end");
        assert!(result.trim_end().ends_with(".end"), "netlist should end with .end directive");
        // Directive should be before .end
        let temp_pos = result.find(".temp 125").unwrap();
        let end_pos = result.find(".end").unwrap();
        assert!(temp_pos < end_pos);
    }

    #[test]
    fn no_directives_passthrough() {
        let netlist = "* RC filter\nR1 in out 1k\n.end\n";
        let profile = SimulationProfile::default();
        let result = inject_profile_directives(netlist, &profile);
        assert_eq!(result, netlist);
    }

    #[test]
    fn validate_empty_netlist() {
        let issues = validate_netlist_for_simulation("");
        assert!(issues.iter().any(|i| i.contains("empty")));
    }

    #[test]
    fn validate_missing_end() {
        let issues = validate_netlist_for_simulation("* RC\n.tran 1u 1m\nR1 in out 1k\n");
        assert!(issues.iter().any(|i| i.contains(".end")));
    }

    #[test]
    fn validate_missing_analysis() {
        let issues = validate_netlist_for_simulation("* RC\nR1 in out 1k\n.end\n");
        assert!(issues.iter().any(|i| i.contains("analysis")));
    }

    #[test]
    fn validate_valid_netlist() {
        let issues = validate_netlist_for_simulation("* RC\n.tran 1u 1m\nR1 in out 1k\n.end\n");
        assert!(issues.is_empty());
    }

    #[test]
    fn parse_log_extracts_errors() {
        let log = "ngspice warning: singular matrix\nError: cannot find model\nDone.";
        let (errors, warnings, summary) = parse_ngspice_log(log);
        assert!(errors.iter().any(|e| e.contains("cannot find model")));
        assert!(warnings.iter().any(|w| w.contains("singular matrix")));
    }
}
