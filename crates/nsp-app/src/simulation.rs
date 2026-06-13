//! 仿真任务分发器。根据后端选择调度 ngspice/Xyce 仿真任务。
//!
use crate::document::NspGuiDocument;
use crate::report_summary::{GuiReportSummary, GuiReportSummaryState};
use crate::waveform_summary::GuiWaveformSummaryState;
use nsp_core::{OslResult, RunMetadata, make_run_id, write_text};
use nsp_sim::{
    NgspiceCliBackend, SimulationProfile, SimulatorBackend, XyceCliBackend, finalize_run_artifacts,
    format_simulation_log_summary, inject_profile_directives, parse_ngspice_log,
    validate_netlist_for_simulation,
};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::thread;

#[derive(Debug, Clone)]
pub(crate) struct GuiSimulationRun {
    pub(crate) output_dir: PathBuf,
    pub(crate) metadata: RunMetadata,
    pub(crate) report: GuiReportSummaryState,
    pub(crate) waveform: GuiWaveformSummaryState,
}

impl GuiSimulationRun {
    /// from output dir。
    pub(crate) fn from_output_dir(output_dir: PathBuf) -> Result<Self, String> {
        crate::simulation_run_loader::load_gui_simulation_run(output_dir)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct GuiSimulationJob {
    schematic_path: PathBuf,
    netlist: String,
    runs_root: PathBuf,
    /// Simulation profile carrying user-configured settings (temperature,
    /// tolerances, method, component/model parameter overrides).
    #[allow(dead_code)]
    profile: SimulationProfile,
}

impl GuiSimulationJob {
    /// Create a simulation job from a loaded schematic document and profile.
    ///
    /// The raw netlist from the schematic is combined with the profile's
    /// settings (temperature, tolerances, method, parameter overrides)
    /// to produce a complete, runnable SPICE netlist.
    pub(crate) fn from_document(
        document: &NspGuiDocument,
        runs_root: &Path,
        profile: &SimulationProfile,
    ) -> Result<Self, String> {
        let raw_netlist = document.spice_netlist_preview()?;
        // Inject profile directives into the netlist
        let netlist = inject_profile_directives(&raw_netlist, profile);
        Ok(Self {
            schematic_path: document.path().to_path_buf(),
            netlist,
            runs_root: runs_root.to_path_buf(),
            profile: profile.clone(),
        })
    }

    /// Validate the netlist before running simulation.
    /// Returns warnings/errors that should be shown to the user.
    pub(crate) fn validate(&self) -> Vec<String> {
        validate_netlist_for_simulation(&self.netlist)
    }
}

#[derive(Debug)]
pub(crate) struct GuiSimulationTask {
    receiver: Receiver<Result<GuiSimulationRun, String>>,
}

impl GuiSimulationTask {
    /// spawn ngspice。
    pub(crate) fn spawn_ngspice(job: GuiSimulationJob, executable: &str) -> Self {
        let exec = executable.to_string();
        Self::spawn(job, move || Box::new(NgspiceCliBackend::new(exec)))
    }

    /// spawn xyce。
    pub(crate) fn spawn_xyce(job: GuiSimulationJob, executable: &str) -> Self {
        let exec = executable.to_string();
        Self::spawn(job, move || Box::new(XyceCliBackend::new(exec)))
    }

    /// spawn with backend。
    pub(crate) fn spawn_with_backend(
        job: GuiSimulationJob,
        backend: &str,
        ngspice: &str,
        xyce: &str,
    ) -> Self {
        match backend {
            "xyce" => Self::spawn_xyce(job, xyce),
            _ => Self::spawn_ngspice(job, ngspice),
        }
    }

    /// try finish。
    pub(crate) fn try_finish(&self) -> Option<Result<GuiSimulationRun, String>> {
        match self.receiver.try_recv() {
            Ok(result) => Some(result),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => Some(Err(
                "simulation worker disconnected before returning a result".to_string(),
            )),
        }
    }

    fn spawn(
        job: GuiSimulationJob,
        backend_factory: impl FnOnce() -> Box<dyn SimulatorBackend + Send> + Send + 'static,
    ) -> Self {
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let backend = backend_factory();
            let result =
                run_job_with_backend(&job, backend.as_ref()).map_err(|error| error.to_string());
            let _ = sender.send(result);
        });
        Self { receiver }
    }
}

/// Normalize SPICE model types in a netlist string.
///
/// Maps vendor-specific model type names (LPNP, LNPN, etc.) to standard
/// ngspice-compatible types (PNP, NPN, etc.).
fn normalize_model_types(netlist: &str) -> String {
    netlist
        .lines()
        .map(|line| {
            let t = line.trim();
            if t.to_ascii_lowercase().starts_with(".model ") {
                let parts: Vec<&str> = t.split_whitespace().collect();
                if parts.len() >= 3 {
                    let normalized = match parts[2].to_ascii_uppercase().as_str() {
                        "LPNP" => "PNP",
                        "LNPN" => "NPN",
                        "LPMOS" => "PMOS",
                        "LNMOS" => "NMOS",
                        _ => parts[2],
                    };
                    if normalized != parts[2] {
                        let prefix_len = line.len() - line.trim_start().len();
                        let prefix = &line[..prefix_len];
                        let after_type =
                            line[line.find(parts[2]).unwrap() + parts[2].len()..].to_string();
                        return format!("{prefix}.model {} {}{}", parts[1], normalized, after_type);
                    }
                }
            }
            line.to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Resolve relative .include/.lib paths in a netlist to absolute paths.
fn resolve_include_paths(netlist: &str, base_dir: &std::path::Path) -> String {
    netlist
        .lines()
        .map(|line| {
            let trimmed = line.trim();
            if (trimmed.starts_with(".include ") || trimmed.starts_with(".lib "))
                && !trimmed.contains("$")
            {
                let directive_end = trimmed.find(char::is_whitespace).unwrap_or(trimmed.len());
                let path_part = trimmed[directive_end..].trim();
                let unquoted = path_part.trim_matches('"').trim_matches('\'');
                if !unquoted.is_empty()
                    && !std::path::Path::new(unquoted).is_absolute()
                    && !unquoted.contains("$")
                {
                    let absolute = base_dir.join(unquoted);
                    if absolute.exists() {
                        let abs_str = absolute.to_string_lossy().to_string();
                        let prefix = &trimmed[..directive_end];
                        let rest = &trimmed[directive_end..];
                        let resolved = rest.replace(unquoted, &abs_str);
                        return format!("{prefix}{resolved}");
                    }
                }
            }
            line.to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn run_job_with_backend(
    job: &GuiSimulationJob,
    backend: &dyn SimulatorBackend,
) -> OslResult<GuiSimulationRun> {
    let output_dir = job.runs_root.join(run_directory_name(&job.schematic_path));
    let source_netlist = output_dir.join("schematic.cir");
    let normalized_netlist = normalize_model_types(&job.netlist);
    let schematic_dir = job
        .schematic_path
        .parent()
        .unwrap_or(std::path::Path::new("."));
    let resolved_netlist = resolve_include_paths(&normalized_netlist, schematic_dir);
    write_text(&source_netlist, &resolved_netlist)?;
    let mut metadata = backend.run(&source_netlist, &output_dir)?;

    // If simulation failed, parse the log file for meaningful error messages.
    // Check both ngspice.log and xyce.log since we don't know which backend ran.
    if metadata.status == nsp_core::RunStatus::Failed {
        let log_path = if output_dir.join("xyce.log").is_file() {
            output_dir.join("xyce.log")
        } else {
            output_dir.join("ngspice.log")
        };
        if let Ok(log_content) = std::fs::read_to_string(&log_path) {
            let (errors, warnings, summary) = parse_ngspice_log(&log_content);
            if !errors.is_empty() || !warnings.is_empty() {
                let log_summary =
                    format_simulation_log_summary(&errors, &warnings, summary.as_deref());
                write_text(&output_dir.join("simulation-error.txt"), &log_summary)?;
            }
        }
    }

    finalize_run_artifacts(&output_dir, &mut metadata)?;
    let report = GuiReportSummary::from_report_dir(&output_dir);
    let waveform = GuiWaveformSummaryState::from_run_dir(&output_dir);
    Ok(GuiSimulationRun {
        output_dir,
        metadata,
        report,
        waveform,
    })
}

fn run_directory_name(schematic_path: &Path) -> String {
    let stem = schematic_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(sanitize_run_name)
        .filter(|stem| !stem.is_empty())
        .unwrap_or_else(|| "schematic".to_string());
    format!("{}-{}", stem, make_run_id("gui"))
}

fn sanitize_run_name(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '_' || character == '-' {
                character
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}

#[cfg(test)]
#[path = "simulation_tests.rs"]
mod tests;
