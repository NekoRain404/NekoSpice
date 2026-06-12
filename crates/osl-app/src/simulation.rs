//! 仿真任务分发器。根据后端选择调度 ngspice/Xyce 仿真任务。
//!
use crate::document::KicadGuiDocument;
use crate::report_summary::{GuiReportSummary, GuiReportSummaryState};
use crate::waveform_summary::GuiWaveformSummaryState;
#[cfg(test)]
use osl_core::OslError;
use osl_core::{OslResult, RunMetadata, make_run_id, write_text};
use osl_sim::{NgspiceCliBackend, XyceCliBackend, SimulatorBackend, SimulationProfile, finalize_run_artifacts, inject_profile_directives, validate_netlist_for_simulation, parse_ngspice_log, format_simulation_log_summary};
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
    #[allow(dead_code)] profile: SimulationProfile,
}

impl GuiSimulationJob {
    /// Create a simulation job from a loaded schematic document and profile.
    ///
    /// The raw netlist from the schematic is combined with the profile's
    /// settings (temperature, tolerances, method, parameter overrides)
    /// to produce a complete, runnable SPICE netlist.
    pub(crate) fn from_document(
        document: &KicadGuiDocument,
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
    pub(crate) fn spawn_ngspice(job: GuiSimulationJob) -> Self {
        Self::spawn(job, || Box::new(NgspiceCliBackend::default()))
    }

    /// spawn xyce。
    pub(crate) fn spawn_xyce(job: GuiSimulationJob) -> Self {
        Self::spawn(job, || Box::new(XyceCliBackend::default()))
    }

    /// spawn with backend。
    pub(crate) fn spawn_with_backend(job: GuiSimulationJob, backend: &str) -> Self {
        match backend {
            "xyce" => Self::spawn_xyce(job),
            _ => Self::spawn_ngspice(job),
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

#[cfg(test)]
pub(crate) fn run_document_with_backend(
    document: &KicadGuiDocument,
    runs_root: &Path,
    backend: &dyn SimulatorBackend,
) -> OslResult<GuiSimulationRun> {
    let job = GuiSimulationJob::from_document(
        document,
        runs_root,
        &SimulationProfile::default(),
    )
    .map_err(OslError::InvalidInput)?;
    run_job_with_backend(&job, backend)
}

fn run_job_with_backend(
    job: &GuiSimulationJob,
    backend: &dyn SimulatorBackend,
) -> OslResult<GuiSimulationRun> {
    let output_dir = job.runs_root.join(run_directory_name(&job.schematic_path));
    let source_netlist = output_dir.join("schematic.cir");
    write_text(&source_netlist, &job.netlist)?;
    let mut metadata = backend.run(&source_netlist, &output_dir)?;

    // If simulation failed, parse the log file for meaningful error messages.
    // Check both ngspice.log and xyce.log since we don't know which backend ran.
    if metadata.status == osl_core::RunStatus::Failed {
        let log_path = if output_dir.join("xyce.log").is_file() {
            output_dir.join("xyce.log")
        } else {
            output_dir.join("ngspice.log")
        };
        if let Ok(log_content) = std::fs::read_to_string(&log_path) {
            let (errors, warnings, summary) = parse_ngspice_log(&log_content);
            if !errors.is_empty() || !warnings.is_empty() {
                let log_summary = format_simulation_log_summary(&errors, &warnings, summary.as_deref());
                write_text(
                    &output_dir.join("simulation-error.txt"),
                    &log_summary,
                )?;
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
mod tests {
    use super::*;
    use osl_core::{Artifact, ParameterOverride, RunStatus};
    use std::fs;
    use std::time::{Duration, Instant};

    #[derive(Debug)]
    struct RecordingBackend;

    const SAMPLE_RAW: &str = r#"
Title: gui run
Plotname: Transient Analysis
Flags: real
No. Variables: 2
No. Points: 2
Variables:
	0	time	time
	1	v(out)	voltage
Values:
 0	0.000000000000000e+00
	1.000000000000000e+00

 1	1.000000000000000e-06
	3.000000000000000e+00
"#;

    impl SimulatorBackend for RecordingBackend {
        fn name(&self) -> &'static str {
            "recording"
        }

        fn capabilities(&self) -> osl_sim::BackendCapabilities {
            osl_sim::BackendCapabilities {
                batch: true,
                process_isolated: true,
                writes_binary_waveform: false,
            }
        }

        fn run(&self, source_netlist: &Path, output_dir: &Path) -> OslResult<RunMetadata> {
            self.run_with_parameters(source_netlist, output_dir, &[])
        }

        fn run_with_parameters(
            &self,
            source_netlist: &Path,
            output_dir: &Path,
            _parameters: &[ParameterOverride],
        ) -> OslResult<RunMetadata> {
            assert!(source_netlist.is_file());
            assert!(output_dir.is_dir());
            osl_core::write_text(&output_dir.join("waveform.raw"), SAMPLE_RAW)?;
            Ok(RunMetadata {
                schema_version: 1,
                run_id: "recorded".to_string(),
                backend: self.name().to_string(),
                backend_executable: "recording".to_string(),
                source_netlist: source_netlist.display().to_string(),
                working_netlist: source_netlist.display().to_string(),
                output_dir: output_dir.display().to_string(),
                status: RunStatus::Passed,
                exit_code: Some(0),
                duration_ms: 0,
                started_unix_ms: 0,
                parameters: Vec::new(),
                artifacts: vec![
                    Artifact {
                        path: "schematic.cir".to_string(),
                        kind: "netlist".to_string(),
                    },
                    Artifact {
                        path: "waveform.raw".to_string(),
                        kind: "waveform".to_string(),
                    },
                ],
            })
        }
    }

    #[test]
    fn writes_document_netlist_and_runs_backend() {
        let temp = crate::test_support::temp_schematic_copy("gui_run");
        let document = KicadGuiDocument::load(temp.path().to_path_buf()).unwrap();
        let runs_root =
            std::env::temp_dir().join(format!("nekospice_gui_run_{}", osl_core::now_unix_ms()));

        let run = run_document_with_backend(&document, &runs_root, &RecordingBackend).unwrap();

        let netlist_path = run.output_dir.join("schematic.cir");
        let netlist = fs::read_to_string(netlist_path).unwrap();
        assert!(netlist.contains(".tran 1u 1m"));
        assert_eq!(run.metadata.backend, "recording");
        let GuiWaveformSummaryState::Ready(summary) = run.waveform else {
            panic!("expected waveform summary");
        };
        assert_eq!(summary.point_count, 2);
        assert_eq!(summary.variables[1].name, "v(out)");
        assert!(run.output_dir.join("waveform.csv").is_file());
        assert!(run.output_dir.join("waveform-summary.json").is_file());
        assert!(run.output_dir.join("report.html").is_file());
        assert!(run.output_dir.join("run.json").is_file());
        let GuiReportSummaryState::Ready(report) = run.report else {
            panic!("expected report summary");
        };
        assert_eq!(report.report_file, "report.html");
        assert_eq!(report.source_file.as_deref(), Some("run.json"));
        assert!(report.reused_existing_html);
        assert!(
            run.metadata
                .artifacts
                .iter()
                .any(|artifact| artifact.path == "waveform-summary.json")
        );
        assert!(
            run.metadata
                .artifacts
                .iter()
                .any(|artifact| artifact.path == "report.html" && artifact.kind == "report")
        );
        let _ = fs::remove_dir_all(runs_root);
    }

    #[test]
    fn background_task_returns_backend_result() {
        let temp = crate::test_support::temp_schematic_copy("gui_background_run");
        let document = KicadGuiDocument::load(temp.path().to_path_buf()).unwrap();
        let runs_root =
            std::env::temp_dir().join(format!("nekospice_gui_task_{}", osl_core::now_unix_ms()));
        let job = GuiSimulationJob::from_document(&document, &runs_root, &SimulationProfile::default()).unwrap();
        let task = GuiSimulationTask::spawn(job, || Box::new(RecordingBackend));

        let started = Instant::now();
        let run = loop {
            if let Some(result) = task.try_finish() {
                break result.unwrap();
            }
            assert!(started.elapsed() < Duration::from_secs(2));
            thread::sleep(Duration::from_millis(5));
        };

        assert_eq!(run.metadata.backend, "recording");
        assert!(run.output_dir.join("schematic.cir").is_file());
        assert!(matches!(run.waveform, GuiWaveformSummaryState::Ready(_)));
        assert!(run.output_dir.join("waveform.csv").is_file());
        assert!(run.output_dir.join("report.html").is_file());
        let _ = fs::remove_dir_all(runs_root);
    }
}
