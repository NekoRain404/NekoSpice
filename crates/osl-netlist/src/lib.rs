use osl_core::{OslResult, html_escape, json_escape, read_text};
use std::path::Path;

#[derive(Debug)]
pub struct ImportReport {
    pub source: String,
    pub flavor: NetlistFlavor,
    pub line_count: usize,
    pub components: Vec<ComponentSummary>,
    pub directives: Vec<DirectiveSummary>,
    pub includes: Vec<IncludeSummary>,
    pub diagnostics: Vec<ImportDiagnostic>,
}

impl ImportReport {
    pub fn parse(path: &Path) -> OslResult<Self> {
        let content = read_text(path)?;
        parse_netlist(&content, &path.display().to_string())
    }

    pub fn component_count(&self) -> usize {
        self.components.len()
    }

    pub fn symbol_count(&self) -> usize {
        self.components
            .iter()
            .filter(|component| component.reference != ".control")
            .count()
    }

    pub fn directive_count(&self) -> usize {
        self.directives.len()
    }

    pub fn error_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == ImportSeverity::Error)
            .count()
    }

    pub fn warning_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == ImportSeverity::Warning)
            .count()
    }

    pub fn compatibility_score(&self) -> u32 {
        let penalty = self.error_count() as u32 * 25 + self.warning_count() as u32 * 8;
        100_u32.saturating_sub(penalty)
    }

    pub fn to_json(&self) -> String {
        let components = self
            .components
            .iter()
            .map(ComponentSummary::to_json)
            .collect::<Vec<_>>()
            .join(",\n");
        let directives = self
            .directives
            .iter()
            .map(DirectiveSummary::to_json)
            .collect::<Vec<_>>()
            .join(",\n");
        let includes = self
            .includes
            .iter()
            .map(IncludeSummary::to_json)
            .collect::<Vec<_>>()
            .join(",\n");
        let diagnostics = self
            .diagnostics
            .iter()
            .map(ImportDiagnostic::to_json)
            .collect::<Vec<_>>()
            .join(",\n");

        format!(
            concat!(
                "{{\n",
                "  \"schema_version\": 1,\n",
                "  \"source\": \"{}\",\n",
                "  \"flavor\": \"{}\",\n",
                "  \"compatibility_score\": {},\n",
                "  \"line_count\": {},\n",
                "  \"component_count\": {},\n",
                "  \"symbol_count\": {},\n",
                "  \"directive_count\": {},\n",
                "  \"include_count\": {},\n",
                "  \"errors\": {},\n",
                "  \"warnings\": {},\n",
                "  \"components\": [\n",
                "{}\n",
                "  ],\n",
                "  \"directives\": [\n",
                "{}\n",
                "  ],\n",
                "  \"includes\": [\n",
                "{}\n",
                "  ],\n",
                "  \"diagnostics\": [\n",
                "{}\n",
                "  ]\n",
                "}}\n"
            ),
            json_escape(&self.source),
            self.flavor.as_str(),
            self.compatibility_score(),
            self.line_count,
            self.component_count(),
            self.symbol_count(),
            self.directive_count(),
            self.includes.len(),
            self.error_count(),
            self.warning_count(),
            components,
            directives,
            includes,
            diagnostics
        )
    }

    pub fn to_html(&self, css: &str) -> String {
        let component_rows = if self.components.is_empty() {
            "<tr><td colspan=\"6\">No components found.</td></tr>".to_string()
        } else {
            self.components
                .iter()
                .map(|component| {
                    format!(
                        "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                        html_escape(&component.reference),
                        html_escape(component.kind.as_str()),
                        component.line,
                        html_escape(&component.nodes.join(", ")),
                        html_escape(&component.value.clone().unwrap_or_else(|| "none".to_string())),
                        html_escape(&component.model.clone().unwrap_or_else(|| "none".to_string()))
                    )
                })
                .collect::<String>()
        };
        let directive_rows = if self.directives.is_empty() {
            "<tr><td colspan=\"3\">No directives found.</td></tr>".to_string()
        } else {
            self.directives
                .iter()
                .map(|directive| {
                    format!(
                        "<tr><td>{}</td><td>{}</td><td>{}</td></tr>",
                        directive.line,
                        html_escape(&directive.name),
                        html_escape(&directive.text)
                    )
                })
                .collect::<String>()
        };
        let diagnostic_rows = if self.diagnostics.is_empty() {
            "<tr><td colspan=\"5\">No diagnostics.</td></tr>".to_string()
        } else {
            self.diagnostics
                .iter()
                .map(|diagnostic| {
                    format!(
                        "<tr class=\"{}\"><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                        html_escape(diagnostic.severity.css_class()),
                        html_escape(diagnostic.severity.as_str()),
                        diagnostic.line,
                        html_escape(&diagnostic.code),
                        html_escape(&diagnostic.message),
                        html_escape(&diagnostic.suggestion)
                    )
                })
                .collect::<String>()
        };

        format!(
            concat!(
                "<!doctype html><html><head><meta charset=\"utf-8\">",
                "<title>NekoSpice Import Report</title>{}</head><body>",
                "<main><h1>Import Report</h1>",
                "<section class=\"summary\"><strong>Score:</strong> {} <strong>Flavor:</strong> {} ",
                "<strong>Components:</strong> {} <strong>Symbols:</strong> {} ",
                "<strong>Directives:</strong> {} <strong>Warnings:</strong> {}</section>",
                "<h2>Normalized Project</h2>",
                "<ul><li><a href=\"project/project.osl.yaml\">project.osl.yaml</a></li>",
                "<li><a href=\"project/input.cir\">input.cir</a></li>",
                "<li><a href=\"project/manifest.json\">manifest.json</a></li></ul>",
                "<h2>Diagnostics</h2>",
                "<table><thead><tr><th>Severity</th><th>Line</th><th>Code</th><th>Message</th><th>Suggestion</th></tr></thead><tbody>{}</tbody></table>",
                "<h2>Components</h2>",
                "<table><thead><tr><th>Ref</th><th>Kind</th><th>Line</th><th>Nodes</th><th>Value</th><th>Model</th></tr></thead><tbody>{}</tbody></table>",
                "<h2>Directives</h2>",
                "<table><thead><tr><th>Line</th><th>Name</th><th>Text</th></tr></thead><tbody>{}</tbody></table>",
                "</main></body></html>\n"
            ),
            css,
            self.compatibility_score(),
            self.flavor.as_str(),
            self.component_count(),
            self.symbol_count(),
            self.directive_count(),
            self.warning_count(),
            diagnostic_rows,
            component_rows,
            directive_rows
        )
    }

    pub fn normalized_project(&self, source_netlist: &str) -> NormalizedImportProject {
        let project_name = normalized_project_name(&self.source);
        let netlist_path = "input.cir".to_string();
        let validation_path = "project.osl.yaml".to_string();
        let manifest_path = "manifest.json".to_string();
        let run_name = sanitize_identifier(&project_name);
        let normalized_netlist = normalize_imported_netlist(source_netlist);
        let validation_yaml = format!(
            concat!(
                "project: {}\n",
                "\n",
                "runs:\n",
                "  - name: {}\n",
                "    netlist: {}\n",
                "    checks: []\n"
            ),
            yaml_scalar(&project_name),
            yaml_scalar(&run_name),
            yaml_scalar(&netlist_path)
        );
        let manifest_json = format!(
            concat!(
                "{{\n",
                "  \"schema_version\": 1,\n",
                "  \"project\": \"{}\",\n",
                "  \"source\": \"{}\",\n",
                "  \"flavor\": \"{}\",\n",
                "  \"compatibility_score\": {},\n",
                "  \"netlist\": \"{}\",\n",
                "  \"validation\": \"{}\",\n",
                "  \"import_report\": \"../import.json\",\n",
                "  \"component_count\": {},\n",
                "  \"symbol_count\": {},\n",
                "  \"directive_count\": {},\n",
                "  \"include_count\": {},\n",
                "  \"errors\": {},\n",
                "  \"warnings\": {}\n",
                "}}\n"
            ),
            json_escape(&project_name),
            json_escape(&self.source),
            self.flavor.as_str(),
            self.compatibility_score(),
            json_escape(&netlist_path),
            json_escape(&validation_path),
            self.component_count(),
            self.symbol_count(),
            self.directive_count(),
            self.includes.len(),
            self.error_count(),
            self.warning_count()
        );

        NormalizedImportProject {
            project_name,
            netlist_path,
            validation_path,
            manifest_path,
            netlist: normalized_netlist,
            validation_yaml,
            manifest_json,
        }
    }
}

#[derive(Debug, Clone)]
pub struct NormalizedImportProject {
    pub project_name: String,
    pub netlist_path: String,
    pub validation_path: String,
    pub manifest_path: String,
    pub netlist: String,
    pub validation_yaml: String,
    pub manifest_json: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetlistFlavor {
    KiCad,
    Ltspice,
    GenericSpice,
}

impl NetlistFlavor {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::KiCad => "kicad",
            Self::Ltspice => "ltspice",
            Self::GenericSpice => "generic-spice",
        }
    }
}

#[derive(Debug)]
pub struct ComponentSummary {
    pub line: usize,
    pub reference: String,
    pub kind: ComponentKind,
    pub nodes: Vec<String>,
    pub value: Option<String>,
    pub model: Option<String>,
}

impl ComponentSummary {
    fn to_json(&self) -> String {
        format!(
            concat!(
                "    {{ \"line\": {}, \"reference\": \"{}\", \"kind\": \"{}\", ",
                "\"nodes\": [{}], \"value\": {}, \"model\": {} }}"
            ),
            self.line,
            json_escape(&self.reference),
            self.kind.as_str(),
            quoted_json_list(&self.nodes),
            option_string_json(self.value.as_deref()),
            option_string_json(self.model.as_deref())
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentKind {
    Resistor,
    Capacitor,
    Inductor,
    VoltageSource,
    CurrentSource,
    Diode,
    Bjt,
    Mosfet,
    Jfet,
    Subcircuit,
    Behavioral,
    Other,
}

impl ComponentKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Resistor => "resistor",
            Self::Capacitor => "capacitor",
            Self::Inductor => "inductor",
            Self::VoltageSource => "voltage-source",
            Self::CurrentSource => "current-source",
            Self::Diode => "diode",
            Self::Bjt => "bjt",
            Self::Mosfet => "mosfet",
            Self::Jfet => "jfet",
            Self::Subcircuit => "subcircuit",
            Self::Behavioral => "behavioral",
            Self::Other => "other",
        }
    }
}

#[derive(Debug)]
pub struct DirectiveSummary {
    pub line: usize,
    pub name: String,
    pub text: String,
}

impl DirectiveSummary {
    fn to_json(&self) -> String {
        format!(
            "    {{ \"line\": {}, \"name\": \"{}\", \"text\": \"{}\" }}",
            self.line,
            json_escape(&self.name),
            json_escape(&self.text)
        )
    }
}

#[derive(Debug)]
pub struct IncludeSummary {
    pub line: usize,
    pub path: String,
}

impl IncludeSummary {
    fn to_json(&self) -> String {
        format!(
            "    {{ \"line\": {}, \"path\": \"{}\" }}",
            self.line,
            json_escape(&self.path)
        )
    }
}

#[derive(Debug)]
pub struct ImportDiagnostic {
    pub line: usize,
    pub severity: ImportSeverity,
    pub code: String,
    pub message: String,
    pub suggestion: String,
}

impl ImportDiagnostic {
    fn to_json(&self) -> String {
        format!(
            concat!(
                "    {{ \"line\": {}, \"severity\": \"{}\", \"code\": \"{}\", ",
                "\"message\": \"{}\", \"suggestion\": \"{}\" }}"
            ),
            self.line,
            self.severity.as_str(),
            json_escape(&self.code),
            json_escape(&self.message),
            json_escape(&self.suggestion)
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportSeverity {
    Error,
    Warning,
    Info,
}

impl ImportSeverity {
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
}

pub fn parse_netlist(input: &str, source: &str) -> OslResult<ImportReport> {
    let mut report = ImportReport {
        source: source.to_string(),
        flavor: detect_flavor(input),
        line_count: input.lines().count(),
        components: Vec::new(),
        directives: Vec::new(),
        includes: Vec::new(),
        diagnostics: Vec::new(),
    };

    for (line_number, line) in spice_logical_lines(input) {
        let Some(statement) = normalized_spice_statement(&line) else {
            continue;
        };
        if statement.starts_with('.') {
            parse_directive(line_number, &statement, &mut report);
        } else {
            parse_component(line_number, &statement, &mut report);
        }
    }

    if report.components.is_empty() {
        push_diagnostic(
            &mut report,
            1,
            ImportSeverity::Warning,
            "no_components",
            "netlist contains no component instances",
            "Check whether the file is a model library instead of a runnable netlist.",
        );
    }
    if !report.directives.iter().any(|directive| {
        matches!(
            directive.name.as_str(),
            ".tran" | ".ac" | ".dc" | ".op" | ".control"
        )
    }) {
        push_diagnostic(
            &mut report,
            1,
            ImportSeverity::Warning,
            "missing_analysis",
            "netlist has no obvious analysis directive",
            "Add .tran, .ac, .dc, .op, or a .control block before running simulation.",
        );
    }

    Ok(report)
}

fn parse_directive(line: usize, statement: &str, report: &mut ImportReport) {
    let name = statement
        .split_whitespace()
        .next()
        .unwrap_or(statement)
        .to_ascii_lowercase();
    report.directives.push(DirectiveSummary {
        line,
        name: name.clone(),
        text: statement.to_string(),
    });

    match name.as_str() {
        ".include" | ".inc" | ".lib" => {
            if let Some(path) = statement.split_whitespace().nth(1) {
                report.includes.push(IncludeSummary {
                    line,
                    path: path.trim_matches('"').to_string(),
                });
            }
        }
        ".step" | ".protect" | ".unprotect" | ".alter" => push_diagnostic(
            report,
            line,
            ImportSeverity::Warning,
            "dialect_directive",
            &format!(
                "{} is dialect-specific and may not run as-is in ngspice",
                name
            ),
            "Normalize this directive during import or move it into a verification sweep.",
        ),
        ".end" | ".tran" | ".ac" | ".dc" | ".op" | ".control" | ".endc" | ".model" | ".subckt"
        | ".ends" | ".param" | ".options" | ".option" => {}
        _ => push_diagnostic(
            report,
            line,
            ImportSeverity::Info,
            "unknown_directive",
            &format!("{} is not classified by the importer yet", name),
            "Keep this directive in the import report for compatibility review.",
        ),
    }
}

fn parse_component(line: usize, statement: &str, report: &mut ImportReport) {
    let tokens = statement.split_whitespace().collect::<Vec<_>>();
    if tokens.is_empty() {
        return;
    }
    let reference = tokens[0].to_string();
    let kind = component_kind(&reference);
    let (nodes, value, model, min_pin_count) = match kind {
        ComponentKind::Subcircuit => {
            let instance_tokens = tokens.iter().skip(1).copied().collect::<Vec<_>>();
            let model = instance_tokens.last().map(|model| model.to_string());
            let nodes = instance_tokens
                .iter()
                .take(instance_tokens.len().saturating_sub(1))
                .map(|node| node.to_string())
                .collect::<Vec<_>>();
            (nodes, model.clone(), model, 1)
        }
        _ => {
            let pin_count = expected_pin_count(kind);
            let nodes = tokens
                .iter()
                .skip(1)
                .take(pin_count)
                .map(|token| token.to_string())
                .collect::<Vec<_>>();
            let value = tokens.get(1 + pin_count).map(|value| value.to_string());
            let model = match kind {
                ComponentKind::Diode
                | ComponentKind::Bjt
                | ComponentKind::Mosfet
                | ComponentKind::Jfet => value.clone(),
                _ => None,
            };
            (nodes, value, model, pin_count)
        }
    };

    if nodes.len() < min_pin_count {
        push_diagnostic(
            report,
            line,
            ImportSeverity::Error,
            "component_too_few_nodes",
            &format!(
                "{} expects at least {} nodes but only {} were found",
                reference,
                min_pin_count,
                nodes.len()
            ),
            "Check the exported netlist line and symbol pin mapping.",
        );
    }

    report.components.push(ComponentSummary {
        line,
        reference,
        kind,
        nodes,
        value,
        model,
    });
}

fn push_diagnostic(
    report: &mut ImportReport,
    line: usize,
    severity: ImportSeverity,
    code: &str,
    message: &str,
    suggestion: &str,
) {
    report.diagnostics.push(ImportDiagnostic {
        line,
        severity,
        code: code.to_string(),
        message: message.to_string(),
        suggestion: suggestion.to_string(),
    });
}

fn detect_flavor(input: &str) -> NetlistFlavor {
    let lowered = input.to_ascii_lowercase();
    if lowered.contains("eeschema") || lowered.contains("kicad") {
        NetlistFlavor::KiCad
    } else if lowered.contains("ltspice") {
        NetlistFlavor::Ltspice
    } else {
        NetlistFlavor::GenericSpice
    }
}

fn component_kind(reference: &str) -> ComponentKind {
    match reference
        .chars()
        .next()
        .map(|character| character.to_ascii_uppercase())
    {
        Some('R') => ComponentKind::Resistor,
        Some('C') => ComponentKind::Capacitor,
        Some('L') => ComponentKind::Inductor,
        Some('V') => ComponentKind::VoltageSource,
        Some('I') => ComponentKind::CurrentSource,
        Some('D') => ComponentKind::Diode,
        Some('Q') => ComponentKind::Bjt,
        Some('M') => ComponentKind::Mosfet,
        Some('J') => ComponentKind::Jfet,
        Some('X') => ComponentKind::Subcircuit,
        Some('B') | Some('E') | Some('G') | Some('F') | Some('H') => ComponentKind::Behavioral,
        _ => ComponentKind::Other,
    }
}

fn expected_pin_count(kind: ComponentKind) -> usize {
    match kind {
        ComponentKind::Bjt | ComponentKind::Jfet => 3,
        ComponentKind::Mosfet => 4,
        ComponentKind::Subcircuit => 2,
        _ => 2,
    }
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

fn quoted_json_list(values: &[String]) -> String {
    values
        .iter()
        .map(|value| format!("\"{}\"", json_escape(value)))
        .collect::<Vec<_>>()
        .join(", ")
}

fn option_string_json(value: Option<&str>) -> String {
    value
        .map(|value| format!("\"{}\"", json_escape(value)))
        .unwrap_or_else(|| "null".to_string())
}

fn normalized_project_name(source: &str) -> String {
    Path::new(source)
        .file_stem()
        .and_then(|name| name.to_str())
        .map(sanitize_identifier)
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "imported_project".to_string())
}

fn sanitize_identifier(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    for character in input.chars() {
        if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
            output.push(character);
        } else {
            output.push('_');
        }
    }
    if output.is_empty() {
        "imported_project".to_string()
    } else {
        output
    }
}

fn normalize_imported_netlist(source: &str) -> String {
    let mut output = source.trim_end().to_string();
    output.push('\n');
    output
}

fn yaml_scalar(value: &str) -> String {
    if value.chars().all(|character| {
        character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.' | '/')
    }) && !value.is_empty()
    {
        value.to_string()
    } else {
        format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
    }
}

#[cfg(test)]
mod tests {
    use super::{ComponentKind, NetlistFlavor, parse_netlist};

    #[test]
    fn parses_kicad_style_netlist_summary() {
        let input = r#"
* KiCad Eeschema generated SPICE netlist
.include "models.lib"
V1 in 0 DC 5
R1 in out 1k
C1 out 0 10n
XU1 in out vcc vee GOODAMP
.tran 1u 1m
.end
"#;

        let report = parse_netlist(input, "demo.cir").unwrap();

        assert_eq!(report.flavor, NetlistFlavor::KiCad);
        assert_eq!(report.component_count(), 4);
        assert_eq!(report.directive_count(), 3);
        assert_eq!(report.includes[0].path, "models.lib");
        assert_eq!(report.components[3].kind, ComponentKind::Subcircuit);
        assert_eq!(report.components[3].nodes, ["in", "out", "vcc", "vee"]);
        assert_eq!(report.components[3].model.as_deref(), Some("GOODAMP"));
        assert_eq!(report.error_count(), 0);
    }

    #[test]
    fn reports_missing_analysis() {
        let report = parse_netlist("R1 in out 1k\n", "missing.cir").unwrap();

        assert_eq!(report.warning_count(), 1);
        assert_eq!(report.diagnostics[0].code, "missing_analysis");
    }

    #[test]
    fn builds_normalized_import_project() {
        let source = "* KiCad netlist\nV1 in 0 DC 5\nR1 in out 1k\n.tran 1u 1m\n.end\n";
        let report = parse_netlist(source, "examples/kicad_import/kicad_rc.cir").unwrap();

        let project = report.normalized_project(source);

        assert_eq!(project.project_name, "kicad_rc");
        assert_eq!(project.netlist_path, "input.cir");
        assert!(project.netlist.ends_with('\n'));
        assert!(project.validation_yaml.contains("project: kicad_rc"));
        assert!(project.validation_yaml.contains("netlist: input.cir"));
        assert!(project.manifest_json.contains("\"flavor\": \"kicad\""));
        assert!(
            project
                .manifest_json
                .contains("\"validation\": \"project.osl.yaml\"")
        );
    }
}
