use crate::VerifyReport;
use crate::format::{parameters_text, summary_text};
use osl_core::html_escape;

/// report html。
pub(crate) fn report_html(report: &VerifyReport) -> String {
    let failure_rows = report
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
    let failure_section = if report.failure_count() == 0 {
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

    let rows = report
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
        html_escape(&report.project),
        report.passed_count(),
        report.failed_count(),
        report.failure_count(),
        failure_section,
        rows
    )
}

/// report css。
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
