use crate::document::KicadGuiDocument;
use osl_core::{OslError, OslResult, RunMetadata, make_run_id, write_text};
use osl_sim::{NgspiceCliBackend, SimulatorBackend};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub(crate) struct GuiSimulationRun {
    pub(crate) output_dir: PathBuf,
    pub(crate) metadata: RunMetadata,
}

pub(crate) fn run_document_with_ngspice(
    document: &KicadGuiDocument,
    runs_root: &Path,
) -> Result<GuiSimulationRun, String> {
    run_document_with_backend(document, runs_root, &NgspiceCliBackend::default())
        .map_err(|error| error.to_string())
}

pub(crate) fn run_document_with_backend(
    document: &KicadGuiDocument,
    runs_root: &Path,
    backend: &dyn SimulatorBackend,
) -> OslResult<GuiSimulationRun> {
    let netlist = document
        .spice_netlist_preview()
        .map_err(OslError::InvalidInput)?;
    let output_dir = runs_root.join(run_directory_name(document.path()));
    let source_netlist = output_dir.join("schematic.cir");
    write_text(&source_netlist, &netlist)?;
    let metadata = backend.run(&source_netlist, &output_dir)?;
    Ok(GuiSimulationRun {
        output_dir,
        metadata,
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

    #[derive(Debug)]
    struct RecordingBackend;

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
                artifacts: vec![Artifact {
                    path: "schematic.cir".to_string(),
                    kind: "netlist".to_string(),
                }],
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
        let _ = fs::remove_dir_all(runs_root);
    }
}
