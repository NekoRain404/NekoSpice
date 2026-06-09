use crate::format::{
    cdata_escape, junit_seconds, option_f64_text, parameters_text, summary_text, xml_escape,
};
use crate::{VerifyReport, VerifyRunResult};
use osl_core::RunStatus;

pub(crate) fn junit_xml(report: &VerifyReport) -> String {
    let testcases = report
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
        xml_escape(&report.project),
        report.results.len(),
        report.failed_count(),
        junit_seconds(
            report
                .results
                .iter()
                .map(|result| result.metadata.duration_ms)
                .sum()
        ),
        testcases
    )
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
