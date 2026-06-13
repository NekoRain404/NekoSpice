//! 仿真运行结果加载器。解析 ngspice raw 文件和 Xyce 输出。
//!
use crate::report_summary::GuiReportSummary;
use crate::simulation::GuiSimulationRun;
use crate::waveform_summary::GuiWaveformSummaryState;
use nsp_core::{Artifact, RunMetadata, RunStatus, read_text};
use std::path::{Path, PathBuf};

/// load gui simulation run。
pub(crate) fn load_gui_simulation_run(output_dir: PathBuf) -> Result<GuiSimulationRun, String> {
    let metadata = read_run_metadata(&output_dir.join("run.json"))?;
    let report = GuiReportSummary::from_report_dir(&output_dir);
    let waveform = GuiWaveformSummaryState::from_run_dir(&output_dir);
    Ok(GuiSimulationRun {
        output_dir,
        metadata,
        report,
        waveform,
    })
}

fn read_run_metadata(path: &Path) -> Result<RunMetadata, String> {
    let json = read_text(path).map_err(|error| error.to_string())?;
    Ok(RunMetadata {
        schema_version: json_u32(&json, "schema_version")?,
        run_id: json_string(&json, "run_id")?,
        backend: json_string(&json, "backend")?,
        backend_executable: json_string(&json, "backend_executable")?,
        source_netlist: json_string(&json, "source_netlist")?,
        working_netlist: json_string(&json, "working_netlist")?,
        output_dir: json_string(&json, "output_dir")?,
        status: json_run_status(&json)?,
        exit_code: json_i32_option(&json, "exit_code")?,
        duration_ms: json_u128(&json, "duration_ms")?,
        started_unix_ms: json_u128(&json, "started_unix_ms")?,
        parameters: Vec::new(),
        artifacts: json_artifacts(&json),
    })
}

fn json_run_status(json: &str) -> Result<RunStatus, String> {
    match json_string(json, "status")?.as_str() {
        "passed" => Ok(RunStatus::Passed),
        "failed" => Ok(RunStatus::Failed),
        status => Err(format!("unsupported run status {status:?}")),
    }
}

fn json_string(json: &str, key: &str) -> Result<String, String> {
    let marker = format!("\"{key}\":");
    let start = json
        .find(&marker)
        .ok_or_else(|| format!("run metadata missing {key}"))?
        + marker.len();
    let value = json[start..].trim_start();
    let quoted = value
        .strip_prefix('"')
        .ok_or_else(|| format!("run metadata field {key} is not a string"))?;
    let end = quoted
        .find('"')
        .ok_or_else(|| format!("run metadata field {key} is unterminated"))?;
    Ok(quoted[..end].replace("\\\"", "\"").replace("\\\\", "\\"))
}

fn json_u32(json: &str, key: &str) -> Result<u32, String> {
    json_number_text(json, key)?
        .parse::<u32>()
        .map_err(|error| format!("invalid {key}: {error}"))
}

fn json_u128(json: &str, key: &str) -> Result<u128, String> {
    json_number_text(json, key)?
        .parse::<u128>()
        .map_err(|error| format!("invalid {key}: {error}"))
}

fn json_i32_option(json: &str, key: &str) -> Result<Option<i32>, String> {
    let value = json_number_text(json, key)?;
    if value == "null" {
        return Ok(None);
    }
    value
        .parse::<i32>()
        .map(Some)
        .map_err(|error| format!("invalid {key}: {error}"))
}

fn json_number_text<'a>(json: &'a str, key: &str) -> Result<&'a str, String> {
    let marker = format!("\"{key}\":");
    let start = json
        .find(&marker)
        .ok_or_else(|| format!("run metadata missing {key}"))?
        + marker.len();
    Ok(json[start..]
        .trim_start()
        .split([',', '\n'])
        .next()
        .unwrap_or_default()
        .trim())
}

fn json_artifacts(json: &str) -> Vec<Artifact> {
    let Some(start) = json.find("\"artifacts\":") else {
        return Vec::new();
    };
    json[start..]
        .lines()
        .filter(|line| line.contains("\"path\"") && line.contains("\"kind\""))
        .filter_map(|line| {
            Some(Artifact {
                path: json_string(line, "path").ok()?,
                kind: json_string(line, "kind").ok()?,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::load_gui_simulation_run;
    use crate::waveform_summary::GuiWaveformSummaryState;
    use nsp_core::{Artifact, RunMetadata, RunStatus};
    use std::fs;

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

    #[test]
    fn loads_existing_run_directory_for_gui_smoke() {
        let runs_root = std::env::temp_dir().join(format!(
            "nekospice_gui_existing_run_{}",
            nsp_core::now_unix_ms()
        ));
        let output_dir = runs_root.join("recorded");
        fs::create_dir_all(&output_dir).unwrap();
        let mut metadata = RunMetadata {
            schema_version: 1,
            run_id: "recorded".to_string(),
            backend: "recording".to_string(),
            backend_executable: "recording".to_string(),
            source_netlist: "schematic.cir".to_string(),
            working_netlist: "input.cir".to_string(),
            output_dir: output_dir.display().to_string(),
            status: RunStatus::Passed,
            exit_code: Some(0),
            duration_ms: 7,
            started_unix_ms: 10,
            parameters: Vec::new(),
            artifacts: vec![Artifact {
                path: "waveform.raw".to_string(),
                kind: "waveform".to_string(),
            }],
        };
        nsp_core::write_text(&output_dir.join("waveform.raw"), SAMPLE_RAW).unwrap();
        nsp_sim::finalize_run_artifacts(&output_dir, &mut metadata).unwrap();
        metadata.duration_ms = 7;
        nsp_core::write_text(&output_dir.join("run.json"), &metadata.to_json()).unwrap();

        let run = load_gui_simulation_run(output_dir.clone()).unwrap();

        assert_eq!(run.metadata.run_id, "recorded");
        assert_eq!(run.metadata.duration_ms, 7);
        assert!(
            run.metadata
                .artifacts
                .iter()
                .any(|artifact| { artifact.path == "waveform.raw" && artifact.kind == "waveform" })
        );
        assert!(matches!(run.waveform, GuiWaveformSummaryState::Ready(_)));

        let _ = fs::remove_dir_all(runs_root);
    }
}
