use osl_core::{OslResult, html_escape, json_escape, read_text};
use std::collections::{BTreeMap, BTreeSet};
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
        self.normalized_project_with_dependencies(source_netlist, &[])
    }

    pub fn normalized_project_with_dependencies(
        &self,
        source_netlist: &str,
        dependencies: &[NormalizedDependency],
    ) -> NormalizedImportProject {
        let project_name = normalized_project_name(&self.source);
        let netlist_path = "input.cir".to_string();
        let validation_path = "project.osl.yaml".to_string();
        let manifest_path = "manifest.json".to_string();
        let run_name = sanitize_identifier(&project_name);
        let normalized_netlist = normalize_imported_netlist(source_netlist, dependencies);
        let dependencies_json = dependencies
            .iter()
            .map(NormalizedDependency::to_json)
            .collect::<Vec<_>>()
            .join(",\n");
        let suggested_signals = self.suggested_signals();
        let suggested_checks = self.suggested_checks_from_signals(&suggested_signals);
        let suggested_signals_json = suggested_signals
            .iter()
            .map(SuggestedSignal::to_json)
            .collect::<Vec<_>>()
            .join(",\n");
        let suggested_checks_json = suggested_checks
            .iter()
            .map(SuggestedCheck::to_json)
            .collect::<Vec<_>>()
            .join(",\n");
        let suggested_checks_yaml = suggested_checks_yaml(&suggested_checks);
        let validation_yaml = format!(
            concat!(
                "project: {}\n",
                "\n",
                "runs:\n",
                "  - name: {}\n",
                "    netlist: {}\n",
                "    checks: []\n",
                "{}"
            ),
            yaml_scalar(&project_name),
            yaml_scalar(&run_name),
            yaml_scalar(&netlist_path),
            suggested_checks_yaml
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
                "  \"warnings\": {},\n",
                "  \"suggested_signals\": [\n",
                "{}\n",
                "  ],\n",
                "  \"suggested_checks\": [\n",
                "{}\n",
                "  ],\n",
                "  \"dependencies\": [\n",
                "{}\n",
                "  ]\n",
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
            self.warning_count(),
            suggested_signals_json,
            suggested_checks_json,
            dependencies_json
        );

        NormalizedImportProject {
            project_name,
            netlist_path,
            validation_path,
            manifest_path,
            netlist: normalized_netlist,
            validation_yaml,
            manifest_json,
            dependencies: dependencies.to_vec(),
        }
    }

    pub fn suggested_signals(&self) -> Vec<SuggestedSignal> {
        let mut voltage_signals = BTreeMap::new();
        let mut source_current_signals = BTreeMap::new();

        for component in &self.components {
            for node in &component.nodes {
                let normalized_node = normalize_signal_node(node);
                if normalized_node.is_empty() || is_ground_node(&normalized_node) {
                    continue;
                }
                voltage_signals
                    .entry(format!("v({normalized_node})"))
                    .or_insert_with(|| format!("node voltage {normalized_node}"));
            }

            if component.kind == ComponentKind::VoltageSource {
                let reference = component.reference.trim().to_ascii_lowercase();
                if !reference.is_empty() {
                    source_current_signals
                        .entry(format!("i({reference})"))
                        .or_insert_with(|| format!("current through {}", component.reference));
                }
            }
        }

        voltage_signals
            .into_iter()
            .chain(source_current_signals)
            .map(|(signal, source)| SuggestedSignal { signal, source })
            .collect()
    }

    pub fn suggested_checks(&self) -> Vec<SuggestedCheck> {
        let signals = self.suggested_signals();
        self.suggested_checks_from_signals(&signals)
    }

    fn suggested_checks_from_signals(&self, signals: &[SuggestedSignal]) -> Vec<SuggestedCheck> {
        let analysis = self.primary_analysis_kind();
        let mut ordered_signals = signals.iter().collect::<Vec<_>>();
        ordered_signals.sort_by(|left, right| {
            suggested_signal_priority(&left.signal).cmp(&suggested_signal_priority(&right.signal))
        });

        let mut names = BTreeSet::new();
        let mut checks = Vec::new();
        for signal in ordered_signals.into_iter().take(8) {
            let kind = suggested_check_kind(analysis, &signal.signal);
            let name = suggested_check_name(&signal.signal, kind);
            if !names.insert(name.clone()) {
                continue;
            }
            checks.push(SuggestedCheck {
                name,
                kind: kind.to_string(),
                signal: signal.signal.clone(),
                reason: format!(
                    "Template derived from {} import signal; set min/max after the first run.",
                    analysis.as_str()
                ),
            });
        }
        checks
    }

    fn primary_analysis_kind(&self) -> AnalysisKind {
        self.directives
            .iter()
            .find_map(|directive| AnalysisKind::from_directive(&directive.name))
            .unwrap_or(AnalysisKind::Unknown)
    }
}

#[derive(Debug, Clone)]
pub struct NormalizedDependency {
    pub source: String,
    pub project_path: String,
}

impl NormalizedDependency {
    pub fn to_json(&self) -> String {
        format!(
            "    {{ \"source\": \"{}\", \"project_path\": \"{}\" }}",
            json_escape(&self.source),
            json_escape(&self.project_path)
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuggestedSignal {
    pub signal: String,
    pub source: String,
}

impl SuggestedSignal {
    fn to_json(&self) -> String {
        format!(
            "    {{ \"signal\": \"{}\", \"source\": \"{}\" }}",
            json_escape(&self.signal),
            json_escape(&self.source)
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuggestedCheck {
    pub name: String,
    pub kind: String,
    pub signal: String,
    pub reason: String,
}

impl SuggestedCheck {
    fn to_json(&self) -> String {
        format!(
            concat!(
                "    {{ \"name\": \"{}\", \"kind\": \"{}\", \"signal\": \"{}\", ",
                "\"min\": null, \"max\": null, \"reason\": \"{}\" }}"
            ),
            json_escape(&self.name),
            json_escape(&self.kind),
            json_escape(&self.signal),
            json_escape(&self.reason)
        )
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
    pub dependencies: Vec<NormalizedDependency>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AnalysisKind {
    Transient,
    OperatingPoint,
    Ac,
    Dc,
    Unknown,
}

impl AnalysisKind {
    fn from_directive(name: &str) -> Option<Self> {
        match name {
            ".tran" => Some(Self::Transient),
            ".op" => Some(Self::OperatingPoint),
            ".ac" => Some(Self::Ac),
            ".dc" => Some(Self::Dc),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Transient => "transient",
            Self::OperatingPoint => "operating-point",
            Self::Ac => "ac",
            Self::Dc => "dc",
            Self::Unknown => "unknown-analysis",
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

fn normalize_signal_node(node: &str) -> String {
    node.trim()
        .trim_matches('"')
        .trim_matches('\'')
        .to_ascii_lowercase()
}

fn is_ground_node(node: &str) -> bool {
    matches!(node, "0" | "gnd" | "agnd" | "dgnd")
}

fn suggested_signal_priority(signal: &str) -> (u8, String) {
    let lowered = signal.to_ascii_lowercase();
    let rank = if lowered == "v(out)" {
        0
    } else if lowered.contains("out") {
        1
    } else if lowered == "v(in)" {
        2
    } else if lowered.starts_with("v(") {
        3
    } else if lowered.starts_with("i(") {
        4
    } else {
        5
    };
    (rank, lowered)
}

fn suggested_check_kind(analysis: AnalysisKind, signal: &str) -> &'static str {
    let is_current = signal.to_ascii_lowercase().starts_with("i(");
    match (analysis, is_current) {
        (AnalysisKind::Transient, true) => "rms",
        (AnalysisKind::Transient, false) => "avg",
        (AnalysisKind::Ac, _) => "max",
        _ => "final_value",
    }
}

fn suggested_check_name(signal: &str, kind: &str) -> String {
    format!("{}_{}", signal_name_slug(signal), kind)
}

fn signal_name_slug(signal: &str) -> String {
    signal
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .split('_')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}

fn suggested_checks_yaml(checks: &[SuggestedCheck]) -> String {
    if checks.is_empty() {
        return concat!(
            "    # Suggested checks: no observable node or source-current signals were found.\n",
            "    # Add checks after confirming the imported netlist produces waveform data.\n"
        )
        .to_string();
    }

    let mut output = concat!(
        "    # Suggested checks to customize after the first run:\n",
        "    #   Keep checks empty for import smoke tests, then copy entries below\n",
        "    #   into checks with tuned min/max limits once waveform-summary.json is known.\n"
    )
    .to_string();
    for check in checks {
        output.push_str(&format!("    #   - name: {}\n", yaml_scalar(&check.name)));
        output.push_str(&format!("    #     kind: {}\n", yaml_scalar(&check.kind)));
        output.push_str(&format!(
            "    #     signal: {}\n",
            yaml_scalar(&check.signal)
        ));
        output.push_str("    #     min: TODO\n");
        output.push_str("    #     max: TODO\n");
    }
    output
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

fn normalize_imported_netlist(source: &str, dependencies: &[NormalizedDependency]) -> String {
    let mut output = source
        .lines()
        .map(|line| rewrite_dependency_line(line, dependencies))
        .collect::<Vec<_>>()
        .join("\n");
    output.push('\n');
    output
}

fn rewrite_dependency_line(line: &str, dependencies: &[NormalizedDependency]) -> String {
    let trimmed_start = line.trim_start();
    let indent_len = line.len() - trimmed_start.len();
    let indent = &line[..indent_len];
    let tokens = trimmed_start.split_whitespace().collect::<Vec<_>>();
    if tokens.len() < 2 {
        return line.to_string();
    }
    let directive = tokens[0].to_ascii_lowercase();
    if !matches!(directive.as_str(), ".include" | ".inc" | ".lib") {
        return line.to_string();
    }

    let raw_path = tokens[1];
    let path = raw_path.trim_matches('"').trim_matches('\'');
    let Some(dependency) = dependencies
        .iter()
        .find(|dependency| dependency.source == path)
    else {
        return line.to_string();
    };

    let suffix = tokens
        .iter()
        .skip(2)
        .map(|token| format!(" {}", token))
        .collect::<String>();
    format!(
        "{}{} \"{}\"{}",
        indent, tokens[0], dependency.project_path, suffix
    )
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
    use super::{ComponentKind, NetlistFlavor, NormalizedDependency, parse_netlist};

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
        assert!(project.validation_yaml.contains("checks: []"));
        assert!(
            project
                .validation_yaml
                .contains("Suggested checks to customize")
        );
        assert!(project.validation_yaml.contains("signal: \"v(out)\""));
        assert!(project.validation_yaml.contains("signal: \"v(in)\""));
        assert!(project.validation_yaml.contains("signal: \"i(v1)\""));
        assert!(project.manifest_json.contains("\"flavor\": \"kicad\""));
        assert!(project.manifest_json.contains("\"suggested_signals\""));
        assert!(project.manifest_json.contains("\"signal\": \"v(out)\""));
        assert!(project.manifest_json.contains("\"signal\": \"v(in)\""));
        assert!(project.manifest_json.contains("\"signal\": \"i(v1)\""));
        assert!(project.manifest_json.contains("\"suggested_checks\""));
        assert!(
            project
                .manifest_json
                .contains("\"validation\": \"project.osl.yaml\"")
        );
    }

    #[test]
    fn suggests_import_checks_without_activating_them() {
        let source =
            "* imported netlist\nV1 in 0 DC 5\nR1 in out 1k\nC1 out 0 10n\n.tran 1u 1m\n.end\n";
        let report = parse_netlist(source, "imported.cir").unwrap();

        let signals = report.suggested_signals();
        let checks = report.suggested_checks();
        let project = report.normalized_project(source);

        assert_eq!(
            signals
                .iter()
                .map(|signal| signal.signal.as_str())
                .collect::<Vec<_>>(),
            ["v(in)", "v(out)", "i(v1)"]
        );
        assert_eq!(checks[0].signal, "v(out)");
        assert_eq!(checks[0].kind, "avg");
        assert_eq!(checks[1].signal, "v(in)");
        assert_eq!(checks[1].kind, "avg");
        assert_eq!(checks[2].signal, "i(v1)");
        assert_eq!(checks[2].kind, "rms");
        assert!(project.validation_yaml.contains("    checks: []\n"));
        assert!(!project.validation_yaml.contains("    checks:\n"));
    }

    #[test]
    fn rewrites_normalized_include_dependencies() {
        let source = "* KiCad netlist\n.include \"models.lib\"\nV1 in 0 DC 5\n.tran 1u 1m\n.end\n";
        let report = parse_netlist(source, "examples/kicad_import/kicad_with_model.cir").unwrap();
        let dependencies = vec![NormalizedDependency {
            source: "models.lib".to_string(),
            project_path: "models/models.lib".to_string(),
        }];

        let project = report.normalized_project_with_dependencies(source, &dependencies);

        assert!(project.netlist.contains(".include \"models/models.lib\""));
        assert!(project.manifest_json.contains("\"source\": \"models.lib\""));
        assert!(
            project
                .manifest_json
                .contains("\"project_path\": \"models/models.lib\"")
        );
        assert_eq!(project.dependencies.len(), 1);
    }
}
