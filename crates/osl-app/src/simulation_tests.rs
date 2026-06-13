use super::*;
use crate::document::KicadGuiDocument;
use osl_core::{OslError, OslResult};
use osl_sim::{SimulationProfile, SimulatorBackend};
use std::path::Path;

#[cfg(test)]
pub(crate) fn run_document_with_backend(
    document: &KicadGuiDocument,
    runs_root: &Path,
    backend: &dyn SimulatorBackend,
) -> OslResult<GuiSimulationRun> {
    let job = GuiSimulationJob::from_document(document, runs_root, &SimulationProfile::default())
        .map_err(OslError::InvalidInput)?;
    run_job_with_backend(&job, backend)
}

#[cfg(test)]
mod unit_tests {
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
        let job =
            GuiSimulationJob::from_document(&document, &runs_root, &SimulationProfile::default())
                .unwrap();
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
