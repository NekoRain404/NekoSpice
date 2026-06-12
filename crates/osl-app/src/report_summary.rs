//! 报告汇总数据结构。
//!
use osl_report::write_report_directory_summary;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum GuiReportSummaryState {
    Ready(GuiReportSummary),
    Missing(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GuiReportSummary {
    pub(crate) report_file: String,
    pub(crate) source_file: Option<String>,
    pub(crate) source_kind: Option<&'static str>,
    pub(crate) reused_existing_html: bool,
    pub(crate) size_bytes: u64,
}

impl GuiReportSummary {
    /// from report dir。
    pub(crate) fn from_report_dir(output_dir: &Path) -> GuiReportSummaryState {
        match write_report_directory_summary(output_dir) {
            Ok(summary) => {
                let size_bytes = fs::metadata(&summary.report_path)
                    .map(|metadata| metadata.len())
                    .unwrap_or_default();
                GuiReportSummaryState::Ready(Self {
                    report_file: file_name_text(&summary.report_path),
                    source_file: summary.source_path.as_deref().map(file_name_text),
                    source_kind: summary.source_kind,
                    reused_existing_html: summary.reused_existing_html,
                    size_bytes,
                })
            }
            Err(error) => GuiReportSummaryState::Missing(error.to_string()),
        }
    }
}

fn file_name_text(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::{GuiReportSummary, GuiReportSummaryState};
    use osl_core::write_text;

    #[test]
    fn summarizes_existing_run_report_for_gui() {
        let output_dir = temp_report_dir("ready");
        write_text(&output_dir.join("run.json"), "{\"status\":\"passed\"}").unwrap();
        write_text(&output_dir.join("report.html"), "<html>rich</html>").unwrap();

        let GuiReportSummaryState::Ready(summary) = GuiReportSummary::from_report_dir(&output_dir)
        else {
            panic!("expected report summary");
        };

        assert_eq!(summary.report_file, "report.html");
        assert_eq!(summary.source_file.as_deref(), Some("run.json"));
        assert_eq!(summary.source_kind, Some("run"));
        assert!(summary.reused_existing_html);
        assert_eq!(summary.size_bytes, 17);

        let _ = std::fs::remove_dir_all(output_dir);
    }

    #[test]
    fn reports_missing_gui_report_artifacts() {
        let output_dir = temp_report_dir("missing");

        let GuiReportSummaryState::Missing(message) =
            GuiReportSummary::from_report_dir(&output_dir)
        else {
            panic!("expected missing report state");
        };

        assert!(message.contains("does not contain run.json"));

        let _ = std::fs::remove_dir_all(output_dir);
    }

    fn temp_report_dir(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "nekospice_gui_report_summary_{name}_{}_{}",
            std::process::id(),
            osl_core::now_unix_ms()
        ))
    }
}
