use osl_core::{
    Artifact, OslError, OslResult, RunMetadata, RunStatus, make_run_id, now_unix_ms, write_text,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct BackendCapabilities {
    pub batch: bool,
    pub process_isolated: bool,
    pub writes_ascii_waveform: bool,
}

pub trait SimulatorBackend {
    fn name(&self) -> &'static str;
    fn capabilities(&self) -> BackendCapabilities;
    fn run(&self, source_netlist: &Path, output_dir: &Path) -> OslResult<RunMetadata>;
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
            writes_ascii_waveform: true,
        }
    }

    fn run(&self, source_netlist: &Path, output_dir: &Path) -> OslResult<RunMetadata> {
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

        fs::copy(&source_abs, &working_netlist).map_err(|err| {
            OslError::io(
                format!(
                    "copy {} to {}",
                    source_abs.display(),
                    working_netlist.display()
                ),
                err,
            )
        })?;

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
    if file_name.ends_with(".csv") {
        "waveform"
    } else if file_name.ends_with(".log") || file_name.ends_with(".txt") {
        "log"
    } else if file_name.ends_with(".cir") || file_name.ends_with(".net") {
        "netlist"
    } else {
        "file"
    }
}
