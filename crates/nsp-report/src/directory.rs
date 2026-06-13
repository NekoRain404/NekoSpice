//! Directory-based report output — writes reports to run output directories.

use crate::report_css;
use nsp_core::{OslError, OslResult, html_escape, read_text, write_text};
use std::path::{Path, PathBuf};

const REPORT_HTML: &str = "report.html";

#[derive(Debug, Clone, Copy)]
struct DirectoryReportSource {
    file_name: &'static str,
    kind: &'static str,
    title: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportDirectorySummary {
    pub report_path: PathBuf,
    pub source_path: Option<PathBuf>,
    pub source_kind: Option<&'static str>,
    pub reused_existing_html: bool,
}

const REPORT_SOURCES: &[DirectoryReportSource] = &[
    DirectoryReportSource {
        file_name: "run.json",
        kind: "run",
        title: "NekoSpice Run Report",
    },
    DirectoryReportSource {
        file_name: "verify.json",
        kind: "verify",
        title: "NekoSpice Batch Report",
    },
    DirectoryReportSource {
        file_name: "model-check.json",
        kind: "model-check",
        title: "NekoSpice Batch Report",
    },
    DirectoryReportSource {
        file_name: "import.json",
        kind: "import",
        title: "NekoSpice Batch Report",
    },
    DirectoryReportSource {
        file_name: "bench.json",
        kind: "bench",
        title: "NekoSpice Batch Report",
    },
];

/// write report directory html。
pub fn write_report_directory_html(dir: &Path) -> OslResult<PathBuf> {
    write_report_directory_summary(dir).map(|summary| summary.report_path)
}

/// write report directory summary。
pub fn write_report_directory_summary(dir: &Path) -> OslResult<ReportDirectorySummary> {
    let output_path = dir.join(REPORT_HTML);
    if output_path.is_file() {
        let source = select_report_source(dir);
        return Ok(ReportDirectorySummary {
            report_path: output_path,
            source_path: source.map(|source| dir.join(source.file_name)),
            source_kind: source.map(|source| source.kind),
            reused_existing_html: true,
        });
    }

    let source = select_report_source(dir).ok_or_else(|| {
        OslError::InvalidInput(format!(
            "{} does not contain run.json, verify.json, bench.json, model-check.json, or import.json",
            dir.display()
        ))
    })?;
    let content = read_text(&dir.join(source.file_name))?;
    write_text(
        &output_path,
        &json_preview_report_html(source.title, &content),
    )?;
    Ok(ReportDirectorySummary {
        report_path: output_path,
        source_path: Some(dir.join(source.file_name)),
        source_kind: Some(source.kind),
        reused_existing_html: false,
    })
}

fn select_report_source(dir: &Path) -> Option<DirectoryReportSource> {
    REPORT_SOURCES
        .iter()
        .copied()
        .find(|source| dir.join(source.file_name).is_file())
}

fn json_preview_report_html(title: &str, content: &str) -> String {
    format!(
        concat!(
            "<!doctype html><html><head><meta charset=\"utf-8\">",
            "<title>{}</title>{}</head><body>",
            "<main><h1>{}</h1><pre>{}</pre></main></body></html>\n"
        ),
        html_escape(title),
        report_css(),
        html_escape(title),
        html_escape(content)
    )
}

#[cfg(test)]
mod tests {
    use super::{write_report_directory_html, write_report_directory_summary};
    use nsp_core::{OslError, read_text, write_text};

    #[test]
    fn writes_run_directory_report_from_run_json() {
        let dir = temp_report_dir("run");
        write_text(&dir.join("run.json"), "{\"status\":\"passed\"}").unwrap();

        let output = write_report_directory_html(&dir).unwrap();
        let html = read_text(&output).unwrap();

        assert_eq!(output, dir.join("report.html"));
        assert!(html.contains("NekoSpice Run Report"));
        assert!(html.contains("{&quot;status&quot;:&quot;passed&quot;}"));

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn writes_batch_directory_report_from_verify_json() {
        let dir = temp_report_dir("batch");
        write_text(&dir.join("bench.json"), "{\"project\":\"bench\"}").unwrap();
        write_text(&dir.join("verify.json"), "{\"project\":\"verify\"}").unwrap();

        let output = write_report_directory_html(&dir).unwrap();
        let html = read_text(&output).unwrap();

        assert_eq!(output, dir.join("report.html"));
        assert!(html.contains("NekoSpice Batch Report"));
        assert!(html.contains("{&quot;project&quot;:&quot;verify&quot;}"));
        assert!(!html.contains("{&quot;project&quot;:&quot;bench&quot;}"));

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn reports_directory_summary_metadata() {
        let dir = temp_report_dir("summary");
        write_text(&dir.join("model-check.json"), "{\"kind\":\"model\"}").unwrap();

        let summary = write_report_directory_summary(&dir).unwrap();

        assert_eq!(summary.report_path, dir.join("report.html"));
        assert_eq!(summary.source_path, Some(dir.join("model-check.json")));
        assert_eq!(summary.source_kind, Some("model-check"));
        assert!(!summary.reused_existing_html);

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn preserves_existing_directory_report_html() {
        let dir = temp_report_dir("existing");
        write_text(&dir.join("run.json"), "{\"status\":\"passed\"}").unwrap();
        write_text(&dir.join("report.html"), "<html>rich report</html>").unwrap();

        let summary = write_report_directory_summary(&dir).unwrap();
        let html = read_text(&summary.report_path).unwrap();

        assert_eq!(summary.report_path, dir.join("report.html"));
        assert_eq!(summary.source_path, Some(dir.join("run.json")));
        assert_eq!(summary.source_kind, Some("run"));
        assert!(summary.reused_existing_html);
        assert_eq!(html, "<html>rich report</html>");

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn reports_missing_directory_report_source() {
        let dir = temp_report_dir("missing");

        let error = write_report_directory_html(&dir).unwrap_err();

        assert!(matches!(error, OslError::InvalidInput(_)));
        assert!(
            error
                .to_string()
                .contains("does not contain run.json, verify.json")
        );

        let _ = std::fs::remove_dir_all(dir);
    }

    fn temp_report_dir(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "nekospice_directory_report_{name}_{}_{}",
            std::process::id(),
            nsp_core::now_unix_ms()
        ))
    }
}
