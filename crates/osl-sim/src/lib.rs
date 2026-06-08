use osl_core::{
    Artifact, OslError, OslResult, ParameterOverride, RunMetadata, RunStatus, make_run_id,
    now_unix_ms, read_text, write_text,
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
    pub fn new(executable: impl Into<PathBuf>) -> Self {
        Self {
            executable: executable.into(),
        }
    }

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
            artifacts: collect_artifacts(&output_abs)?,
        };

        metadata
            .artifacts
            .sort_by(|left, right| left.path.cmp(&right.path));
        write_text(&output_abs.join("run.json"), &metadata.to_json())?;

        Ok(metadata)
    }
}

fn collect_artifacts(output_dir: &Path) -> OslResult<Vec<Artifact>> {
    let mut artifacts = Vec::new();
    for entry in fs::read_dir(output_dir)
        .map_err(|err| OslError::io(format!("read {}", output_dir.display()), err))?
    {
        let entry = entry.map_err(|err| OslError::io("read output directory entry", err))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if file_name == "run.json" {
            continue;
        }

        artifacts.push(Artifact {
            path: file_name.to_string(),
            kind: artifact_kind(file_name).to_string(),
        });
    }
    Ok(artifacts)
}

fn artifact_kind(file_name: &str) -> &'static str {
    if file_name.ends_with(".raw") || file_name.ends_with(".csv") {
        "waveform"
    } else if file_name.ends_with(".log") || file_name.ends_with(".txt") {
        "log"
    } else if file_name.ends_with(".cir") || file_name.ends_with(".net") {
        "netlist"
    } else {
        "file"
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

fn ensure_ngspice_control_exports(source: &str) -> String {
    if source.to_ascii_lowercase().contains("waveform.raw") {
        return source.to_string();
    }

    let lines = source.lines().collect::<Vec<_>>();
    let mut output = String::new();
    let mut inserted = false;

    for line in &lines {
        let trimmed = line.trim().to_ascii_lowercase();
        if !inserted && trimmed == ".endc" {
            output.push_str("set filetype=binary\n");
            output.push_str("write waveform.raw all\n");
            inserted = true;
        }
        output.push_str(line);
        output.push('\n');
    }

    if inserted {
        output
    } else {
        let mut without_end = String::new();
        let mut end_line = None::<&str>;
        for line in lines {
            if line.trim().eq_ignore_ascii_case(".end") {
                end_line = Some(line);
            } else {
                without_end.push_str(line);
                without_end.push('\n');
            }
        }

        without_end.push_str(".control\n");
        without_end.push_str("set filetype=binary\n");
        without_end.push_str("run\n");
        without_end.push_str("write waveform.raw all\n");
        without_end.push_str(".endc\n");
        without_end.push_str(end_line.unwrap_or(".end"));
        without_end.push('\n');
        without_end
    }
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

#[cfg(test)]
mod tests {
    use super::{apply_parameter_overrides, ensure_ngspice_control_exports, relative_dependencies};
    use osl_core::ParameterOverride;

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
}
