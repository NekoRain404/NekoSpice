//! Simulation log parsing — extract errors, warnings, and summary from ngspice/Xyce output logs.

/// Parse ngspice log file to extract errors and warnings.
///
/// Returns (errors, warnings, summary_line).
pub fn parse_ngspice_log(log_content: &str) -> (Vec<String>, Vec<String>, Option<String>) {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let mut summary_line = None;

    for line in log_content.lines() {
        let trimmed = line.trim();
        let lower = trimmed.to_ascii_lowercase();

        if lower.contains("error") || lower.contains("fatal") {
            errors.push(trimmed.to_string());
        } else if lower.contains("warning") {
            warnings.push(trimmed.to_string());
        }

        // Look for convergence/success/failure summary lines
        if lower.contains("simulation aborted")
            || lower.contains("simulation done")
            || lower.contains("tran analysis")
            || lower.contains("operating point")
            || lower.contains("convergence")
            || lower.contains("singular matrix")
            || lower.contains("doAnalyses")
        {
            summary_line = Some(trimmed.to_string());
        }
    }

    (errors, warnings, summary_line)
}

/// Format ngspice log issues into a user-friendly message.
pub fn format_simulation_log_summary(
    errors: &[String],
    warnings: &[String],
    summary: Option<&str>,
) -> String {
    let mut parts = Vec::new();

    if let Some(summary) = summary {
        parts.push(summary.to_string());
    }

    if !errors.is_empty() {
        parts.push(format!("{} error(s) found", errors.len()));
        // Include first 3 errors for brevity
        for error in errors.iter().take(3) {
            parts.push(format!("  -> {}", error));
        }
    }

    if !warnings.is_empty() {
        parts.push(format!("{} warning(s)", warnings.len()));
    }

    if parts.is_empty() {
        "No issues found in log".to_string()
    } else {
        parts.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_log_extracts_errors() {
        let log = "ngspice warning: singular matrix\nError: cannot find model\nDone.";
        let (errors, warnings, _summary) = parse_ngspice_log(log);
        assert!(errors.iter().any(|e| e.contains("cannot find model")));
        assert!(warnings.iter().any(|w| w.contains("singular matrix")));
    }
}
