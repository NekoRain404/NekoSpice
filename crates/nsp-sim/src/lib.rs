//! Simulation backend library - ngspice/Xyce profile building, netlist injection, and log parsing.

mod artifacts;
mod profile;
mod profile_inject;
mod profile_log;

pub use profile::{
    ProfileParamEntry, SimulationProfile, SpiceMethod, available_presets, simulation_preset,
};

pub use profile_inject::{inject_profile_directives, validate_netlist_for_simulation};

pub use profile_log::{format_simulation_log_summary, parse_ngspice_log};

pub use artifacts::{
    collect_run_artifacts, export_waveform_artifacts, finalize_run_artifacts,
    refresh_run_artifacts, run_artifact_kind, run_report_html, write_run_report,
};

use nsp_core::{
    OslError, OslResult, ParameterOverride, RunMetadata, RunStatus, make_run_id, now_unix_ms,
    read_text, write_text,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct BackendCapabilities {
    pub batch: bool,
    pub process_isolated: bool,
    pub writes_binary_waveform: bool,
}

pub trait SimulatorBackend {
    fn name(&self) -> &'static str;
    fn capabilities(&self) -> BackendCapabilities;
    fn run(&self, source_netlist: &Path, output_dir: &Path) -> OslResult<RunMetadata>;
    fn run_with_parameters(
        &self,
        source_netlist: &Path,
        output_dir: &Path,
        parameters: &[ParameterOverride],
    ) -> OslResult<RunMetadata>;
}

#[derive(Debug, Clone)]
pub struct NgspiceCliBackend {
    executable: PathBuf,
}

impl NgspiceCliBackend {
    /// new。
    pub fn new(executable: impl Into<PathBuf>) -> Self {
        Self {
            executable: executable.into(),
        }
    }

    /// executable。
    pub fn executable(&self) -> &Path {
        &self.executable
    }
}

impl Default for NgspiceCliBackend {
    fn default() -> Self {
        Self::new("ngspice")
    }
}

impl SimulatorBackend for NgspiceCliBackend {
    fn name(&self) -> &'static str {
        "ngspice-cli"
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            batch: true,
            process_isolated: true,
            writes_binary_waveform: true,
        }
    }

    fn run(&self, source_netlist: &Path, output_dir: &Path) -> OslResult<RunMetadata> {
        self.run_with_parameters(source_netlist, output_dir, &[])
    }

    fn run_with_parameters(
        &self,
        source_netlist: &Path,
        output_dir: &Path,
        parameters: &[ParameterOverride],
    ) -> OslResult<RunMetadata> {
        if !source_netlist.is_file() {
            return Err(OslError::InvalidInput(format!(
                "netlist does not exist: {}",
                source_netlist.display()
            )));
        }

        fs::create_dir_all(output_dir)
            .map_err(|err| OslError::io(format!("create {}", output_dir.display()), err))?;

        let source_abs = fs::canonicalize(source_netlist).map_err(|err| {
            OslError::io(format!("canonicalize {}", source_netlist.display()), err)
        })?;
        let output_abs = fs::canonicalize(output_dir)
            .map_err(|err| OslError::io(format!("canonicalize {}", output_dir.display()), err))?;
        let working_netlist = output_abs.join("input.cir");

        let source = read_text(&source_abs)?;
        let working_source =
            apply_parameter_overrides(&ensure_ngspice_control_exports(&source), parameters);
        write_text(&working_netlist, &working_source)?;
        copy_relative_dependencies(&source_abs, &source, &output_abs)?;

        let started_unix_ms = now_unix_ms();
        let timer = Instant::now();
        let output = Command::new(&self.executable)
            .arg("-b")
            .arg("-o")
            .arg("ngspice.log")
            .arg("input.cir")
            .current_dir(&output_abs)
            .output()
            .map_err(|err| {
                OslError::io(
                    format!("run {} -b input.cir", self.executable.display()),
                    err,
                )
            })?;
        let duration_ms = timer.elapsed().as_millis();

        write_text(
            &output_abs.join("stdout.txt"),
            &String::from_utf8_lossy(&output.stdout),
        )?;
        write_text(
            &output_abs.join("stderr.txt"),
            &String::from_utf8_lossy(&output.stderr),
        )?;

        let status = if output.status.success() {
            RunStatus::Passed
        } else {
            RunStatus::Failed
        };

        let mut metadata = RunMetadata {
            schema_version: 1,
            run_id: make_run_id("run"),
            backend: self.name().to_string(),
            backend_executable: self.executable.display().to_string(),
            source_netlist: source_abs.display().to_string(),
            working_netlist: working_netlist.display().to_string(),
            output_dir: output_abs.display().to_string(),
            status,
            exit_code: output.status.code(),
            duration_ms,
            started_unix_ms,
            parameters: parameters.to_vec(),
            artifacts: collect_run_artifacts(&output_abs)?,
        };

        metadata
            .artifacts
            .sort_by(|left, right| left.path.cmp(&right.path));
        write_text(&output_abs.join("run.json"), &metadata.to_json())?;

        Ok(metadata)
    }
}

fn copy_relative_dependencies(
    source_netlist: &Path,
    source: &str,
    output_dir: &Path,
) -> OslResult<()> {
    let base_dir = source_netlist.parent().unwrap_or_else(|| Path::new("."));
    for dependency in relative_dependencies(source) {
        let dependency_path = Path::new(&dependency);
        let source_path = base_dir.join(dependency_path);
        if !source_path.is_file() {
            continue;
        }
        let destination = output_dir.join(dependency_path);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)
                .map_err(|err| OslError::io(format!("create {}", parent.display()), err))?;
        }
        fs::copy(&source_path, &destination).map_err(|err| {
            OslError::io(
                format!(
                    "copy {} to {}",
                    source_path.display(),
                    destination.display()
                ),
                err,
            )
        })?;
    }
    Ok(())
}

fn relative_dependencies(source: &str) -> Vec<String> {
    source
        .lines()
        .filter_map(relative_dependency_from_line)
        .collect()
}

fn relative_dependency_from_line(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('*') || trimmed.starts_with(';') {
        return None;
    }
    let tokens = trimmed.split_whitespace().collect::<Vec<_>>();
    if tokens.len() < 2 {
        return None;
    }
    let directive = tokens[0].to_ascii_lowercase();
    if !matches!(directive.as_str(), ".include" | ".inc" | ".lib") {
        return None;
    }
    let path = tokens[1].trim_matches('"').trim_matches('\'');
    if path.is_empty() {
        return None;
    }
    let path = Path::new(path);
    if path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return None;
    }
    Some(path.display().to_string())
}

/// Ensure the netlist contains a `.control` block that writes `waveform.raw`.
///
/// ngspice analysis directives (`.tran`, `.ac`, `.dc`, `.op`) must remain at
/// deck level — **outside** `.control`.  Inside `.control` we only place
/// commands (`run`, `write`, `set`, `print`).
fn ensure_ngspice_control_exports(source: &str) -> String {
    if source.to_ascii_lowercase().contains("waveform.raw") {
        return source.to_string();
    }

    let has_op = source.lines().any(|line| {
        line.trim().eq_ignore_ascii_case(".op")
    });
    let has_tran_ac_dc = source.lines().any(|line| {
        let lower = line.trim().to_ascii_lowercase();
        lower.starts_with(".tran ") || lower.starts_with(".ac ") || lower.starts_with(".dc ")
    });

    let lines = source.lines().collect::<Vec<_>>();
    let mut output = String::new();
    let mut inserted = false;

    // Pass 1: if a `.endc` already exists, inject write directives just before it.
    for line in &lines {
        let trimmed = line.trim().to_ascii_lowercase();
        if !inserted && trimmed == ".endc" {
            if !has_tran_ac_dc && !has_op {
                output.push_str(".tran 1u 10m\n");
                output.push_str("run\n");
            }
            output.push_str("set filetype=binary\n");
            output.push_str("write waveform.raw all\n");
            inserted = true;
        }
        output.push_str(line);
        output.push('\n');
    }

    if inserted {
        return output;
    }

    // Pass 2: no `.control`/`.endc` found — wrap with a new `.control` block.
    // Analysis directives (.tran/.ac/.dc/.op) stay OUTSIDE .control at deck level.
    let mut deck_lines = Vec::new();
    let mut end_line: Option<&str> = None;
    for line in &lines {
        if line.trim().eq_ignore_ascii_case(".end") {
            end_line = Some(line);
        } else {
            deck_lines.push(line);
        }
    }

    let mut result = String::new();

    // Emit deck lines (including any analysis directives at deck level).
    for line in &deck_lines {
        result.push_str(line);
        result.push('\n');
    }

    // Open the .control block — only commands go inside.
    result.push_str(".control\n");

    if !has_tran_ac_dc && !has_op {
        // No analysis directive at all — inject a default transient for useful output.
        result.push_str(".tran 1u 10m\n");
        result.push_str("set filetype=binary\n");
        result.push_str("run\n");
        result.push_str("write waveform.raw all\n");
    } else if has_op && !has_tran_ac_dc {
        result.push_str("run\n");
        result.push_str("print all\n");
    } else {
        result.push_str("set filetype=binary\n");
        result.push_str("run\n");
        result.push_str("write waveform.raw all\n");
    }

    result.push_str(".endc\n");
    result.push_str(end_line.unwrap_or(".end"));
    result.push('\n');
    result
}

fn apply_parameter_overrides(source: &str, parameters: &[ParameterOverride]) -> String {
    if parameters.is_empty() {
        return source.to_string();
    }

    let parameter_names = parameters
        .iter()
        .map(|parameter| parameter.name.to_ascii_lowercase())
        .collect::<Vec<_>>();
    let mut output = String::new();
    output.push_str("* NekoSpice parameter overrides\n");
    for parameter in parameters {
        output.push_str(".param ");
        output.push_str(&parameter.name);
        output.push('=');
        output.push_str(&parameter.value.to_string());
        output.push('\n');
    }
    output.push('\n');

    for line in source.lines() {
        if defines_overridden_parameter(line, &parameter_names) {
            output.push_str("* NekoSpice removed overridden parameter: ");
            output.push_str(line);
            output.push('\n');
        } else {
            output.push_str(line);
            output.push('\n');
        }
    }
    output
}

fn defines_overridden_parameter(line: &str, parameter_names: &[String]) -> bool {
    let trimmed = line.trim();
    if !trimmed.to_ascii_lowercase().starts_with(".param") {
        return false;
    }

    let body = trimmed[6..].trim();
    if body.is_empty() {
        return false;
    }

    for definition in body.split_whitespace() {
        let name = definition
            .split_once('=')
            .map(|(name, _)| name)
            .unwrap_or(definition)
            .trim()
            .to_ascii_lowercase();
        if parameter_names.iter().any(|parameter| parameter == &name) {
            return true;
        }
    }

    false
}

// ---------------------------------------------------------------------------
// Xyce backend
// ---------------------------------------------------------------------------

/// Xyce SPICE simulator CLI backend.
///
/// Xyce uses `.sp` extension and different control block syntax than ngspice.
/// Output waveform format is rawfile ASCII (not binary like ngspice).
#[derive(Debug, Clone)]
pub struct XyceCliBackend {
    executable: PathBuf,
}

impl XyceCliBackend {
    /// new。
    pub fn new(executable: impl Into<PathBuf>) -> Self {
        Self {
            executable: executable.into(),
        }
    }

    /// executable。
    pub fn executable(&self) -> &Path {
        &self.executable
    }
}

impl Default for XyceCliBackend {
    fn default() -> Self {
        Self::new("xyce")
    }
}

impl SimulatorBackend for XyceCliBackend {
    fn name(&self) -> &'static str {
        "xyce-cli"
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            batch: true,
            process_isolated: true,
            writes_binary_waveform: false, // Xyce uses rawfile ASCII
        }
    }

    fn run(&self, source_netlist: &Path, output_dir: &Path) -> OslResult<RunMetadata> {
        self.run_with_parameters(source_netlist, output_dir, &[])
    }

    fn run_with_parameters(
        &self,
        source_netlist: &Path,
        output_dir: &Path,
        parameters: &[ParameterOverride],
    ) -> OslResult<RunMetadata> {
        if !source_netlist.is_file() {
            return Err(OslError::InvalidInput(format!(
                "netlist does not exist: {}",
                source_netlist.display()
            )));
        }

        fs::create_dir_all(output_dir)
            .map_err(|err| OslError::io(format!("create {}", output_dir.display()), err))?;

        let source_abs = fs::canonicalize(source_netlist).map_err(|err| {
            OslError::io(format!("canonicalize {}", source_netlist.display()), err)
        })?;
        let output_abs = fs::canonicalize(output_dir)
            .map_err(|err| OslError::io(format!("canonicalize {}", output_dir.display()), err))?;
        let working_netlist = output_abs.join("input.sp");

        let source = read_text(&source_abs)?;
        let working_source = apply_parameter_overrides(&prepare_xyce_netlist(&source), parameters);
        write_text(&working_netlist, &working_source)?;
        copy_relative_dependencies(&source_abs, &source, &output_abs)?;

        let started_unix_ms = now_unix_ms();
        let timer = Instant::now();
        let output = Command::new(&self.executable)
            .arg("-o")
            .arg("xyce.log")
            .arg("input.sp")
            .current_dir(&output_abs)
            .output()
            .map_err(|err| {
                OslError::io(
                    format!("run {} -o xyce.log input.sp", self.executable.display()),
                    err,
                )
            })?;
        let duration_ms = timer.elapsed().as_millis();

        write_text(
            &output_abs.join("stdout.txt"),
            &String::from_utf8_lossy(&output.stdout),
        )?;
        write_text(
            &output_abs.join("stderr.txt"),
            &String::from_utf8_lossy(&output.stderr),
        )?;

        let status = if output.status.success() {
            RunStatus::Passed
        } else {
            RunStatus::Failed
        };

        let mut metadata = RunMetadata {
            schema_version: 1,
            run_id: make_run_id("run"),
            backend: self.name().to_string(),
            backend_executable: self.executable.display().to_string(),
            source_netlist: source_abs.display().to_string(),
            working_netlist: working_netlist.display().to_string(),
            output_dir: output_abs.display().to_string(),
            status,
            exit_code: output.status.code(),
            duration_ms,
            started_unix_ms,
            parameters: parameters.to_vec(),
            artifacts: collect_run_artifacts(&output_abs)?,
        };

        metadata
            .artifacts
            .sort_by(|left, right| left.path.cmp(&right.path));
        write_text(&output_abs.join("run.json"), &metadata.to_json())?;

        Ok(metadata)
    }
}

/// Extract unique node names from a SPICE netlist for print directives.
///
/// Scans component lines (R, C, L, V, I, D, Q, M, J, B, etc.) and collects
/// all referenced node names, excluding the ground node (0/GND/gnd).
/// Returns up to `max_nodes` unique node names.
fn extract_netlist_nodes(source: &str, max_nodes: usize) -> Vec<String> {
    let ground_names: &[&str] = &["0", "gnd", "GND"];
    let mut nodes = Vec::new();
    let mut seen = std::collections::HashSet::new();

    // SPICE component prefixes: R, C, L, V, I, D, Q, M, J, B, K, F, E, G, H, T, U, W, X
    let component_re = regex::Regex::new(r"(?i)^[rcdvijqbkegfhtuwx]\S+\s+(\S+)\s+(\S+)").unwrap();

    for line in source.lines() {
        let trimmed = line.trim();
        // Skip comments, directives, and empty lines
        if trimmed.is_empty()
            || trimmed.starts_with('*')
            || trimmed.starts_with('.')
            || trimmed.starts_with('#')
        {
            continue;
        }

        if let Some(caps) = component_re.captures(trimmed) {
            for i in 1..=2 {
                if let Some(m) = caps.get(i) {
                    let node = m.as_str().to_string();
                    if !ground_names.contains(&node.as_str()) && seen.insert(node.clone()) {
                        nodes.push(node);
                        if nodes.len() >= max_nodes {
                            return nodes;
                        }
                    }
                }
            }
        }
    }

    // Fallback to "out" if no nodes found
    if nodes.is_empty() {
        nodes.push("out".to_string());
    }
    nodes
}

/// Prepare a SPICE netlist for Xyce simulation.
///
/// Xyce differences from ngspice:
/// - Uses `.print` instead of `.control`/`write` for waveform output
/// - Output files use `.raw` extension (rawfile ASCII format)
/// - `.endc` is not used; control blocks are ngspice-specific
fn prepare_xyce_netlist(source: &str) -> String {
    let mut output = String::new();
    let mut has_print = false;
    let mut has_tran = false;
    let mut has_ac = false;
    let mut has_dc = false;

    for line in source.lines() {
        let trimmed = line.trim();

        // Skip ngspice-specific control blocks
        if trimmed.eq_ignore_ascii_case(".control") || trimmed.eq_ignore_ascii_case(".endc") {
            continue;
        }

        // Detect analysis type and print directives
        let lower = trimmed.to_ascii_lowercase();
        if lower.starts_with(".tran ") {
            has_tran = true;
        } else if lower.starts_with(".ac ") {
            has_ac = true;
        } else if lower.starts_with(".dc ") {
            has_dc = true;
        }
        if lower.starts_with(".print ") {
            has_print = true;
        }

        // Skip ngspice-specific write commands
        if lower.starts_with("write ") || lower == "run" || lower.starts_with("set ") {
            continue;
        }

        output.push_str(line);
        output.push('\n');
    }

    // Add Xyce-compatible print directives if analysis exists but no .print
    if !has_print && (has_tran || has_ac || has_dc) {
        let nodes = extract_netlist_nodes(&output, 8);
        let v_args: String = nodes
            .iter()
            .map(|n| format!("v({})", n))
            .collect::<Vec<_>>()
            .join(" ");

        output.push_str("\n* NekoSpice: Xyce waveform output directives\n");
        if has_tran {
            output.push_str(&format!(".print TRAN {}\n", v_args));
        }
        if has_ac {
            output.push_str(&format!(".print AC {}\n", v_args));
        }
        if has_dc {
            output.push_str(&format!(".print DC {}\n", v_args));
        }
    }

    output
}

/// 为 UI 预览生成 Xyce 格式的网表。
/// 与 prepare_xyce_netlist 相同逻辑，但公开给 GUI 层使用。
pub fn prepare_xyce_netlist_display(source: &str) -> String {
    prepare_xyce_netlist(source)
}

#[cfg(test)]
mod tests {
    use super::{
        apply_parameter_overrides, ensure_ngspice_control_exports, extract_netlist_nodes,
        prepare_xyce_netlist, relative_dependencies,
    };
    use nsp_core::ParameterOverride;

    #[test]
    fn injects_raw_export_before_endc() {
        let source = ".tran 1n 1u\n.control\nrun\n.endc\n.end\n";
        let output = ensure_ngspice_control_exports(source);

        assert!(output.contains("write waveform.raw all\n.endc"));
        assert!(output.contains("set filetype=binary"));
    }

    #[test]
    fn adds_control_block_when_missing() {
        let source = ".tran 1n 1u\n.end\n";
        let output = ensure_ngspice_control_exports(source);

        assert!(
            output.contains(
                ".control\nset filetype=binary\nrun\nwrite waveform.raw all\n.endc\n.end"
            )
        );
    }

    #[test]
    fn prepends_parameter_overrides() {
        let output = apply_parameter_overrides(
            "R1 in out {rload}\n.end\n",
            &[ParameterOverride::new("rload", 2000.0)],
        );

        assert!(output.starts_with("* NekoSpice parameter overrides\n.param rload=2000"));
        assert!(output.contains("R1 in out {rload}"));
    }

    #[test]
    fn removes_overridden_param_definition() {
        let output = apply_parameter_overrides(
            ".param rload=1000\nR1 in out {rload}\n.end\n",
            &[ParameterOverride::new("rload", 2000.0)],
        );

        assert!(output.contains(".param rload=2000"));
        assert!(output.contains("* NekoSpice removed overridden parameter: .param rload=1000"));
        assert!(!output.contains("\n.param rload=1000\n"));
    }

    #[test]
    fn finds_relative_include_dependencies() {
        let dependencies = relative_dependencies(
            r#"
* comment
.include "models/ideal.lib"
.lib vendor/opamp.lib fast
.include /usr/share/global.lib
.include ../outside.lib
"#,
        );

        assert_eq!(
            dependencies,
            vec![
                "models/ideal.lib".to_string(),
                "vendor/opamp.lib".to_string()
            ]
        );
    }

    #[test]
    fn prepare_xyce_netlist_strips_control_blocks() {
        let source =
            ".tran 1n 1u\n.control\nrun\nwrite waveform.raw all\n.endc\nR1 in out 1k\n.end\n";
        let output = prepare_xyce_netlist(source);
        assert!(!output.contains(".control"));
        assert!(!output.contains(".endc"));
        assert!(!output.contains("write waveform.raw"));
        assert!(output.contains("R1 in out 1k"));
        assert!(output.contains(".print TRAN"));
        assert!(output.contains("v(in)"));
        assert!(output.contains("v(out)"));
    }

    #[test]
    fn prepare_xyce_netlist_adds_print_for_tran() {
        let source = ".tran 1n 1u\nR1 in out 1k\n.end\n";
        let output = prepare_xyce_netlist(source);
        assert!(output.contains(".print TRAN"));
        assert!(output.contains("v(in)"));
        assert!(output.contains("v(out)"));
    }

    #[test]
    fn prepare_xyce_netlist_no_control_needed() {
        let source = ".tran 1n 1u\n.print TRAN v(out)\nR1 in out 1k\n.end\n";
        let output = prepare_xyce_netlist(source);
        assert!(output.contains(".print TRAN v(out)"));
        assert_eq!(output.matches(".print TRAN v(out)").count(), 1);
    }

    #[test]
    fn extract_netlist_nodes_from_rc_circuit() {
        let source = "R1 in out 1k\nC1 out 0 1u\nV1 in 0 AC 1\n";
        let nodes = extract_netlist_nodes(source, 10);
        assert!(nodes.contains(&"in".to_string()));
        assert!(nodes.contains(&"out".to_string()));
        assert!(!nodes.contains(&"0".to_string()));
    }

    #[test]
    fn extract_netlist_nodes_fallback() {
        let source = "* just a comment\n.end\n";
        let nodes = extract_netlist_nodes(source, 10);
        assert_eq!(nodes, vec!["out".to_string()]);
    }

    #[test]
    fn extract_netlist_nodes_respects_limit() {
        let source = "R1 a b 1k\nR2 c d 1k\nR3 e f 1k\nR4 g h 1k\n";
        let nodes = extract_netlist_nodes(source, 3);
        assert!(nodes.len() <= 3);
    }
}
