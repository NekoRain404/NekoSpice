use crate::report_css;
use osl_core::{OslError, OslResult, html_escape, read_text, write_text};
use std::path::{Path, PathBuf};

const REPORT_HTML: &str = "report.html";

#[derive(Debug, Clone, Copy)]
struct DirectoryReportSource {
    file_name: &'static str,
    title: &'static str,
}

const REPORT_SOURCES: &[DirectoryReportSource] = &[
    DirectoryReportSource {
        file_name: "run.json",
        title: "NekoSpice Run Report",
    },
    DirectoryReportSource {
        file_name: "verify.json",
        title: "NekoSpice Batch Report",
    },
    DirectoryReportSource {
        file_name: "model-check.json",
        title: "NekoSpice Batch Report",
    },
    DirectoryReportSource {
        file_name: "import.json",
        title: "NekoSpice Batch Report",
    },
    DirectoryReportSource {
        file_name: "bench.json",
        title: "NekoSpice Batch Report",
    },
];

pub fn write_report_directory_html(dir: &Path) -> OslResult<PathBuf> {
    let output_path = dir.join(REPORT_HTML);
    if output_path.is_file() {
        return Ok(output_path);
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
    Ok(output_path)
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
    use super::write_report_directory_html;
    use osl_core::{OslError, read_text, write_text};

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
    fn preserves_existing_directory_report_html() {
        let dir = temp_report_dir("existing");
        write_text(&dir.join("run.json"), "{\"status\":\"passed\"}").unwrap();
        write_text(&dir.join("report.html"), "<html>rich report</html>").unwrap();

        let output = write_report_directory_html(&dir).unwrap();
        let html = read_text(&output).unwrap();

        assert_eq!(output, dir.join("report.html"));
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
            osl_core::now_unix_ms()
        ))
    }
}
