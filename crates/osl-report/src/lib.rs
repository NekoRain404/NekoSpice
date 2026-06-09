mod format;
mod html;
mod json;
mod junit;
mod markdown;

pub use html::report_css;

use osl_core::{ParameterOverride, RunMetadata, RunStatus};
use osl_waveform::WaveformSummary;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct VerifyRunResult {
    pub index: usize,
    pub name: String,
    pub netlist: String,
    pub run_dir: String,
    pub metadata: RunMetadata,
    pub parameters: Vec<ParameterOverride>,
    pub checks: Vec<CheckResult>,
}

impl VerifyRunResult {
    pub fn status(&self) -> RunStatus {
        if self.metadata.status == RunStatus::Passed && self.checks.iter().all(|check| check.passed)
        {
            RunStatus::Passed
        } else {
            RunStatus::Failed
        }
    }

    pub fn failed_checks(&self) -> impl Iterator<Item = &CheckResult> {
        self.checks.iter().filter(|check| !check.passed)
    }

    fn run_dir_name(&self) -> String {
        Path::new(&self.run_dir)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(&self.name)
            .to_string()
    }

    pub(crate) fn artifact_href(&self, artifact: &str) -> String {
        let run_dir = Path::new(&self.run_dir);
        let run_dir_name = self.run_dir_name();
        if run_dir
            .parent()
            .and_then(|parent| parent.file_name())
            .and_then(|name| name.to_str())
            == Some("runs")
        {
            format!("runs/{run_dir_name}/{artifact}")
        } else {
            format!("{run_dir_name}/{artifact}")
        }
    }
}

#[derive(Debug, Clone)]
pub struct CheckResult {
    pub name: String,
    pub kind: String,
    pub signal: String,
    pub from: Option<f64>,
    pub to: Option<f64>,
    pub value: Option<f64>,
    pub summary: Option<WaveformSummary>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub passed: bool,
    pub message: String,
}

impl CheckResult {
    pub fn status_text(&self) -> &'static str {
        if self.passed { "pass" } else { "fail" }
    }
}

#[derive(Debug, Clone)]
pub struct VerifyReport {
    pub project: String,
    pub results: Vec<VerifyRunResult>,
}

impl VerifyReport {
    pub fn passed_count(&self) -> usize {
        self.results
            .iter()
            .filter(|result| result.status() == RunStatus::Passed)
            .count()
    }

    pub fn failed_count(&self) -> usize {
        self.results.len() - self.passed_count()
    }

    pub fn failure_count(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.failed_checks().count())
            .sum()
    }

    pub fn to_json(&self) -> String {
        json::report_json(self)
    }

    pub fn to_html(&self) -> String {
        html::report_html(self)
    }

    pub fn to_junit_xml(&self) -> String {
        junit::junit_xml(self)
    }

    pub fn to_markdown(&self) -> String {
        markdown::report_markdown(self)
    }
}

#[cfg(test)]
mod tests {
    use super::{CheckResult, VerifyReport, VerifyRunResult};
    use osl_core::{ParameterOverride, RunMetadata, RunStatus};
    use osl_waveform::WaveformSummary;

    #[test]
    fn renders_verification_report_json_and_html() {
        let report = VerifyReport {
            project: "demo".to_string(),
            results: vec![sample_run(
                "/tmp/demo/runs/rc_fast",
                false,
                Some(WaveformSummary {
                    samples: 2,
                    first: 0.0,
                    last: 1.0,
                    min: 0.0,
                    max: 1.0,
                    avg: 0.5,
                    peak_to_peak: 1.0,
                    rms: 0.707,
                }),
            )],
        };

        let json = report.to_json();
        let html = report.to_html();

        assert!(json.contains("\"project\": \"demo\""));
        assert!(json.contains("\"failure_count\": 1"));
        assert!(html.contains("NekoSpice Verification Report"));
        assert!(html.contains("runs/rc_fast/report.html"));
        assert!(html.contains("samples=2"));
    }

    #[test]
    fn renders_junit_xml_for_ci() {
        let report = VerifyReport {
            project: "demo & ci".to_string(),
            results: vec![sample_run(
                "/tmp/demo/runs/rc_fast",
                false,
                Some(WaveformSummary {
                    samples: 2,
                    first: 0.0,
                    last: 1.0,
                    min: 0.0,
                    max: 1.0,
                    avg: 0.5,
                    peak_to_peak: 1.0,
                    rms: 0.707,
                }),
            )],
        };

        let xml = report.to_junit_xml();

        assert!(xml.contains("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
        assert!(xml.contains("name=\"demo &amp; ci\""));
        assert!(xml.contains("tests=\"1\" failures=\"1\""));
        assert!(xml.contains("<testcase classname=\"rc.cir\" name=\"rc\" time=\"0.012000\">"));
        assert!(xml.contains("<failure message=\"1 failed check\">"));
        assert!(xml.contains("signal=v(out)"));
        assert!(xml.contains("summary=samples=2"));
    }

    #[test]
    fn renders_markdown_report_for_reviews() {
        let report = VerifyReport {
            project: "demo | review".to_string(),
            results: vec![sample_run(
                "/tmp/demo/runs/rc_fast",
                false,
                Some(WaveformSummary {
                    samples: 2,
                    first: 0.0,
                    last: 1.0,
                    min: 0.0,
                    max: 1.0,
                    avg: 0.5,
                    peak_to_peak: 1.0,
                    rms: 0.707,
                }),
            )],
        };

        let markdown = report.to_markdown();

        assert!(markdown.contains("# NekoSpice Verification Report"));
        assert!(markdown.contains("Project: `demo | review`"));
        assert!(markdown.contains("| Run | Parameters | Check | Message | Summary | Artifacts |"));
        assert!(markdown.contains("[report](runs/rc_fast/report.html)"));
        assert!(markdown.contains("samples=2"));
    }

    #[test]
    fn bench_style_run_dirs_link_directly_to_child_dirs() {
        let report = VerifyReport {
            project: "bench".to_string(),
            results: vec![sample_run("/tmp/bench/rc", true, None)],
        };

        let html = report.to_html();
        let markdown = report.to_markdown();

        assert!(html.contains("rc/report.html"));
        assert!(!html.contains("runs/rc/report.html"));
        assert!(markdown.contains("[report](rc/report.html)"));
        assert!(!markdown.contains("[report](runs/rc/report.html)"));
    }

    fn sample_run(
        run_dir: &str,
        passed: bool,
        summary: Option<WaveformSummary>,
    ) -> VerifyRunResult {
        VerifyRunResult {
            index: 0,
            name: "rc".to_string(),
            netlist: "rc.cir".to_string(),
            run_dir: run_dir.to_string(),
            metadata: RunMetadata {
                schema_version: 1,
                run_id: "run".to_string(),
                backend: "ngspice-cli".to_string(),
                backend_executable: "ngspice".to_string(),
                source_netlist: "rc.cir".to_string(),
                working_netlist: "input.cir".to_string(),
                output_dir: run_dir.to_string(),
                status: RunStatus::Passed,
                exit_code: Some(0),
                duration_ms: 12,
                started_unix_ms: 0,
                parameters: vec![ParameterOverride::new("rload", 1000.0)],
                artifacts: Vec::new(),
            },
            parameters: vec![ParameterOverride::new("rload", 1000.0)],
            checks: vec![CheckResult {
                name: "vout".to_string(),
                kind: "max".to_string(),
                signal: "v(out)".to_string(),
                from: None,
                to: None,
                value: Some(1.0),
                summary,
                min: Some(2.0),
                max: None,
                passed,
                message: "value=1 min=2 max=none window=all".to_string(),
            }],
        }
    }
}
