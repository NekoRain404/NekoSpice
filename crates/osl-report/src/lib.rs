use osl_core::{
    ParameterOverride, RunMetadata, RunStatus, html_escape, json_escape, parameters_json,
};
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

    fn artifact_href(&self, artifact: &str) -> String {
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
        let failures = self
            .results
            .iter()
            .flat_map(|result| {
                result.failed_checks().map(|check| {
                    format!(
                        concat!(
                            "    {{ \"run\": \"{}\", \"netlist\": \"{}\", \"run_dir\": \"{}\", ",
                            "\"check\": \"{}\", \"signal\": \"{}\", \"value\": {}, ",
                            "\"min\": {}, \"max\": {}, \"summary\": {}, \"message\": \"{}\" }}"
                        ),
                        json_escape(&result.name),
                        json_escape(&result.netlist),
                        json_escape(&result.run_dir),
                        json_escape(&check.name),
                        json_escape(&check.signal),
                        option_f64_json(check.value),
                        option_f64_json(check.min),
                        option_f64_json(check.max),
                        summary_json(check.summary),
                        json_escape(&check.message)
                    )
                })
            })
            .collect::<Vec<_>>()
            .join(",\n");
        let runs = self
            .results
            .iter()
            .map(|result| {
                let parameters = parameters_json(&result.parameters, 8);
                let checks = result
                    .checks
                    .iter()
                    .map(|check| {
                        format!(
                            concat!(
                                "        {{ \"name\": \"{}\", \"kind\": \"{}\", \"signal\": \"{}\", ",
                                "\"from\": {}, \"to\": {}, \"value\": {}, \"min\": {}, \"max\": {}, \"passed\": {}, \"summary\": {}, \"message\": \"{}\" }}"
                            ),
                            json_escape(&check.name),
                            json_escape(&check.kind),
                            json_escape(&check.signal),
                            option_f64_json(check.from),
                            option_f64_json(check.to),
                            option_f64_json(check.value),
                            option_f64_json(check.min),
                            option_f64_json(check.max),
                            check.passed,
                            summary_json(check.summary),
                            json_escape(&check.message)
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(",\n");
                format!(
                    concat!(
                        "    {{\n",
                        "      \"name\": \"{}\",\n",
                        "      \"netlist\": \"{}\",\n",
                        "      \"run_dir\": \"{}\",\n",
                        "      \"status\": \"{}\",\n",
                        "      \"simulation_status\": \"{}\",\n",
                        "      \"exit_code\": {},\n",
                        "      \"duration_ms\": {},\n",
                        "      \"parameters\": [\n",
                        "{}\n",
                        "      ],\n",
                        "      \"checks\": [\n",
                        "{}\n",
                        "      ]\n",
                        "    }}"
                    ),
                    json_escape(&result.name),
                    json_escape(&result.netlist),
                    json_escape(&result.run_dir),
                    result.status().as_str(),
                    result.metadata.status.as_str(),
                    result
                        .metadata
                        .exit_code
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "null".to_string()),
                    result.metadata.duration_ms,
                    parameters,
                    checks
                )
            })
            .collect::<Vec<_>>()
            .join(",\n");

        format!(
            concat!(
                "{{\n",
                "  \"schema_version\": 1,\n",
                "  \"project\": \"{}\",\n",
                "  \"passed\": {},\n",
                "  \"failed\": {},\n",
                "  \"failure_count\": {},\n",
                "  \"failures\": [\n",
                "{}\n",
                "  ],\n",
                "  \"runs\": [\n",
                "{}\n",
                "  ]\n",
                "}}\n"
            ),
            json_escape(&self.project),
            self.passed_count(),
            self.failed_count(),
            self.failure_count(),
            failures,
            runs
        )
    }

    pub fn to_html(&self) -> String {
        let failure_rows = self
            .results
            .iter()
            .flat_map(|result| {
                result.failed_checks().map(|check| {
                    format!(
                        concat!(
                            "<tr class=\"failed\"><td>{}</td><td>{}</td><td>{}</td>",
                            "<td>{}</td><td>{}</td><td><a href=\"{}\">run</a> <a href=\"{}\">raw</a> <a href=\"{}\">csv</a> <a href=\"{}\">summary</a> <a href=\"{}\">log</a></td></tr>"
                        ),
                        html_escape(&result.name),
                        html_escape(&parameters_text(&result.parameters)),
                        html_escape(&check.name),
                        html_escape(&check.message),
                        html_escape(&summary_text(check.summary)),
                        html_escape(&result.artifact_href("report.html")),
                        html_escape(&result.artifact_href("waveform.raw")),
                        html_escape(&result.artifact_href("waveform.csv")),
                        html_escape(&result.artifact_href("waveform-summary.json")),
                        html_escape(&result.artifact_href("ngspice.log"))
                    )
                })
            })
            .collect::<String>();
        let failure_section = if self.failure_count() == 0 {
            "<section class=\"summary\"><strong>No failed checks.</strong></section>".to_string()
        } else {
            format!(
                concat!(
                    "<section><h2>Failures</h2>",
                    "<table><thead><tr><th>Run</th><th>Parameters</th><th>Check</th><th>Message</th><th>Summary</th><th>Artifacts</th></tr></thead>",
                    "<tbody>{}</tbody></table></section>"
                ),
                failure_rows
            )
        };

        let rows = self
            .results
            .iter()
            .map(|result| {
                let status = result.status().as_str();
                let parameters = parameters_text(&result.parameters);
                let checks = if result.checks.is_empty() {
                    "no checks".to_string()
                } else {
                    result
                        .checks
                        .iter()
                        .map(|check| {
                            format!(
                                "{}: {} ({}; {})",
                                check.name,
                                check.status_text(),
                                check.message,
                                summary_text(check.summary)
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("; ")
                };
                format!(
                    concat!(
                        "<tr class=\"{}\"><td>{}</td><td>{}</td><td>{}</td>",
                        "<td>{}</td><td>{}</td><td><a href=\"{}\">report</a> <a href=\"{}\">run.json</a> <a href=\"{}\">raw</a> <a href=\"{}\">csv</a> <a href=\"{}\">summary</a> <a href=\"{}\">ngspice.log</a> <a href=\"{}\">input.cir</a></td><td>{}</td></tr>"
                    ),
                    status,
                    html_escape(status),
                    html_escape(&result.name),
                    html_escape(&result.netlist),
                    html_escape(&parameters),
                    result.metadata.duration_ms,
                    html_escape(&result.artifact_href("report.html")),
                    html_escape(&result.artifact_href("run.json")),
                    html_escape(&result.artifact_href("waveform.raw")),
                    html_escape(&result.artifact_href("waveform.csv")),
                    html_escape(&result.artifact_href("waveform-summary.json")),
                    html_escape(&result.artifact_href("ngspice.log")),
                    html_escape(&result.artifact_href("input.cir")),
                    html_escape(&checks)
                )
            })
            .collect::<String>();

        format!(
            concat!(
                "<!doctype html><html><head><meta charset=\"utf-8\">",
                "<title>NekoSpice Verification Report</title>{}</head><body>",
                "<main><h1>{}</h1>",
                "<section class=\"summary\"><strong>{}</strong> passed, <strong>{}</strong> failed, <strong>{}</strong> failed checks</section>",
                "{}",
                "<h2>Runs</h2>",
                "<table><thead><tr><th>Status</th><th>Run</th><th>Netlist</th><th>Parameters</th><th>ms</th><th>Artifacts</th><th>Checks</th></tr></thead>",
                "<tbody>{}</tbody></table>",
                "</main></body></html>\n"
            ),
            report_css(),
            html_escape(&self.project),
            self.passed_count(),
            self.failed_count(),
            self.failure_count(),
            failure_section,
            rows
        )
    }

    pub fn to_junit_xml(&self) -> String {
        let testcases = self
            .results
            .iter()
            .map(junit_testcase_xml)
            .collect::<String>();
        format!(
            concat!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n",
                "<testsuite name=\"{}\" tests=\"{}\" failures=\"{}\" errors=\"0\" time=\"{}\">\n",
                "{}",
                "</testsuite>\n"
            ),
            xml_escape(&self.project),
            self.results.len(),
            self.failed_count(),
            junit_seconds(
                self.results
                    .iter()
                    .map(|result| result.metadata.duration_ms)
                    .sum()
            ),
            testcases
        )
    }
}

fn junit_testcase_xml(result: &VerifyRunResult) -> String {
    let testcase_open = format!(
        "  <testcase classname=\"{}\" name=\"{}\" time=\"{}\">",
        xml_escape(&result.netlist),
        xml_escape(&result.name),
        junit_seconds(result.metadata.duration_ms)
    );
    if result.status() == RunStatus::Passed {
        return format!("{testcase_open}</testcase>\n");
    }

    let message = result_failure_message(result);
    let details = result_failure_details(result);
    format!(
        concat!(
            "{}\n",
            "    <failure message=\"{}\"><![CDATA[{}]]></failure>\n",
            "  </testcase>\n"
        ),
        testcase_open,
        xml_escape(&message),
        cdata_escape(&details)
    )
}

fn result_failure_message(result: &VerifyRunResult) -> String {
    let failed_checks = result.failed_checks().count();
    if result.metadata.status != RunStatus::Passed {
        format!(
            "simulation {} exit {:?}",
            result.metadata.status.as_str(),
            result.metadata.exit_code
        )
    } else if failed_checks == 1 {
        "1 failed check".to_string()
    } else {
        format!("{failed_checks} failed checks")
    }
}

fn result_failure_details(result: &VerifyRunResult) -> String {
    let mut details = Vec::new();
    details.push(format!("run: {}", result.name));
    details.push(format!("netlist: {}", result.netlist));
    details.push(format!("run_dir: {}", result.run_dir));
    details.push(format!(
        "simulation_status: {}",
        result.metadata.status.as_str()
    ));
    details.push(format!("exit_code: {:?}", result.metadata.exit_code));
    details.push(format!("duration_ms: {}", result.metadata.duration_ms));

    let parameters = parameters_text(&result.parameters);
    details.push(format!("parameters: {parameters}"));

    for check in result.failed_checks() {
        details.push(format!(
            "check {} {} signal={} value={} min={} max={} summary={} message={}",
            check.name,
            check.status_text(),
            check.signal,
            option_f64_text(check.value),
            option_f64_text(check.min),
            option_f64_text(check.max),
            summary_text(check.summary),
            check.message
        ));
    }

    details.join("\n")
}

pub fn report_css() -> &'static str {
    concat!(
        "<style>",
        "body{margin:0;background:#f7f7f4;color:#1f2933;font:14px/1.5 system-ui,-apple-system,BlinkMacSystemFont,sans-serif}",
        "main{max-width:1100px;margin:0 auto;padding:32px}",
        "h1{font-size:28px;margin:0 0 20px}",
        "h2{font-size:18px;margin-top:24px}",
        ".summary{background:#fff;border:1px solid #d9ded7;border-radius:6px;padding:12px 14px;margin-bottom:16px}",
        "table{width:100%;border-collapse:collapse;background:#fff;border:1px solid #d9ded7}",
        "th,td{padding:10px 12px;border-bottom:1px solid #e5e7eb;text-align:left;vertical-align:top}",
        "th{background:#ecefeb;font-weight:700}",
        "tr.passed td:first-child{color:#0f766e;font-weight:700}",
        "tr.warning td:first-child{color:#b45309;font-weight:700}",
        "tr.failed td:first-child{color:#b91c1c;font-weight:700}",
        "code,pre{font-family:ui-monospace,SFMono-Regular,Consolas,monospace}",
        "pre{white-space:pre-wrap;background:#fff;border:1px solid #d9ded7;border-radius:6px;padding:16px;overflow:auto}",
        "dl{display:grid;grid-template-columns:120px 1fr;gap:8px 12px;background:#fff;border:1px solid #d9ded7;border-radius:6px;padding:14px}",
        "dt{font-weight:700}",
        "ul{background:#fff;border:1px solid #d9ded7;border-radius:6px;padding:14px 14px 14px 30px}",
        "</style>"
    )
}

fn option_f64_json(value: Option<f64>) -> String {
    value
        .map(|value| {
            if value.is_finite() {
                value.to_string()
            } else {
                "null".to_string()
            }
        })
        .unwrap_or_else(|| "null".to_string())
}

fn option_f64_text(value: Option<f64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "none".to_string())
}

fn summary_json(summary: Option<WaveformSummary>) -> String {
    match summary {
        Some(summary) => format!(
            concat!(
                "{{ \"samples\": {}, \"first\": {}, \"last\": {}, \"min\": {}, ",
                "\"max\": {}, \"avg\": {}, \"pp\": {}, \"rms\": {} }}"
            ),
            summary.samples,
            summary.first,
            summary.last,
            summary.min,
            summary.max,
            summary.avg,
            summary.peak_to_peak,
            summary.rms
        ),
        None => "null".to_string(),
    }
}

fn summary_text(summary: Option<WaveformSummary>) -> String {
    match summary {
        Some(summary) => format!(
            "samples={} first={} last={} min={} max={} avg={} pp={} rms={}",
            summary.samples,
            summary.first,
            summary.last,
            summary.min,
            summary.max,
            summary.avg,
            summary.peak_to_peak,
            summary.rms
        ),
        None => "summary unavailable".to_string(),
    }
}

fn parameters_text(parameters: &[ParameterOverride]) -> String {
    if parameters.is_empty() {
        "none".to_string()
    } else {
        parameters
            .iter()
            .map(|parameter| format!("{}={}", parameter.name, parameter.value))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn junit_seconds(duration_ms: u128) -> String {
    format!("{:.6}", duration_ms as f64 / 1000.0)
}

fn xml_escape(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());
    for character in input.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            character => escaped.push(character),
        }
    }
    escaped
}

fn cdata_escape(input: &str) -> String {
    input.replace("]]>", "]]]]><![CDATA[>")
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
    fn bench_style_run_dirs_link_directly_to_child_dirs() {
        let report = VerifyReport {
            project: "bench".to_string(),
            results: vec![sample_run("/tmp/bench/rc", true, None)],
        };

        let html = report.to_html();

        assert!(html.contains("rc/report.html"));
        assert!(!html.contains("runs/rc/report.html"));
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
