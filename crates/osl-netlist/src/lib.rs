//! Netlist parsing and conversion — KiCad, LTspice, and SPICE format support.

mod kicad_import;
mod ltspice_import;

use kicad_import::{
    is_kicad_schematic, kicad_schematic_diagnostic_to_import, resolve_import_source_path,
};
use ltspice_import::{import_ltspice_asc, is_ltspice_schematic};
use osl_core::{OslResult, html_escape, json_escape, read_text};
use osl_kicad::read_kicad_schematic_hierarchy_netlist;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct ImportInput {
    pub source_netlist: String,
    pub source_path: PathBuf,
    pub report: ImportReport,
}

/// read import input。
pub fn read_import_input(path: &Path) -> OslResult<ImportInput> {
    let path = resolve_import_source_path(path)?;
    let source = path.display().to_string();
    if is_kicad_schematic(&path) {
        let exported = read_kicad_schematic_hierarchy_netlist(&path)?;
        let mut report = parse_netlist(&exported.netlist, &source)?;
        report.flavor = NetlistFlavor::KiCad;
        report.diagnostics.extend(
            exported
                .diagnostics
                .iter()
                .map(kicad_schematic_diagnostic_to_import),
        );
        Ok(ImportInput {
            source_netlist: exported.netlist,
            source_path: path,
            report,
        })
    } else if is_ltspice_schematic(&path) {
        let content = read_text(&path)?;
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
        let content = read_text(&path)?;
        let report = parse_netlist(&content, &source)?;
        Ok(ImportInput {
            source_netlist: content,
            source_path: path,
            report,
        })
    }
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
    /// parse。
    pub fn parse(path: &Path) -> OslResult<Self> {
        let content = read_text(path)?;
        parse_netlist(&content, &path.display().to_string())
    }

    /// component count。
    pub fn component_count(&self) -> usize {
        self.components.len()
    }

    /// symbol count。
    pub fn symbol_count(&self) -> usize {
        self.components
            .iter()
            .filter(|component| component.reference != ".control")
            .count()
    }

    /// directive count。
    pub fn directive_count(&self) -> usize {
        self.directives.len()
    }

    /// error count。
    pub fn error_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == ImportSeverity::Error)
            .count()
    }

    /// warning count。
    pub fn warning_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == ImportSeverity::Warning)
            .count()
    }

    /// compatibility score。
    pub fn compatibility_score(&self) -> u32 {
        let penalty = self.error_count() as u32 * 25 + self.warning_count() as u32 * 8;
        100_u32.saturating_sub(penalty)
    }

    /// to json。
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

    /// to html。
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

    /// normalized project。
    pub fn normalized_project(&self, source_netlist: &str) -> NormalizedImportProject {
        self.normalized_project_with_dependencies(source_netlist, &[])
    }

    /// normalized project with dependencies。
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

    /// suggested signals。
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

    /// suggested checks。
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
    /// to json。
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
    /// as str。
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
    /// as str。
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
    /// as str。
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


include!("netlist_parse_impl.rs");

