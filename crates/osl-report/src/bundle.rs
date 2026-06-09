use crate::VerifyReport;
use osl_core::{OslResult, write_text};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportBundleFile {
    pub path: &'static str,
    pub kind: &'static str,
}

pub fn write_verify_report_bundle(
    output_dir: &Path,
    report: &VerifyReport,
) -> OslResult<Vec<ReportBundleFile>> {
    write_report_bundle(output_dir, "verify.json", report)
}

pub fn write_bench_report_bundle(
    output_dir: &Path,
    report: &VerifyReport,
) -> OslResult<Vec<ReportBundleFile>> {
    write_report_bundle(output_dir, "bench.json", report)
}

pub fn write_json_html_report_bundle(
    output_dir: &Path,
    json_name: &'static str,
    json: &str,
    html: &str,
) -> OslResult<Vec<ReportBundleFile>> {
    write_text(&output_dir.join(json_name), json)?;
    write_text(&output_dir.join("report.html"), html)?;

    Ok(vec![
        ReportBundleFile {
            path: json_name,
            kind: "json",
        },
        ReportBundleFile {
            path: "report.html",
            kind: "html",
        },
    ])
}

fn write_report_bundle(
    output_dir: &Path,
    json_name: &'static str,
    report: &VerifyReport,
) -> OslResult<Vec<ReportBundleFile>> {
    write_text(&output_dir.join(json_name), &report.to_json())?;
    write_text(&output_dir.join("report.html"), &report.to_html())?;
    write_text(&output_dir.join("report.md"), &report.to_markdown())?;
    write_text(&output_dir.join("junit.xml"), &report.to_junit_xml())?;

    Ok(vec![
        ReportBundleFile {
            path: json_name,
            kind: "json",
        },
        ReportBundleFile {
            path: "report.html",
            kind: "html",
        },
        ReportBundleFile {
            path: "report.md",
            kind: "markdown",
        },
        ReportBundleFile {
            path: "junit.xml",
            kind: "junit",
        },
    ])
}
