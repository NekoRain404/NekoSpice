use osl_core::{OslError, OslResult, html_escape, json_escape, read_text};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct ModelCheckReport {
    pub root: String,
    pub files: Vec<ModelFileSummary>,
    pub subckts: Vec<SubcktSummary>,
    pub models: Vec<ModelSummary>,
    pub diagnostics: Vec<ModelDiagnostic>,
}

impl ModelCheckReport {
    pub fn scan(root: &Path) -> OslResult<Self> {
        let files = find_model_files(root)?;
        if files.is_empty() {
            return Err(OslError::InvalidInput(format!(
                "no SPICE model or netlist files found under {}",
                root.display()
            )));
        }

        let mut report = Self {
            root: root.display().to_string(),
            files: Vec::new(),
            subckts: Vec::new(),
            models: Vec::new(),
            diagnostics: Vec::new(),
        };

        for file in files {
            scan_model_file(&file, &mut report)?;
        }

        report
            .files
            .sort_by(|left, right| left.path.cmp(&right.path));
        report.subckts.sort_by(|left, right| {
            left.file
                .cmp(&right.file)
                .then(left.line.cmp(&right.line))
                .then(left.name.cmp(&right.name))
        });
        report.models.sort_by(|left, right| {
            left.file
                .cmp(&right.file)
                .then(left.line.cmp(&right.line))
                .then(left.name.cmp(&right.name))
        });
        report.diagnostics.sort_by(|left, right| {
            left.file
                .cmp(&right.file)
                .then(left.line.cmp(&right.line))
                .then(left.severity.rank().cmp(&right.severity.rank()))
                .then(left.code.cmp(&right.code))
        });

        Ok(report)
    }

    pub fn error_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
            .count()
    }

    pub fn warning_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == DiagnosticSeverity::Warning)
            .count()
    }

    pub fn info_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == DiagnosticSeverity::Info)
            .count()
    }

    pub fn compatibility_score(&self) -> u32 {
        let penalty = self.error_count() as u32 * 25 + self.warning_count() as u32 * 8;
        100_u32.saturating_sub(penalty)
    }

    pub fn to_json(&self) -> String {
        let files = self
            .files
            .iter()
            .map(ModelFileSummary::to_json)
            .collect::<Vec<_>>()
            .join(",\n");
        let subckts = self
            .subckts
            .iter()
            .map(SubcktSummary::to_json)
            .collect::<Vec<_>>()
            .join(",\n");
        let models = self
            .models
            .iter()
            .map(ModelSummary::to_json)
            .collect::<Vec<_>>()
            .join(",\n");
        let diagnostics = self
            .diagnostics
            .iter()
            .map(ModelDiagnostic::to_json)
            .collect::<Vec<_>>()
            .join(",\n");

        format!(
            concat!(
                "{{\n",
                "  \"schema_version\": 1,\n",
                "  \"root\": \"{}\",\n",
                "  \"compatibility_score\": {},\n",
                "  \"files_checked\": {},\n",
                "  \"subckt_count\": {},\n",
                "  \"model_count\": {},\n",
                "  \"diagnostic_count\": {},\n",
                "  \"errors\": {},\n",
                "  \"warnings\": {},\n",
                "  \"infos\": {},\n",
                "  \"files\": [\n",
                "{}\n",
                "  ],\n",
                "  \"subckts\": [\n",
                "{}\n",
                "  ],\n",
                "  \"models\": [\n",
                "{}\n",
                "  ],\n",
                "  \"diagnostics\": [\n",
                "{}\n",
                "  ]\n",
                "}}\n"
            ),
            json_escape(&self.root),
            self.compatibility_score(),
            self.files.len(),
            self.subckts.len(),
            self.models.len(),
            self.diagnostics.len(),
            self.error_count(),
            self.warning_count(),
            self.info_count(),
            files,
            subckts,
            models,
            diagnostics
        )
    }

    pub fn to_html(&self, css: &str) -> String {
        let diagnostic_rows = if self.diagnostics.is_empty() {
            "<tr><td colspan=\"6\">No diagnostics.</td></tr>".to_string()
        } else {
            self.diagnostics
                .iter()
                .map(|diagnostic| {
                    format!(
                        concat!(
                            "<tr class=\"{}\"><td>{}</td><td>{}</td><td>{}</td>",
                            "<td>{}</td><td>{}</td><td>{}</td></tr>"
                        ),
                        html_escape(diagnostic.severity.css_class()),
                        html_escape(diagnostic.severity.as_str()),
                        html_escape(&diagnostic.file),
                        diagnostic.line,
                        html_escape(&diagnostic.code),
                        html_escape(&diagnostic.message),
                        html_escape(&diagnostic.suggestion)
                    )
                })
                .collect::<String>()
        };
        let subckt_rows = if self.subckts.is_empty() {
            "<tr><td colspan=\"5\">No subcircuits found.</td></tr>".to_string()
        } else {
            self.subckts
                .iter()
                .map(|subckt| {
                    format!(
                        "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                        html_escape(&subckt.name),
                        html_escape(&subckt.file),
                        subckt.line,
                        subckt.pin_count,
                        html_escape(&subckt.pins.join(", "))
                    )
                })
                .collect::<String>()
        };
        let model_rows = if self.models.is_empty() {
            "<tr><td colspan=\"4\">No .model statements found.</td></tr>".to_string()
        } else {
            self.models
                .iter()
                .map(|model| {
                    format!(
                        "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                        html_escape(&model.name),
                        html_escape(&model.kind),
                        html_escape(&model.file),
                        model.line
                    )
                })
                .collect::<String>()
        };

        format!(
            concat!(
                "<!doctype html><html><head><meta charset=\"utf-8\">",
                "<title>NekoSpice Model Check Report</title>{}</head><body>",
                "<main><h1>Model Check</h1>",
                "<section class=\"summary\"><strong>Score:</strong> {} ",
                "<strong>Files:</strong> {} <strong>Subckts:</strong> {} <strong>Models:</strong> {} ",
                "<strong>Errors:</strong> {} <strong>Warnings:</strong> {}</section>",
                "<h2>Diagnostics</h2>",
                "<table><thead><tr><th>Severity</th><th>File</th><th>Line</th><th>Code</th><th>Message</th><th>Suggestion</th></tr></thead>",
                "<tbody>{}</tbody></table>",
                "<h2>Subcircuits</h2>",
                "<table><thead><tr><th>Name</th><th>File</th><th>Line</th><th>Pins</th><th>Pin List</th></tr></thead>",
                "<tbody>{}</tbody></table>",
                "<h2>Models</h2>",
                "<table><thead><tr><th>Name</th><th>Type</th><th>File</th><th>Line</th></tr></thead>",
                "<tbody>{}</tbody></table>",
                "</main></body></html>\n"
            ),
            css,
            self.compatibility_score(),
            self.files.len(),
            self.subckts.len(),
            self.models.len(),
            self.error_count(),
            self.warning_count(),
            diagnostic_rows,
            subckt_rows,
            model_rows
        )
    }
}

#[derive(Debug)]
pub struct ModelFileSummary {
    pub path: String,
    pub line_count: usize,
}

impl ModelFileSummary {
    fn to_json(&self) -> String {
        format!(
            "    {{ \"path\": \"{}\", \"line_count\": {} }}",
            json_escape(&self.path),
            self.line_count
        )
    }
}

#[derive(Debug)]
pub struct SubcktSummary {
    pub file: String,
    pub line: usize,
    pub name: String,
    pub pins: Vec<String>,
    pub pin_count: usize,
}

impl SubcktSummary {
    fn to_json(&self) -> String {
        format!(
            "    {{ \"file\": \"{}\", \"line\": {}, \"name\": \"{}\", \"pin_count\": {}, \"pins\": [{}] }}",
            json_escape(&self.file),
            self.line,
            json_escape(&self.name),
            self.pin_count,
            quoted_json_list(&self.pins)
        )
    }
}

#[derive(Debug)]
pub struct ModelSummary {
    pub file: String,
    pub line: usize,
    pub name: String,
    pub kind: String,
}

impl ModelSummary {
    fn to_json(&self) -> String {
        format!(
            "    {{ \"file\": \"{}\", \"line\": {}, \"name\": \"{}\", \"kind\": \"{}\" }}",
            json_escape(&self.file),
            self.line,
            json_escape(&self.name),
            json_escape(&self.kind)
        )
    }
}

#[derive(Debug)]
pub struct ModelDiagnostic {
    pub file: String,
    pub line: usize,
    pub severity: DiagnosticSeverity,
    pub code: String,
    pub message: String,
    pub suggestion: String,
}

impl ModelDiagnostic {
    fn to_json(&self) -> String {
        format!(
            concat!(
                "    {{ \"file\": \"{}\", \"line\": {}, \"severity\": \"{}\", ",
                "\"code\": \"{}\", \"message\": \"{}\", \"suggestion\": \"{}\" }}"
            ),
            json_escape(&self.file),
            self.line,
            self.severity.as_str(),
            json_escape(&self.code),
            json_escape(&self.message),
            json_escape(&self.suggestion)
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
}

impl DiagnosticSeverity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Info => "info",
        }
    }

    fn css_class(self) -> &'static str {
        match self {
            Self::Error => "failed",
            Self::Warning => "warning",
            Self::Info => "passed",
        }
    }

    fn rank(self) -> usize {
        match self {
            Self::Error => 0,
            Self::Warning => 1,
            Self::Info => 2,
        }
    }
}

fn scan_model_file(path: &Path, report: &mut ModelCheckReport) -> OslResult<()> {
    let content = read_text(path)?;
    let file = path.display().to_string();
    report.files.push(ModelFileSummary {
        path: file.clone(),
        line_count: content.lines().count(),
    });

    let logical_lines = spice_logical_lines(&content);
    let mut open_subckt: Option<(String, usize)> = None;

    for (line_number, line) in logical_lines {
        let Some(statement) = normalized_spice_statement(&line) else {
            continue;
        };
        let tokens = statement.split_whitespace().collect::<Vec<_>>();
        if tokens.is_empty() {
            continue;
        }

        let directive = tokens[0].to_ascii_lowercase();
        match directive.as_str() {
            ".subckt" => {
                if tokens.len() < 2 {
                    push_model_diagnostic(
                        report,
                        &file,
                        line_number,
                        DiagnosticSeverity::Error,
                        "subckt_missing_name",
                        ".subckt statement is missing a name",
                        "Use '.subckt <name> <pins...>'.",
                    );
                    continue;
                }
                if let Some((name, opened_at)) = &open_subckt {
                    push_model_diagnostic(
                        report,
                        &file,
                        line_number,
                        DiagnosticSeverity::Error,
                        "nested_subckt",
                        &format!(
                            ".subckt '{}' starts before '{}' from line {} is closed",
                            tokens[1], name, opened_at
                        ),
                        "Close the previous subcircuit with '.ends' before starting another.",
                    );
                }
                let pins = tokens
                    .iter()
                    .skip(2)
                    .take_while(|token| !token.contains('='))
                    .map(|token| token.trim_matches(',').to_string())
                    .filter(|pin| !pin.is_empty())
                    .collect::<Vec<_>>();
                if pins.is_empty() {
                    push_model_diagnostic(
                        report,
                        &file,
                        line_number,
                        DiagnosticSeverity::Warning,
                        "subckt_no_pins",
                        &format!(".subckt '{}' has no external pins", tokens[1]),
                        "Confirm the vendor model is complete and that pins were not hidden in a continuation syntax this parser does not support.",
                    );
                }
                report.subckts.push(SubcktSummary {
                    file: file.clone(),
                    line: line_number,
                    name: tokens[1].to_string(),
                    pin_count: pins.len(),
                    pins,
                });
                open_subckt = Some((tokens[1].to_string(), line_number));
            }
            ".ends" | ".endsubckt" => {
                if open_subckt.is_none() {
                    push_model_diagnostic(
                        report,
                        &file,
                        line_number,
                        DiagnosticSeverity::Warning,
                        "orphan_ends",
                        ".ends appears without an open .subckt",
                        "Remove the orphan terminator or check whether the matching .subckt is hidden behind an include.",
                    );
                }
                open_subckt = None;
            }
            ".model" => {
                if tokens.len() < 3 {
                    push_model_diagnostic(
                        report,
                        &file,
                        line_number,
                        DiagnosticSeverity::Error,
                        "model_malformed",
                        ".model statement should include a name and model type",
                        "Use '.model <name> <type>(...)' or '.model <name> <type> ...'.",
                    );
                    continue;
                }
                report.models.push(ModelSummary {
                    file: file.clone(),
                    line: line_number,
                    name: tokens[1].to_string(),
                    kind: parse_model_kind(tokens[2]),
                });
            }
            ".include" | ".inc" | ".lib" => {
                push_model_diagnostic(
                    report,
                    &file,
                    line_number,
                    DiagnosticSeverity::Info,
                    "external_dependency",
                    &format!("{} depends on an external model file", tokens[0]),
                    "Keep imported library paths reproducible relative to the project root.",
                );
            }
            ".func" => {
                push_model_diagnostic(
                    report,
                    &file,
                    line_number,
                    DiagnosticSeverity::Warning,
                    "function_dialect",
                    ".func syntax varies across SPICE dialects",
                    "Verify this function in ngspice, especially if the model came from LTspice or PSpice.",
                );
            }
            ".param" | ".options" | ".option" | ".temp" | ".global" | ".nodeset" | ".ic" => {}
            ".tran" | ".ac" | ".dc" | ".op" | ".control" | ".endc" | ".end" => {}
            ".probe" | ".plot" | ".print" | ".meas" | ".measure" | ".save" => {}
            ".protect" | ".unprotect" | ".endl" | ".alter" | ".step" | ".libstep" => {
                push_model_diagnostic(
                    report,
                    &file,
                    line_number,
                    DiagnosticSeverity::Warning,
                    "unsupported_directive",
                    &format!("{} is commonly unsupported or dialect-specific", tokens[0]),
                    "Convert this directive to an ngspice-compatible equivalent before automated verification.",
                );
            }
            _ if directive.starts_with('.') => {
                push_model_diagnostic(
                    report,
                    &file,
                    line_number,
                    DiagnosticSeverity::Warning,
                    "unknown_directive",
                    &format!(
                        "{} is not recognized by the current NekoSpice checker",
                        tokens[0]
                    ),
                    "Check ngspice compatibility and add an explicit support rule if this directive is expected.",
                );
            }
            _ => {
                detect_instance_risks(&file, line_number, &statement, report);
            }
        }
    }

    if let Some((name, opened_at)) = open_subckt {
        push_model_diagnostic(
            report,
            &file,
            opened_at,
            DiagnosticSeverity::Error,
            "unclosed_subckt",
            &format!(".subckt '{}' is missing a matching .ends", name),
            "Add '.ends' after the subcircuit body.",
        );
    }

    Ok(())
}

fn push_model_diagnostic(
    report: &mut ModelCheckReport,
    file: &str,
    line: usize,
    severity: DiagnosticSeverity,
    code: &str,
    message: &str,
    suggestion: &str,
) {
    report.diagnostics.push(ModelDiagnostic {
        file: file.to_string(),
        line,
        severity,
        code: code.to_string(),
        message: message.to_string(),
        suggestion: suggestion.to_string(),
    });
}

fn parse_model_kind(token: &str) -> String {
    token
        .split_once('(')
        .map(|(kind, _)| kind)
        .unwrap_or(token)
        .to_string()
}

fn detect_instance_risks(
    file: &str,
    line_number: usize,
    statement: &str,
    report: &mut ModelCheckReport,
) {
    let lowered = statement.to_ascii_lowercase();
    if lowered.contains("table(") || lowered.contains("tbl(") {
        push_model_diagnostic(
            report,
            file,
            line_number,
            DiagnosticSeverity::Warning,
            "behavioral_table",
            "behavioral table syntax is dialect-sensitive",
            "Confirm the expression grammar and table interpolation are accepted by ngspice.",
        );
    }
    if lowered.contains("limit(") || lowered.contains("uplim(") || lowered.contains("dnlim(") {
        push_model_diagnostic(
            report,
            file,
            line_number,
            DiagnosticSeverity::Warning,
            "behavioral_function",
            "behavioral limiting functions often differ between LTspice, PSpice, and ngspice",
            "Replace unsupported functions or add a compatibility shim before verification.",
        );
    }
}

fn quoted_json_list(values: &[String]) -> String {
    values
        .iter()
        .map(|value| format!("\"{}\"", json_escape(value)))
        .collect::<Vec<_>>()
        .join(", ")
}

fn find_model_files(root: &Path) -> OslResult<Vec<PathBuf>> {
    let mut files = Vec::new();
    if root.is_file() {
        files.push(root.to_path_buf());
        return Ok(files);
    }

    find_model_files_inner(root, &mut files)?;
    files.sort();
    Ok(files)
}

fn find_model_files_inner(path: &Path, files: &mut Vec<PathBuf>) -> OslResult<()> {
    for entry in
        fs::read_dir(path).map_err(|err| OslError::io(format!("read {}", path.display()), err))?
    {
        let entry = entry.map_err(|err| OslError::io("read directory entry", err))?;
        let entry_path = entry.path();
        if entry_path.is_dir() {
            find_model_files_inner(&entry_path, files)?;
        } else if is_model_file(&entry_path) {
            files.push(entry_path);
        }
    }
    Ok(())
}

fn is_model_file(path: &Path) -> bool {
    let Some(extension) = path.extension().and_then(|extension| extension.to_str()) else {
        return false;
    };
    matches!(
        extension.to_ascii_lowercase().as_str(),
        "cir" | "sp" | "spice" | "lib" | "mod" | "mdl" | "sub" | "subckt"
    )
}

fn spice_logical_lines(content: &str) -> Vec<(usize, String)> {
    let mut logical_lines = Vec::new();
    let mut current = None::<(usize, String)>;

    for (index, raw_line) in content.lines().enumerate() {
        let line_number = index + 1;
        let trimmed_start = raw_line.trim_start();
        if trimmed_start.starts_with('+') {
            let continuation = trimmed_start.trim_start_matches('+').trim();
            if let Some((_, line)) = current.as_mut() {
                line.push(' ');
                line.push_str(continuation);
            } else {
                current = Some((line_number, continuation.to_string()));
            }
            continue;
        }

        if let Some(line) = current.take() {
            logical_lines.push(line);
        }
        current = Some((line_number, raw_line.to_string()));
    }

    if let Some(line) = current {
        logical_lines.push(line);
    }

    logical_lines
}

fn normalized_spice_statement(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('*') || trimmed.starts_with(';') {
        return None;
    }
    let without_inline_comment = trimmed
        .split_once(';')
        .map(|(statement, _)| statement)
        .unwrap_or(trimmed)
        .trim();
    if without_inline_comment.is_empty() {
        None
    } else {
        Some(without_inline_comment.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::{normalized_spice_statement, parse_model_kind, spice_logical_lines};

    #[test]
    fn extracts_model_kind_before_parameters() {
        assert_eq!(parse_model_kind("D(Is=2.52n"), "D");
        assert_eq!(parse_model_kind("npn"), "npn");
    }

    #[test]
    fn folds_spice_continuation_lines() {
        let lines = spice_logical_lines(".subckt demo in out\n+ vcc vee\n.ends\n");

        assert_eq!(lines[0], (1, ".subckt demo in out vcc vee".to_string()));
        assert_eq!(lines[1], (3, ".ends".to_string()));
    }

    #[test]
    fn strips_spice_comments() {
        assert_eq!(normalized_spice_statement("* comment"), None);
        assert_eq!(
            normalized_spice_statement("R1 in out 1k ; load").unwrap(),
            "R1 in out 1k"
        );
    }
}
