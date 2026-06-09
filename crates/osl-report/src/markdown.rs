use crate::format::{
    markdown_cell, markdown_inline, markdown_link, markdown_link_cell, parameters_text,
    summary_text,
};
use crate::{CheckResult, VerifyReport, VerifyRunResult};

pub(crate) fn report_markdown(report: &VerifyReport) -> String {
    let mut output = String::new();
    output.push_str("# NekoSpice Verification Report\n\n");
    output.push_str(&format!(
        "Project: `{}`\n\n",
        markdown_inline(&report.project)
    ));
    output.push_str(&format!(
        "- Passed: {}\n- Failed: {}\n- Failed checks: {}\n\n",
        report.passed_count(),
        report.failed_count(),
        report.failure_count()
    ));

    output.push_str("## Failures\n\n");
    if report.failure_count() == 0 {
        output.push_str("No failed checks.\n\n");
    } else {
        output.push_str("| Run | Parameters | Check | Message | Summary | Artifacts |\n");
        output.push_str("| --- | --- | --- | --- | --- | --- |\n");
        for result in &report.results {
            for check in result.failed_checks() {
                output.push_str(&format!(
                    "| {} | {} | {} | {} | {} | {} |\n",
                    markdown_cell(&result.name),
                    markdown_cell(&parameters_text(&result.parameters)),
                    markdown_cell(&check.name),
                    markdown_cell(&check.message),
                    markdown_cell(&summary_text(check.summary)),
                    markdown_link_cell(&failure_artifact_links(result))
                ));
            }
        }
        output.push('\n');
    }

    output.push_str("## Runs\n\n");
    output.push_str("| Status | Run | Netlist | Parameters | ms | Artifacts | Checks |\n");
    output.push_str("| --- | --- | --- | --- | ---: | --- | --- |\n");
    for result in &report.results {
        output.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} |\n",
            markdown_cell(result.status().as_str()),
            markdown_cell(&result.name),
            markdown_cell(&result.netlist),
            markdown_cell(&parameters_text(&result.parameters)),
            result.metadata.duration_ms,
            markdown_link_cell(&run_artifact_links(result)),
            markdown_cell(&checks_text(&result.checks))
        ));
    }
    output.push('\n');
    output
}

fn failure_artifact_links(result: &VerifyRunResult) -> String {
    [
        ("run", "report.html"),
        ("raw", "waveform.raw"),
        ("csv", "waveform.csv"),
        ("summary", "waveform-summary.json"),
        ("log", "ngspice.log"),
    ]
    .iter()
    .map(|(label, artifact)| markdown_link(label, &result.artifact_href(artifact)))
    .collect::<Vec<_>>()
    .join(" ")
}

fn run_artifact_links(result: &VerifyRunResult) -> String {
    [
        ("report", "report.html"),
        ("run.json", "run.json"),
        ("raw", "waveform.raw"),
        ("csv", "waveform.csv"),
        ("summary", "waveform-summary.json"),
        ("ngspice.log", "ngspice.log"),
        ("input.cir", "input.cir"),
    ]
    .iter()
    .map(|(label, artifact)| markdown_link(label, &result.artifact_href(artifact)))
    .collect::<Vec<_>>()
    .join(" ")
}

fn checks_text(checks: &[CheckResult]) -> String {
    if checks.is_empty() {
        return "no checks".to_string();
    }

    checks
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
}
