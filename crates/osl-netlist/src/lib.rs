use osl_core::{OslError, OslResult, html_escape, json_escape, read_text};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct ImportInput {
    pub source_netlist: String,
    pub source_path: PathBuf,
    pub report: ImportReport,
}

pub fn read_import_input(path: &Path) -> OslResult<ImportInput> {
    let path = resolve_import_source_path(path)?;
    let content = read_text(&path)?;
    let source = path.display().to_string();
    if is_ltspice_schematic(&path) {
        let imported = import_ltspice_asc(
            &content,
            &source,
            path.parent().unwrap_or_else(|| Path::new(".")),
        );
        let mut report = parse_netlist(&imported.netlist, &source)?;
        report.flavor = NetlistFlavor::Ltspice;
        report.diagnostics.extend(imported.diagnostics);
        Ok(ImportInput {
            source_netlist: imported.netlist,
            source_path: path,
            report,
        })
    } else {
        let report = parse_netlist(&content, &source)?;
        Ok(ImportInput {
            source_netlist: content,
            source_path: path,
            report,
        })
    }
}

fn resolve_import_source_path(path: &Path) -> OslResult<PathBuf> {
    if path.is_dir() {
        return discover_kicad_project_netlist(path);
    }
    if is_kicad_project_file(path) {
        return discover_kicad_project_netlist(path.parent().unwrap_or_else(|| Path::new(".")));
    }
    Ok(path.to_path_buf())
}

fn is_kicad_project_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("kicad_pro"))
}

fn discover_kicad_project_netlist(project_dir: &Path) -> OslResult<PathBuf> {
    let mut candidates = Vec::new();
    collect_kicad_netlist_candidates(project_dir, project_dir, &mut candidates)?;
    candidates.sort_by(|left, right| {
        kicad_candidate_score(right)
            .cmp(&kicad_candidate_score(left))
            .then_with(|| left.display().to_string().cmp(&right.display().to_string()))
    });
    candidates.into_iter().next().ok_or_else(|| {
        OslError::InvalidInput(format!(
            "{} does not contain an importable KiCad SPICE netlist (.cir, .spice, .sp)",
            project_dir.display()
        ))
    })
}

fn collect_kicad_netlist_candidates(
    root: &Path,
    dir: &Path,
    candidates: &mut Vec<PathBuf>,
) -> OslResult<()> {
    let entries =
        fs::read_dir(dir).map_err(|err| OslError::io(format!("read {}", dir.display()), err))?;
    for entry in entries {
        let entry = entry.map_err(|err| OslError::io(format!("read {}", dir.display()), err))?;
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().and_then(|name| name.to_str()) == Some("project") {
                continue;
            }
            if path
                .strip_prefix(root)
                .ok()
                .is_some_and(|relative| relative.components().count() > 3)
            {
                continue;
            }
            collect_kicad_netlist_candidates(root, &path, candidates)?;
        } else if is_spice_netlist_file(&path) {
            candidates.push(path);
        }
    }
    Ok(())
}

fn is_spice_netlist_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "cir" | "spice" | "sp"
            )
        })
        .unwrap_or(false)
}

fn kicad_candidate_score(path: &Path) -> usize {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let mut score = 0;
    if name.contains("kicad") {
        score += 20;
    }
    if name.ends_with(".cir") {
        score += 5;
    }
    if let Ok(content) = read_text(path) {
        let lowered = content.to_ascii_lowercase();
        if lowered.contains("kicad") || lowered.contains("eeschema") {
            score += 50;
        }
        if lowered.contains(".tran")
            || lowered.contains(".op")
            || lowered.contains(".ac")
            || lowered.contains(".dc")
        {
            score += 10;
        }
    }
    score
}

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

fn is_ltspice_schematic(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("asc"))
}

#[derive(Debug)]
struct LtspiceSchematicImport {
    netlist: String,
    diagnostics: Vec<ImportDiagnostic>,
}

#[derive(Debug, Default)]
struct LtspiceSchematic {
    wires: Vec<LtspiceWire>,
    flags: Vec<LtspiceFlag>,
    symbols: Vec<LtspiceSymbol>,
    directives: Vec<LtspiceDirective>,
    diagnostics: Vec<ImportDiagnostic>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct AscPoint {
    x: i32,
    y: i32,
}

impl AscPoint {
    fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

#[derive(Debug)]
struct LtspiceWire {
    start: AscPoint,
    end: AscPoint,
}

#[derive(Debug)]
struct LtspiceFlag {
    point: AscPoint,
    name: String,
}

#[derive(Debug)]
struct LtspiceSymbol {
    line: usize,
    name: String,
    origin: AscPoint,
    rotation: String,
    attrs: BTreeMap<String, String>,
}

#[derive(Debug)]
struct LtspiceDirective {
    text: String,
}

#[derive(Debug, Clone)]
struct LtspiceSymbolSpec {
    prefix: String,
    pins: Vec<AscPoint>,
    source: LtspiceSymbolSpecSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LtspiceSymbolSpecSource {
    Builtin,
    AsyFile,
}

#[derive(Debug)]
struct LtspiceSymbolLibrary {
    search_dirs: Vec<PathBuf>,
    cache: BTreeMap<String, Option<LtspiceSymbolSpec>>,
    diagnostics: Vec<ImportDiagnostic>,
}

#[derive(Debug)]
struct AscNetGraph {
    names: BTreeMap<AscPoint, String>,
    has_ground: bool,
}

#[derive(Debug)]
struct DisjointSet {
    parents: Vec<usize>,
}

impl DisjointSet {
    fn new(len: usize) -> Self {
        Self {
            parents: (0..len).collect(),
        }
    }

    fn find(&mut self, item: usize) -> usize {
        let parent = self.parents[item];
        if parent == item {
            item
        } else {
            let root = self.find(parent);
            self.parents[item] = root;
            root
        }
    }

    fn union(&mut self, left: usize, right: usize) {
        let left_root = self.find(left);
        let right_root = self.find(right);
        if left_root != right_root {
            self.parents[right_root] = left_root;
        }
    }
}

impl LtspiceSymbolLibrary {
    fn new(base_dir: &Path) -> Self {
        let search_dirs = ltspice_symbol_search_dirs(base_dir);
        Self {
            search_dirs,
            cache: BTreeMap::new(),
            diagnostics: Vec::new(),
        }
    }

    fn spec_for(&mut self, symbol: &LtspiceSymbol) -> Option<LtspiceSymbolSpec> {
        let key = ltspice_symbol_basename(&symbol.name);
        if let Some(spec) = self.cache.get(&key) {
            return spec.clone();
        }

        let spec = self
            .read_symbol_spec(symbol)
            .or_else(|| ltspice_builtin_symbol(&symbol.name));
        self.cache.insert(key, spec.clone());
        spec
    }

    fn read_symbol_spec(&mut self, symbol: &LtspiceSymbol) -> Option<LtspiceSymbolSpec> {
        let path = self.symbol_path(&symbol.name)?;
        let content = match read_text(&path) {
            Ok(content) => content,
            Err(error) => {
                self.diagnostics.push(import_diagnostic(
                    symbol.line,
                    ImportSeverity::Warning,
                    "ltspice_symbol_read_failed",
                    &format!("could not read LTspice symbol {}: {error}", path.display()),
                    "Keep custom .asy files next to the imported .asc or use supported primitive symbols.",
                ));
                return None;
            }
        };
        match parse_ltspice_asy_spec(&content) {
            Some(spec) => Some(spec),
            None => {
                self.diagnostics.push(import_diagnostic(
                    symbol.line,
                    ImportSeverity::Warning,
                    "ltspice_symbol_no_pins",
                    &format!("LTspice symbol {} contains no ordered pins", path.display()),
                    "Add PIN entries with PINATTR SpiceOrder or use a symbol with explicit pin metadata.",
                ));
                None
            }
        }
    }

    fn symbol_path(&self, symbol_name: &str) -> Option<PathBuf> {
        let normalized = symbol_name.replace('\\', "/");
        let raw_path = Path::new(&normalized);
        self.search_dirs
            .iter()
            .flat_map(|search_dir| symbol_path_candidates(search_dir, raw_path, &normalized))
            .find(|candidate| candidate.is_file())
    }
}

fn ltspice_symbol_search_dirs(base_dir: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    push_unique_path(&mut dirs, base_dir.to_path_buf());
    push_unique_path(&mut dirs, base_dir.join("sym"));

    if let Some(paths) = env::var_os("NEKOSPICE_LTSPICE_SYM_PATH") {
        for path in env::split_paths(&paths) {
            push_unique_path(&mut dirs, path);
        }
    }
    if let Some(home) = env::var_os("HOME").map(PathBuf::from) {
        push_unique_path(&mut dirs, home.join(".local/share/LTspice/lib/sym"));
        push_unique_path(
            &mut dirs,
            home.join(".local/share/wineprefixes/ltspice/drive_c/users")
                .join(env::var("USER").unwrap_or_else(|_| "user".to_string()))
                .join("AppData/Local/LTspice/lib/sym"),
        );
        push_unique_path(&mut dirs, home.join("Documents/LTspiceXVII/lib/sym"));
    }
    push_unique_path(
        &mut dirs,
        PathBuf::from("/Applications/LTspice.app/Contents/lib/sym"),
    );
    push_unique_path(
        &mut dirs,
        PathBuf::from("C:/Users/Public/Documents/LTspiceXVII/lib/sym"),
    );
    dirs
}

fn symbol_path_candidates(search_dir: &Path, raw_path: &Path, normalized: &str) -> Vec<PathBuf> {
    if raw_path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("asy"))
    {
        vec![search_dir.join(raw_path)]
    } else {
        vec![
            search_dir.join(format!("{normalized}.asy")),
            search_dir.join(raw_path).with_extension("asy"),
        ]
    }
}

fn push_unique_path(paths: &mut Vec<PathBuf>, path: PathBuf) {
    if !paths.iter().any(|existing| existing == &path) {
        paths.push(path);
    }
}

impl AscNetGraph {
    fn build(wires: &[LtspiceWire], flags: &[LtspiceFlag], pin_points: &[AscPoint]) -> AscNetGraph {
        let mut points = BTreeSet::new();
        for wire in wires {
            points.insert(wire.start);
            points.insert(wire.end);
        }
        for flag in flags {
            points.insert(flag.point);
        }
        for point in pin_points {
            points.insert(*point);
        }
        for left in wires {
            for right in wires {
                if let Some(point) = wire_intersection(left, right) {
                    points.insert(point);
                }
            }
        }

        let ordered_points = points.into_iter().collect::<Vec<_>>();
        let point_indexes = ordered_points
            .iter()
            .enumerate()
            .map(|(index, point)| (*point, index))
            .collect::<BTreeMap<_, _>>();
        let mut graph = DisjointSet::new(ordered_points.len());

        for wire in wires {
            let mut segment_points = ordered_points
                .iter()
                .filter(|point| wire_contains_point(wire, **point))
                .filter_map(|point| point_indexes.get(point).copied())
                .collect::<Vec<_>>();
            segment_points.sort_by_key(|index| ordered_points[*index]);
            if let Some(first) = segment_points.first().copied() {
                for point in segment_points.iter().copied().skip(1) {
                    graph.union(first, point);
                }
            }
        }

        let mut flags_by_name = BTreeMap::<String, Vec<usize>>::new();
        for flag in flags {
            if let Some(index) = point_indexes.get(&flag.point).copied() {
                flags_by_name
                    .entry(flag.name.clone())
                    .or_default()
                    .push(index);
            }
        }
        for flag_points in flags_by_name.values() {
            if let Some(first) = flag_points.first().copied() {
                for point in flag_points.iter().copied().skip(1) {
                    graph.union(first, point);
                }
            }
        }

        let mut labels_by_root = BTreeMap::<usize, BTreeSet<String>>::new();
        for flag in flags {
            if let Some(index) = point_indexes.get(&flag.point).copied() {
                let root = graph.find(index);
                labels_by_root
                    .entry(root)
                    .or_default()
                    .insert(flag.name.clone());
            }
        }

        let mut names_by_root = BTreeMap::<usize, String>::new();
        let mut generated_index = 1;
        for (index, _) in ordered_points.iter().enumerate() {
            let root = graph.find(index);
            names_by_root.entry(root).or_insert_with(|| {
                labels_by_root
                    .get(&root)
                    .and_then(preferred_label)
                    .unwrap_or_else(|| {
                        let name = format!("n{generated_index:03}");
                        generated_index += 1;
                        name
                    })
            });
        }

        let mut has_ground = false;
        let mut names = BTreeMap::new();
        for (index, point) in ordered_points.iter().enumerate() {
            let root = graph.find(index);
            let name = names_by_root.get(&root).cloned().unwrap_or_else(|| {
                let name = format!("n{generated_index:03}");
                generated_index += 1;
                name
            });
            if is_ground_node(&name.to_ascii_lowercase()) {
                has_ground = true;
            }
            names.insert(*point, name);
        }

        Self { names, has_ground }
    }

    fn node_name(&self, point: AscPoint) -> Option<&str> {
        self.names.get(&point).map(String::as_str)
    }
}

fn import_ltspice_asc(input: &str, source: &str, base_dir: &Path) -> LtspiceSchematicImport {
    let mut schematic = parse_ltspice_asc(input);
    let mut library = LtspiceSymbolLibrary::new(base_dir);
    let pin_points = schematic
        .symbols
        .iter()
        .flat_map(|symbol| ltspice_symbol_pin_points(symbol, &mut library))
        .collect::<Vec<_>>();
    schematic.diagnostics.append(&mut library.diagnostics);
    let graph = AscNetGraph::build(&schematic.wires, &schematic.flags, &pin_points);

    let mut lines = vec![
        format!("* Imported from LTspice schematic: {source}"),
        "* Generated by NekoSpice asc importer.".to_string(),
    ];
    for symbol in &schematic.symbols {
        if let Some(line) =
            ltspice_symbol_to_netlist(symbol, &graph, &mut library, &mut schematic.diagnostics)
        {
            lines.push(line);
        }
    }
    schematic.diagnostics.append(&mut library.diagnostics);
    for directive in &schematic.directives {
        lines.push(directive.text.clone());
    }
    if !lines
        .iter()
        .any(|line| line.trim().eq_ignore_ascii_case(".end"))
    {
        lines.push(".end".to_string());
    }

    if !schematic.symbols.is_empty() && !graph.has_ground {
        schematic.diagnostics.push(import_diagnostic(
            1,
            ImportSeverity::Warning,
            "ltspice_missing_ground",
            "LTspice schematic has no node labelled 0 or ground",
            "Add a ground symbol or FLAG 0 before running the imported netlist.",
        ));
    }

    LtspiceSchematicImport {
        netlist: format!("{}\n", lines.join("\n")),
        diagnostics: schematic.diagnostics,
    }
}

fn parse_ltspice_asc(input: &str) -> LtspiceSchematic {
    let mut schematic = LtspiceSchematic::default();
    let mut current_symbol = None::<LtspiceSymbol>;

    for (line_number, raw_line) in input.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let tokens = line.split_whitespace().collect::<Vec<_>>();
        let Some(keyword) = tokens.first().copied() else {
            continue;
        };

        match keyword {
            "WIRE" => match parse_wire_tokens(&tokens) {
                Some(wire) => schematic.wires.push(wire),
                None => schematic.diagnostics.push(import_diagnostic(
                    line_number + 1,
                    ImportSeverity::Error,
                    "ltspice_bad_wire",
                    "WIRE statement must contain four integer coordinates",
                    "Check the LTspice schematic export around this line.",
                )),
            },
            "FLAG" => match parse_flag_tokens(&tokens) {
                Some(flag) => schematic.flags.push(flag),
                None => schematic.diagnostics.push(import_diagnostic(
                    line_number + 1,
                    ImportSeverity::Error,
                    "ltspice_bad_flag",
                    "FLAG statement must contain x, y, and node name",
                    "Check the LTspice schematic export around this line.",
                )),
            },
            "SYMBOL" => {
                if let Some(symbol) = current_symbol.take() {
                    schematic.symbols.push(symbol);
                }
                match parse_symbol_tokens(&tokens, line_number + 1) {
                    Some(symbol) => current_symbol = Some(symbol),
                    None => schematic.diagnostics.push(import_diagnostic(
                        line_number + 1,
                        ImportSeverity::Error,
                        "ltspice_bad_symbol",
                        "SYMBOL statement must contain name, x, y, and rotation",
                        "Check the LTspice schematic export around this line.",
                    )),
                }
            }
            "SYMATTR" => {
                if let Some(symbol) = current_symbol.as_mut() {
                    if let Some((key, value)) = split_key_value(line.trim_start_matches("SYMATTR"))
                    {
                        symbol.attrs.insert(key.to_string(), value.to_string());
                    }
                } else {
                    schematic.diagnostics.push(import_diagnostic(
                        line_number + 1,
                        ImportSeverity::Warning,
                        "ltspice_orphan_attribute",
                        "SYMATTR appears before any SYMBOL",
                        "Move the attribute below the symbol it belongs to.",
                    ));
                }
            }
            "TEXT" => {
                if let Some(directive) = parse_text_directive(&tokens) {
                    schematic.directives.push(directive);
                }
            }
            _ => {}
        }
    }

    if let Some(symbol) = current_symbol.take() {
        schematic.symbols.push(symbol);
    }
    schematic
}

fn parse_wire_tokens(tokens: &[&str]) -> Option<LtspiceWire> {
    Some(LtspiceWire {
        start: AscPoint::new(tokens.get(1)?.parse().ok()?, tokens.get(2)?.parse().ok()?),
        end: AscPoint::new(tokens.get(3)?.parse().ok()?, tokens.get(4)?.parse().ok()?),
    })
}

fn parse_flag_tokens(tokens: &[&str]) -> Option<LtspiceFlag> {
    Some(LtspiceFlag {
        point: AscPoint::new(tokens.get(1)?.parse().ok()?, tokens.get(2)?.parse().ok()?),
        name: tokens.get(3)?.to_string(),
    })
}

fn parse_symbol_tokens(tokens: &[&str], line: usize) -> Option<LtspiceSymbol> {
    Some(LtspiceSymbol {
        line,
        name: tokens.get(1)?.to_string(),
        origin: AscPoint::new(tokens.get(2)?.parse().ok()?, tokens.get(3)?.parse().ok()?),
        rotation: tokens.get(4)?.to_string(),
        attrs: BTreeMap::new(),
    })
}

fn parse_text_directive(tokens: &[&str]) -> Option<LtspiceDirective> {
    let text = tokens.get(5..)?.join(" ");
    let directive = text.strip_prefix('!')?.trim();
    if directive.is_empty() {
        None
    } else {
        Some(LtspiceDirective {
            text: directive.to_string(),
        })
    }
}

fn split_key_value(input: &str) -> Option<(&str, &str)> {
    let input = input.trim();
    let split_at = input
        .char_indices()
        .find(|(_, character)| character.is_whitespace())
        .map(|(index, _)| index)?;
    let key = input[..split_at].trim();
    let value = input[split_at..].trim();
    if key.is_empty() {
        None
    } else {
        Some((key, value))
    }
}

#[derive(Debug)]
struct LtspiceAsyPin {
    point: AscPoint,
    spice_order: Option<usize>,
}

fn parse_ltspice_asy_spec(input: &str) -> Option<LtspiceSymbolSpec> {
    let mut prefix = None::<String>;
    let mut pins = Vec::new();

    for raw_line in input.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let tokens = line.split_whitespace().collect::<Vec<_>>();
        let Some(keyword) = tokens.first().copied() else {
            continue;
        };

        match keyword {
            "SYMATTR" if tokens.len() >= 3 && tokens[1] == "Prefix" => {
                prefix = Some(tokens[2].to_string());
            }
            "PIN" => {
                let Some(x) = tokens.get(1).and_then(|value| value.parse::<i32>().ok()) else {
                    continue;
                };
                let Some(y) = tokens.get(2).and_then(|value| value.parse::<i32>().ok()) else {
                    continue;
                };
                pins.push(LtspiceAsyPin {
                    point: AscPoint::new(x, y),
                    spice_order: None,
                });
            }
            "PINATTR" if tokens.len() >= 3 && tokens[1] == "SpiceOrder" => {
                if let Some(pin) = pins.last_mut() {
                    pin.spice_order = tokens[2].parse::<usize>().ok();
                }
            }
            _ => {}
        }
    }

    if pins.is_empty() {
        return None;
    }

    pins.sort_by_key(|pin| pin.spice_order.unwrap_or(usize::MAX));
    Some(LtspiceSymbolSpec {
        prefix: prefix.unwrap_or_else(|| "X".to_string()),
        pins: pins.into_iter().map(|pin| pin.point).collect(),
        source: LtspiceSymbolSpecSource::AsyFile,
    })
}

fn ltspice_symbol_pin_points(
    symbol: &LtspiceSymbol,
    library: &mut LtspiceSymbolLibrary,
) -> Vec<AscPoint> {
    let Some(spec) = library.spec_for(symbol) else {
        return Vec::new();
    };
    spec.pins
        .iter()
        .filter_map(|pin| transform_ltspice_point(*pin, symbol.origin, &symbol.rotation))
        .collect()
}

fn ltspice_symbol_to_netlist(
    symbol: &LtspiceSymbol,
    graph: &AscNetGraph,
    library: &mut LtspiceSymbolLibrary,
    diagnostics: &mut Vec<ImportDiagnostic>,
) -> Option<String> {
    let Some(spec) = library.spec_for(symbol) else {
        diagnostics.push(import_diagnostic(
            symbol.line,
            ImportSeverity::Error,
            "ltspice_unsupported_symbol",
            &format!(
                "LTspice symbol '{}' is not supported by the importer yet",
                symbol.name
            ),
            "Replace it with a supported primitive or add an .asy pin mapping rule.",
        ));
        return None;
    };
    let pins = ltspice_symbol_pin_points(symbol, library);
    if pins.len() != spec.pins.len() {
        diagnostics.push(import_diagnostic(
            symbol.line,
            ImportSeverity::Error,
            "ltspice_unsupported_rotation",
            &format!(
                "LTspice symbol '{}' uses unsupported rotation '{}'",
                symbol.name, symbol.rotation
            ),
            "Use R0, R90, R180, R270, M0, M90, M180, or M270 before importing.",
        ));
        return None;
    }

    let mut nodes = Vec::new();
    for pin in pins {
        let Some(node) = graph.node_name(pin) else {
            diagnostics.push(import_diagnostic(
                symbol.line,
                ImportSeverity::Error,
                "ltspice_unmapped_pin",
                &format!(
                    "LTspice symbol '{}' has a pin at {},{} that is not in the net graph",
                    symbol.name, pin.x, pin.y
                ),
                "Connect the pin to a wire or label before importing.",
            ));
            return None;
        };
        nodes.push(node.to_string());
    }

    let instance = symbol_instance_name(symbol, &spec, diagnostics);
    let value = symbol_value(symbol, &spec, diagnostics)?;
    Some(format!("{} {} {}", instance, nodes.join(" "), value))
}

fn symbol_instance_name(
    symbol: &LtspiceSymbol,
    spec: &LtspiceSymbolSpec,
    diagnostics: &mut Vec<ImportDiagnostic>,
) -> String {
    let raw_name = symbol
        .attrs
        .get("InstName")
        .filter(|name| !name.trim().is_empty())
        .cloned()
        .unwrap_or_else(|| {
            diagnostics.push(import_diagnostic(
                symbol.line,
                ImportSeverity::Warning,
                "ltspice_missing_instance_name",
                &format!("LTspice symbol '{}' has no InstName", symbol.name),
                "Assign a stable reference designator before importing.",
            ));
            format!("{}{}", spec.prefix, symbol.line)
        });
    normalize_instance_prefix(&raw_name, &spec.prefix)
}

fn symbol_value(
    symbol: &LtspiceSymbol,
    spec: &LtspiceSymbolSpec,
    diagnostics: &mut Vec<ImportDiagnostic>,
) -> Option<String> {
    let mut parts = Vec::new();
    if let Some(value) = symbol
        .attrs
        .get("Value")
        .filter(|value| !value.trim().is_empty())
    {
        parts.push(value.trim().to_string());
    }
    for key in ["Value2", "SpiceLine", "SpiceLine2"] {
        if let Some(value) = symbol
            .attrs
            .get(key)
            .filter(|value| !value.trim().is_empty())
        {
            parts.push(value.trim().to_string());
        }
    }
    if parts.is_empty() && spec.source == LtspiceSymbolSpecSource::AsyFile {
        parts.push(ltspice_symbol_basename(&symbol.name));
    }
    if parts.is_empty() {
        diagnostics.push(import_diagnostic(
            symbol.line,
            ImportSeverity::Error,
            "ltspice_missing_value",
            &format!("LTspice symbol '{}' has no Value attribute", symbol.name),
            &format!(
                "Add a value compatible with {} before importing.",
                spec.prefix
            ),
        ));
        None
    } else {
        Some(parts.join(" "))
    }
}

fn ltspice_builtin_symbol(name: &str) -> Option<LtspiceSymbolSpec> {
    const RES_PINS: &[AscPoint] = &[AscPoint { x: 16, y: 16 }, AscPoint { x: 16, y: 96 }];
    const CAP_PINS: &[AscPoint] = &[AscPoint { x: 16, y: 0 }, AscPoint { x: 16, y: 64 }];
    const SOURCE_PINS: &[AscPoint] = &[AscPoint { x: 0, y: 16 }, AscPoint { x: 0, y: 96 }];
    const CURRENT_PINS: &[AscPoint] = &[AscPoint { x: 0, y: 0 }, AscPoint { x: 0, y: 80 }];
    const DIODE_PINS: &[AscPoint] = &[AscPoint { x: 16, y: 0 }, AscPoint { x: 16, y: 64 }];
    const BJT_PINS: &[AscPoint] = &[
        AscPoint { x: 64, y: 0 },
        AscPoint { x: 0, y: 48 },
        AscPoint { x: 64, y: 96 },
    ];
    const MOS_PINS: &[AscPoint] = &[
        AscPoint { x: 48, y: 0 },
        AscPoint { x: 0, y: 80 },
        AscPoint { x: 48, y: 96 },
    ];
    const MOS4_PINS: &[AscPoint] = &[
        AscPoint { x: 48, y: 0 },
        AscPoint { x: 0, y: 80 },
        AscPoint { x: 48, y: 96 },
        AscPoint { x: 48, y: 48 },
    ];
    const JFET_PINS: &[AscPoint] = &[
        AscPoint { x: 48, y: 0 },
        AscPoint { x: 0, y: 64 },
        AscPoint { x: 48, y: 96 },
    ];
    const E_SOURCE_PINS: &[AscPoint] = &[
        AscPoint { x: 0, y: 16 },
        AscPoint { x: 0, y: 96 },
        AscPoint { x: -48, y: 32 },
        AscPoint { x: -48, y: 80 },
    ];
    const G_SOURCE_PINS: &[AscPoint] = &[
        AscPoint { x: 0, y: 96 },
        AscPoint { x: 0, y: 16 },
        AscPoint { x: -48, y: 32 },
        AscPoint { x: -48, y: 80 },
    ];
    const SWITCH_PINS: &[AscPoint] = &[
        AscPoint { x: 0, y: 16 },
        AscPoint { x: 0, y: 96 },
        AscPoint { x: -48, y: 80 },
        AscPoint { x: -48, y: 32 },
    ];

    match ltspice_symbol_basename(name).as_str() {
        "res" | "res2" => Some(LtspiceSymbolSpec {
            prefix: "R".to_string(),
            pins: RES_PINS.to_vec(),
            source: LtspiceSymbolSpecSource::Builtin,
        }),
        "cap" | "polcap" => Some(LtspiceSymbolSpec {
            prefix: "C".to_string(),
            pins: CAP_PINS.to_vec(),
            source: LtspiceSymbolSpecSource::Builtin,
        }),
        "ind" | "ind2" => Some(LtspiceSymbolSpec {
            prefix: "L".to_string(),
            pins: RES_PINS.to_vec(),
            source: LtspiceSymbolSpecSource::Builtin,
        }),
        "voltage" => Some(LtspiceSymbolSpec {
            prefix: "V".to_string(),
            pins: SOURCE_PINS.to_vec(),
            source: LtspiceSymbolSpecSource::Builtin,
        }),
        "current" => Some(LtspiceSymbolSpec {
            prefix: "I".to_string(),
            pins: CURRENT_PINS.to_vec(),
            source: LtspiceSymbolSpecSource::Builtin,
        }),
        "diode" | "led" | "schottky" | "tvsdiode" | "varactor" | "zener" => {
            Some(LtspiceSymbolSpec {
                prefix: "D".to_string(),
                pins: DIODE_PINS.to_vec(),
                source: LtspiceSymbolSpecSource::Builtin,
            })
        }
        "npn" | "npn2" | "npn3" | "npn4" => Some(LtspiceSymbolSpec {
            prefix: "Q".to_string(),
            pins: BJT_PINS.to_vec(),
            source: LtspiceSymbolSpecSource::Builtin,
        }),
        "pnp" | "pnp2" | "pnp4" | "lpnp" => Some(LtspiceSymbolSpec {
            prefix: "Q".to_string(),
            pins: BJT_PINS.to_vec(),
            source: LtspiceSymbolSpecSource::Builtin,
        }),
        "nmos" | "pmos" => Some(LtspiceSymbolSpec {
            prefix: "M".to_string(),
            pins: MOS_PINS.to_vec(),
            source: LtspiceSymbolSpecSource::Builtin,
        }),
        "nmos4" | "pmos4" => Some(LtspiceSymbolSpec {
            prefix: "M".to_string(),
            pins: MOS4_PINS.to_vec(),
            source: LtspiceSymbolSpecSource::Builtin,
        }),
        "njf" | "pjf" => Some(LtspiceSymbolSpec {
            prefix: "J".to_string(),
            pins: JFET_PINS.to_vec(),
            source: LtspiceSymbolSpecSource::Builtin,
        }),
        "e" | "e2" => Some(LtspiceSymbolSpec {
            prefix: "E".to_string(),
            pins: E_SOURCE_PINS.to_vec(),
            source: LtspiceSymbolSpecSource::Builtin,
        }),
        "g" | "g2" => Some(LtspiceSymbolSpec {
            prefix: "G".to_string(),
            pins: G_SOURCE_PINS.to_vec(),
            source: LtspiceSymbolSpecSource::Builtin,
        }),
        "f" => Some(LtspiceSymbolSpec {
            prefix: "F".to_string(),
            pins: CURRENT_PINS.to_vec(),
            source: LtspiceSymbolSpecSource::Builtin,
        }),
        "h" => Some(LtspiceSymbolSpec {
            prefix: "H".to_string(),
            pins: SOURCE_PINS.to_vec(),
            source: LtspiceSymbolSpecSource::Builtin,
        }),
        "sw" => Some(LtspiceSymbolSpec {
            prefix: "S".to_string(),
            pins: SWITCH_PINS.to_vec(),
            source: LtspiceSymbolSpecSource::Builtin,
        }),
        "csw" => Some(LtspiceSymbolSpec {
            prefix: "W".to_string(),
            pins: CURRENT_PINS.to_vec(),
            source: LtspiceSymbolSpecSource::Builtin,
        }),
        "bi" | "bv" => Some(LtspiceSymbolSpec {
            prefix: "B".to_string(),
            pins: CURRENT_PINS.to_vec(),
            source: LtspiceSymbolSpecSource::Builtin,
        }),
        _ => None,
    }
}

fn normalize_instance_prefix(instance: &str, prefix: &str) -> String {
    let instance = instance.trim();
    let prefix = prefix.trim();
    if instance.is_empty() || prefix.is_empty() {
        return instance.to_string();
    }
    let first = instance.chars().next().unwrap_or_default();
    let prefix_first = prefix.chars().next().unwrap_or_default();
    if first.eq_ignore_ascii_case(&prefix_first) {
        instance.to_string()
    } else {
        format!("{prefix}{instance}")
    }
}

fn ltspice_symbol_basename(name: &str) -> String {
    name.replace('\\', "/")
        .split('/')
        .next_back()
        .unwrap_or(name)
        .to_ascii_lowercase()
}

fn transform_ltspice_point(point: AscPoint, origin: AscPoint, rotation: &str) -> Option<AscPoint> {
    let (x, y) = match rotation {
        "R0" => (point.x, point.y),
        "R90" => (-point.y, point.x),
        "R180" => (-point.x, -point.y),
        "R270" => (point.y, -point.x),
        "M0" => (-point.x, point.y),
        "M90" => (-point.y, -point.x),
        "M180" => (point.x, -point.y),
        "M270" => (point.y, point.x),
        _ => return None,
    };
    Some(AscPoint::new(origin.x + x, origin.y + y))
}

fn preferred_label(labels: &BTreeSet<String>) -> Option<String> {
    labels
        .iter()
        .find(|label| is_ground_node(&label.to_ascii_lowercase()))
        .cloned()
        .or_else(|| labels.iter().next().cloned())
}

fn wire_contains_point(wire: &LtspiceWire, point: AscPoint) -> bool {
    if wire.start.x == wire.end.x {
        point.x == wire.start.x && between_inclusive(point.y, wire.start.y, wire.end.y)
    } else if wire.start.y == wire.end.y {
        point.y == wire.start.y && between_inclusive(point.x, wire.start.x, wire.end.x)
    } else {
        false
    }
}

fn wire_intersection(left: &LtspiceWire, right: &LtspiceWire) -> Option<AscPoint> {
    if left.start.x == left.end.x && right.start.y == right.end.y {
        let point = AscPoint::new(left.start.x, right.start.y);
        return (wire_contains_point(left, point) && wire_contains_point(right, point))
            .then_some(point);
    }
    if left.start.y == left.end.y && right.start.x == right.end.x {
        let point = AscPoint::new(right.start.x, left.start.y);
        return (wire_contains_point(left, point) && wire_contains_point(right, point))
            .then_some(point);
    }
    None
}

fn between_inclusive(value: i32, left: i32, right: i32) -> bool {
    let min = left.min(right);
    let max = left.max(right);
    value >= min && value <= max
}

fn import_diagnostic(
    line: usize,
    severity: ImportSeverity,
    code: &str,
    message: &str,
    suggestion: &str,
) -> ImportDiagnostic {
    ImportDiagnostic {
        line,
        severity,
        code: code.to_string(),
        message: message.to_string(),
        suggestion: suggestion.to_string(),
    }
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
    use super::{
        ComponentKind, ImportSeverity, NetlistFlavor, NormalizedDependency, import_ltspice_asc,
        parse_netlist, read_import_input,
    };
    use std::path::Path;

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

    #[test]
    fn discovers_kicad_project_netlist_from_directory_or_project_file() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let project_dir = workspace_root.join("examples/kicad_project");

        let from_dir = read_import_input(&project_dir).unwrap();
        let from_project_file =
            read_import_input(&project_dir.join("kicad_project.kicad_pro")).unwrap();

        assert_eq!(from_dir.report.flavor, NetlistFlavor::KiCad);
        assert_eq!(from_project_file.report.flavor, NetlistFlavor::KiCad);
        assert_eq!(from_dir.source_path, project_dir.join("kicad_project.cir"));
        assert_eq!(
            from_project_file.source_path,
            project_dir.join("kicad_project.cir")
        );
        assert!(
            from_dir
                .source_netlist
                .contains(".include \"models/ideal_diode.lib\"")
        );
    }

    #[test]
    fn imports_ltspice_asc_to_runnable_netlist() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let input =
            read_import_input(&workspace_root.join("examples/ltspice_import/ltspice_rc.asc"))
                .unwrap();

        assert_eq!(input.report.flavor, NetlistFlavor::Ltspice);
        assert_eq!(input.report.error_count(), 0);
        assert!(input.source_netlist.contains("V1 n001 0 PULSE"));
        assert!(input.source_netlist.contains("R1 out n001 1k"));
        assert!(input.source_netlist.contains("C1 out 0 100n"));
        assert!(input.source_netlist.contains(".tran 1u 500u"));
        assert!(input.source_netlist.contains("out"));

        let project = input.report.normalized_project(&input.source_netlist);
        assert!(project.validation_yaml.contains("signal: \"v(out)\""));
        assert!(project.manifest_json.contains("\"flavor\": \"ltspice\""));
    }

    #[test]
    fn imports_ltspice_asc_with_local_asy_symbol() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let input =
            read_import_input(&workspace_root.join("examples/ltspice_import/ltspice_subckt.asc"))
                .unwrap();

        assert_eq!(input.report.flavor, NetlistFlavor::Ltspice);
        assert_eq!(input.report.error_count(), 0);
        assert!(input.source_netlist.contains("V1 n001 0 DC 1"));
        assert!(input.source_netlist.contains("XU1 n001 out 0 gain_block"));
        assert!(input.source_netlist.contains(".include \"gain_block.lib\""));

        let project = input.report.normalized_project(&input.source_netlist);
        assert!(project.validation_yaml.contains("signal: \"v(out)\""));
        assert!(project.validation_yaml.contains("signal: \"i(v1)\""));
    }

    #[test]
    fn imports_ltspice_asc_with_symbol_search_dir() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let input = read_import_input(
            &workspace_root.join("examples/ltspice_import/ltspice_sym_search.asc"),
        )
        .unwrap();

        assert_eq!(input.report.flavor, NetlistFlavor::Ltspice);
        assert_eq!(input.report.error_count(), 0);
        assert!(input.source_netlist.contains("XU1 n001 out 0 gain_block"));
        assert!(!input.report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "ltspice_unsupported_symbol"
                || diagnostic.code == "ltspice_symbol_no_pins"
        }));
    }

    #[test]
    fn imports_ltspice_bjt_builtin_symbol() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let input =
            read_import_input(&workspace_root.join("examples/ltspice_import/ltspice_bjt.asc"))
                .unwrap();

        assert_eq!(input.report.flavor, NetlistFlavor::Ltspice);
        assert_eq!(input.report.error_count(), 0);
        assert!(input.source_netlist.contains("Q1 vcc in 0 QTEST"));
        assert!(input.source_netlist.contains(".model QTEST NPN"));
        assert!(!input.report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "ltspice_unsupported_symbol"
                || diagnostic.code == "ltspice_unmapped_pin"
        }));
    }

    #[test]
    fn imports_ltspice_controlled_source_builtin_symbol() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let input =
            read_import_input(&workspace_root.join("examples/ltspice_import/ltspice_vcvs.asc"))
                .unwrap();

        assert_eq!(input.report.flavor, NetlistFlavor::Ltspice);
        assert_eq!(input.report.error_count(), 0);
        assert!(input.source_netlist.contains("E1 out 0 n001 0 2"));
        assert!(!input.report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "ltspice_unsupported_symbol"
                || diagnostic.code == "ltspice_unmapped_pin"
        }));
    }

    #[test]
    fn reports_unsupported_ltspice_asc_symbol() {
        let input = r#"
Version 4
SHEET 1 880 680
FLAG 0 0 0
SYMBOL opamp 0 0 R0
SYMATTR InstName U1
SYMATTR Value OPAMP
TEXT 0 96 Left 2 !.op
"#;

        let imported = import_ltspice_asc(input, "unsupported.asc", Path::new("."));

        assert!(imported.netlist.contains(".op"));
        assert!(
            imported
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "ltspice_unsupported_symbol"
                    && diagnostic.severity == ImportSeverity::Error)
        );
    }
}
