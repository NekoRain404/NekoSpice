//! JSON report formatter.

use crate::VerifyReport;
use crate::format::{option_f64_json, summary_json};
use osl_core::{json_escape, parameters_json};

/// report json。
pub(crate) fn report_json(report: &VerifyReport) -> String {
    let failures = report
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
    let runs = report
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
        json_escape(&report.project),
        report.passed_count(),
        report.failed_count(),
        report.failure_count(),
        failures,
        runs
    )
}
