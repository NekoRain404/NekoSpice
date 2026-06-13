//! Simulation artifact management - collects output files, logs, waveforms, and reports from runs.


use osl_core::{Artifact, OslError, OslResult, RunMetadata, RunStatus, html_escape, write_text};
use osl_waveform::read_ngspice_raw;
use std::fs;
use std::path::Path;

/// finalize run artifacts。
pub fn finalize_run_artifacts(output_dir: &Path, metadata: &mut RunMetadata) -> OslResult<()> {
    if metadata.status == RunStatus::Passed {
        export_waveform_artifacts(output_dir)?;
    }
    refresh_run_artifacts(output_dir, metadata)?;
    write_run_report(output_dir, metadata)?;
    refresh_run_artifacts(output_dir, metadata)?;
    write_text(&output_dir.join("run.json"), &metadata.to_json())
}

/// export waveform artifacts。
pub fn export_waveform_artifacts(output_dir: &Path) -> OslResult<()> {
    let raw_path = output_dir.join("waveform.raw");
    if !raw_path.is_file() {
        return Ok(());
    }

    let waveform = read_ngspice_raw(&raw_path)?;
    write_text(&output_dir.join("waveform.csv"), &waveform.to_csv()?)?;
    write_text(
        &output_dir.join("waveform-summary.json"),
        &waveform.to_summary_json()?,
    )
}

/// write run report。
pub fn write_run_report(output_dir: &Path, metadata: &RunMetadata) -> OslResult<()> {
    write_text(&output_dir.join("report.html"), &run_report_html(metadata))
}

/// run report html。
pub fn run_report_html(metadata: &RunMetadata) -> String {
    let artifact_items = metadata
        .artifacts
        .iter()
        .map(|artifact| {
            format!(
                "<li><a href=\"{}\"><code>{}</code></a> <span>{}</span></li>",
                html_escape(&artifact.path),
                html_escape(&artifact.path),
                html_escape(&artifact.kind)
            )
        })
        .collect::<String>();
    let parameters_section = if metadata.parameters.is_empty() {
        String::new()
    } else {
        let rows = metadata
            .parameters
            .iter()
            .map(|parameter| {
                format!(
                    "<tr><td>{}</td><td>{}</td></tr>",
                    html_escape(&parameter.name),
                    parameter.value
                )
            })
            .collect::<String>();
        format!(
            concat!(
                "<h2>Parameters</h2>",
                "<table><thead><tr><th>Name</th><th>Value</th></tr></thead>",
                "<tbody>{}</tbody></table>"
            ),
            rows
        )
    };

    format!(
        concat!(
            "<!doctype html><html><head><meta charset=\"utf-8\">",
            "<title>NekoSpice Run Report</title>{}</head><body>",
            "<main><h1>{}</h1>",
            "<section class=\"summary {}\"><strong>Status:</strong> {} <strong>Duration:</strong> {} ms</section>",
            "<dl><dt>Backend</dt><dd>{}</dd><dt>Executable</dt><dd>{}</dd><dt>Netlist</dt><dd>{}</dd><dt>Output</dt><dd>{}</dd></dl>",
            "{}",
            "<h2>Artifacts</h2><ul>{}</ul>",
            "</main></body></html>\n"
        ),
        run_report_css(),
        html_escape(&metadata.run_id),
        html_escape(metadata.status.as_str()),
        html_escape(metadata.status.as_str()),
        metadata.duration_ms,
        html_escape(&metadata.backend),
        html_escape(&metadata.backend_executable),
        html_escape(&metadata.source_netlist),
        html_escape(&metadata.output_dir),
        parameters_section,
        artifact_items
    )
}

/// refresh run artifacts。
pub fn refresh_run_artifacts(output_dir: &Path, metadata: &mut RunMetadata) -> OslResult<()> {
    metadata.artifacts = collect_run_artifacts(output_dir)?;
    metadata
        .artifacts
        .sort_by(|left, right| left.path.cmp(&right.path));
    Ok(())
}

/// collect run artifacts。
pub fn collect_run_artifacts(output_dir: &Path) -> OslResult<Vec<Artifact>> {
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
            kind: run_artifact_kind(file_name).to_string(),
        });
    }
    Ok(artifacts)
}

/// run artifact kind。
pub fn run_artifact_kind(file_name: &str) -> &'static str {
    if file_name == "waveform-summary.json"
        || file_name.ends_with(".raw")
        || file_name.ends_with(".csv")
    {
        "waveform"
    } else if file_name.ends_with(".log") || file_name.ends_with(".txt") {
        "log"
    } else if file_name.ends_with(".cir") || file_name.ends_with(".net") {
        "netlist"
    } else if file_name.ends_with(".html") {
        "report"
    } else {
        "file"
    }
}

fn run_report_css() -> &'static str {
    concat!(
        "<style>",
        "body{margin:0;background:#f7f7f4;color:#1f2933;font:14px/1.5 system-ui,-apple-system,BlinkMacSystemFont,sans-serif}",
        "main{max-width:1100px;margin:0 auto;padding:32px}",
        "h1{font-size:28px;margin:0 0 20px}",
        "h2{font-size:18px;margin-top:24px}",
        ".summary{background:#fff;border:1px solid #d9ded7;border-radius:6px;padding:12px 14px;margin-bottom:16px}",
        ".summary.passed{border-left:5px solid #0f766e}",
        ".summary.failed{border-left:5px solid #b91c1c}",
        "table{width:100%;border-collapse:collapse;background:#fff;border:1px solid #d9ded7}",
        "th,td{padding:10px 12px;border-bottom:1px solid #e5e7eb;text-align:left;vertical-align:top}",
        "th{background:#ecefeb;font-weight:700}",
        "code,pre{font-family:ui-monospace,SFMono-Regular,Consolas,monospace}",
        "dl{display:grid;grid-template-columns:120px 1fr;gap:8px 12px;background:#fff;border:1px solid #d9ded7;border-radius:6px;padding:14px}",
        "dt{font-weight:700}",
        "ul{background:#fff;border:1px solid #d9ded7;border-radius:6px;padding:14px 14px 14px 30px}",
        "</style>"
    )
}

#[cfg(test)]
mod tests {
    use super::{finalize_run_artifacts, run_artifact_kind, run_report_html};
    use osl_core::{ParameterOverride, RunMetadata, RunStatus, write_text};
    use std::fs;

    const SAMPLE_RAW: &str = r#"
Title: sim artifact demo
Plotname: Transient Analysis
Flags: real
No. Variables: 2
No. Points: 2
Variables:
	0	time	time
	1	v(out)	voltage
Values:
 0	0.000000000000000e+00
	2.000000000000000e+00

 1	1.000000000000000e-06
	4.000000000000000e+00
"#;

    #[test]
    fn classifies_run_artifacts() {
        assert_eq!(run_artifact_kind("waveform-summary.json"), "waveform");
        assert_eq!(run_artifact_kind("waveform.raw"), "waveform");
        assert_eq!(run_artifact_kind("waveform.csv"), "waveform");
        assert_eq!(run_artifact_kind("ngspice.log"), "log");
        assert_eq!(run_artifact_kind("input.cir"), "netlist");
        assert_eq!(run_artifact_kind("report.html"), "report");
    }

    #[test]
    fn writes_single_run_report_html() {
        let metadata = test_metadata(std::path::Path::new("/tmp/nekospice-report"));
        let html = run_report_html(&metadata);

        assert!(html.contains("NekoSpice Run Report"));
        assert!(html.contains("Status:"));
        assert!(html.contains("rload"));
        assert!(html.contains("waveform.raw"));
    }

    #[test]
    fn finalizes_waveform_run_artifacts() {
        let output_dir = std::env::temp_dir().join(format!(
            "nekospice_sim_artifacts_{}_{}",
            std::process::id(),
            osl_core::now_unix_ms()
        ));
        write_text(&output_dir.join("waveform.raw"), SAMPLE_RAW).unwrap();
        write_text(&output_dir.join("input.cir"), ".tran 1u 1m\n.end\n").unwrap();
        let mut metadata = test_metadata(&output_dir);

        finalize_run_artifacts(&output_dir, &mut metadata).unwrap();

        assert!(output_dir.join("waveform.csv").is_file());
        assert!(output_dir.join("waveform-summary.json").is_file());
        assert!(output_dir.join("report.html").is_file());
        assert!(output_dir.join("run.json").is_file());
        assert!(metadata.artifacts.iter().any(
            |artifact| artifact.path == "waveform-summary.json" && artifact.kind == "waveform"
        ));
        assert!(
            metadata
                .artifacts
                .iter()
                .any(|artifact| artifact.path == "report.html" && artifact.kind == "report")
        );
        let _ = fs::remove_dir_all(output_dir);
    }

    fn test_metadata(output_dir: &std::path::Path) -> RunMetadata {
        RunMetadata {
            schema_version: 1,
            run_id: "test".to_string(),
            backend: "test".to_string(),
            backend_executable: "test".to_string(),
            source_netlist: "source.cir".to_string(),
            working_netlist: output_dir.join("input.cir").display().to_string(),
            output_dir: output_dir.display().to_string(),
            status: RunStatus::Passed,
            exit_code: Some(0),
            duration_ms: 0,
            started_unix_ms: 0,
            parameters: vec![ParameterOverride::new("rload", 1000.0)],
            artifacts: vec![osl_core::Artifact {
                path: "waveform.raw".to_string(),
                kind: "waveform".to_string(),
            }],
        }
    }
}
