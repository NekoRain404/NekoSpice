// OpenSpiceLab-RS model checking subsystem.
// Model scan and check implementation is in model_check_impl.rs.

use osl_core::{OslError, OslResult, html_escape, json_escape, read_text};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct ModelCheckReport {
    pub root: String,
    pub files: Vec<ModelFileSummary>,
    pub symbols: Vec<SymbolSummary>,
    pub subckts: Vec<SubcktSummary>,
    pub models: Vec<ModelSummary>,
    pub pin_mappings: Vec<PinMappingSummary>,
    pub diagnostics: Vec<ModelDiagnostic>,
}

#[derive(Debug, Default)]
pub struct ModelCheckOptions {
    pub symbol_path: Option<PathBuf>,
}

impl ModelCheckReport {
    /// scan。
    pub fn scan(root: &Path) -> OslResult<Self> {
        Self::scan_with_options(root, &ModelCheckOptions::default())
    }

    /// scan with options。
    pub fn scan_with_options(root: &Path, options: &ModelCheckOptions) -> OslResult<Self> {
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
            symbols: Vec::new(),
            subckts: Vec::new(),
            models: Vec::new(),
            pin_mappings: Vec::new(),
            diagnostics: Vec::new(),
        };

        for file in files {
            scan_model_file(&file, &mut report)?;
        }

        if let Some(symbol_path) = &options.symbol_path {
            let symbol = parse_ltspice_symbol(symbol_path)?;
            check_symbol_pin_mapping(&symbol, &mut report);
            report.symbols.push(symbol);
        }

        report
            .files
            .sort_by(|left, right| left.path.cmp(&right.path));
        report
            .symbols
            .sort_by(|left, right| left.file.cmp(&right.file));
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
        report.pin_mappings.sort_by(|left, right| {
            left.symbol_file
                .cmp(&right.symbol_file)
                .then(left.subckt.cmp(&right.subckt))
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

    /// error count。
    pub fn error_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
            .count()
    }

    /// warning count。
    pub fn warning_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == DiagnosticSeverity::Warning)
            .count()
    }

    /// info count。
    pub fn info_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == DiagnosticSeverity::Info)
            .count()
    }

    /// compatibility score。
    pub fn compatibility_score(&self) -> u32 {
        let penalty = self.error_count() as u32 * 25 + self.warning_count() as u32 * 8;
        100_u32.saturating_sub(penalty)
    }

    /// to json。
    pub fn to_json(&self) -> String {
        let files = self
            .files
            .iter()
            .map(ModelFileSummary::to_json)
            .collect::<Vec<_>>()
            .join(",\n");
        let symbols = self
            .symbols
            .iter()
            .map(SymbolSummary::to_json)
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
        let pin_mappings = self
            .pin_mappings
            .iter()
            .map(PinMappingSummary::to_json)
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
                "  \"symbol_count\": {},\n",
                "  \"subckt_count\": {},\n",
                "  \"model_count\": {},\n",
                "  \"pin_mapping_count\": {},\n",
                "  \"diagnostic_count\": {},\n",
                "  \"errors\": {},\n",
                "  \"warnings\": {},\n",
                "  \"infos\": {},\n",
                "  \"files\": [\n",
                "{}\n",
                "  ],\n",
                "  \"symbols\": [\n",
                "{}\n",
                "  ],\n",
                "  \"subckts\": [\n",
                "{}\n",
                "  ],\n",
                "  \"models\": [\n",
                "{}\n",
                "  ],\n",
                "  \"pin_mappings\": [\n",
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
            self.symbols.len(),
            self.subckts.len(),
            self.models.len(),
            self.pin_mappings.len(),
            self.diagnostics.len(),
            self.error_count(),
            self.warning_count(),
            self.info_count(),
            files,
            symbols,
            subckts,
            models,
            pin_mappings,
            diagnostics
        )
    }

    /// to html。
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
        let symbol_rows = if self.symbols.is_empty() {
            "<tr><td colspan=\"4\">No symbols checked.</td></tr>".to_string()
        } else {
            self.symbols
                .iter()
                .map(|symbol| {
                    format!(
                        "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                        html_escape(&symbol.file),
                        html_escape(&symbol.name.clone().unwrap_or_else(|| "unknown".to_string())),
                        symbol.pin_count,
                        html_escape(&symbol.pin_order_text())
                    )
                })
                .collect::<String>()
        };
        let pin_mapping_rows = if self.pin_mappings.is_empty() {
            "<tr><td colspan=\"6\">No pin mappings checked.</td></tr>".to_string()
        } else {
            self.pin_mappings
                .iter()
                .map(|mapping| {
                    format!(
                        "<tr class=\"{}\"><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                        html_escape(if mapping.matched { "passed" } else { "failed" }),
                        html_escape(&mapping.symbol_file),
                        html_escape(&mapping.subckt),
                        mapping.symbol_pin_count,
                        mapping.subckt_pin_count,
                        html_escape(&mapping.symbol_order.join(", ")),
                        html_escape(&mapping.subckt_order.join(", "))
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
                "<h2>Symbols</h2>",
                "<table><thead><tr><th>File</th><th>Name</th><th>Pins</th><th>Spice Order</th></tr></thead>",
                "<tbody>{}</tbody></table>",
                "<h2>Pin Mapping</h2>",
                "<table><thead><tr><th>Symbol</th><th>Subckt</th><th>Symbol Pins</th><th>Subckt Pins</th><th>Symbol Order</th><th>Subckt Order</th></tr></thead>",
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
            symbol_rows,
            pin_mapping_rows,
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
pub struct SymbolSummary {
    pub file: String,
    pub name: Option<String>,
    pub pins: Vec<SymbolPin>,
    pub pin_count: usize,
}

impl SymbolSummary {
    fn to_json(&self) -> String {
        let pins = self
            .pins
            .iter()
            .map(SymbolPin::to_json)
            .collect::<Vec<_>>()
            .join(",\n");
        format!(
            concat!(
                "    {{\n",
                "      \"file\": \"{}\",\n",
                "      \"name\": {},\n",
                "      \"pin_count\": {},\n",
                "      \"spice_order\": [{}],\n",
                "      \"pins\": [\n",
                "{}\n",
                "      ]\n",
                "    }}"
            ),
            json_escape(&self.file),
            option_string_json(self.name.as_deref()),
            self.pin_count,
            quoted_json_list(&self.ordered_pin_names()),
            pins
        )
    }

    fn ordered_pin_names(&self) -> Vec<String> {
        let mut ordered = self.pins.iter().collect::<Vec<_>>();
        ordered.sort_by_key(|pin| pin.spice_order.unwrap_or(usize::MAX));
        ordered
            .into_iter()
            .filter(|pin| pin.spice_order.is_some())
            .map(|pin| pin.name.clone().unwrap_or_else(|| pin.raw_name()))
            .collect()
    }

    fn pin_order_text(&self) -> String {
        let ordered = self.ordered_pin_names();
        if ordered.is_empty() {
            "none".to_string()
        } else {
            ordered.join(", ")
        }
    }
}

#[derive(Debug)]
pub struct SymbolPin {
    pub line: usize,
    pub name: Option<String>,
    pub spice_order: Option<usize>,
    pub x: Option<i64>,
    pub y: Option<i64>,
}

impl SymbolPin {
    fn to_json(&self) -> String {
        format!(
            "        {{ \"line\": {}, \"name\": {}, \"spice_order\": {}, \"x\": {}, \"y\": {} }}",
            self.line,
            option_string_json(self.name.as_deref()),
            option_usize_json(self.spice_order),
            option_i64_json(self.x),
            option_i64_json(self.y)
        )
    }

    fn raw_name(&self) -> String {
        self.spice_order
            .map(|order| format!("pin{}", order))
            .unwrap_or_else(|| "unnamed".to_string())
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
pub struct PinMappingSummary {
    pub symbol_file: String,
    pub subckt: String,
    pub symbol_pin_count: usize,
    pub subckt_pin_count: usize,
    pub symbol_order: Vec<String>,
    pub subckt_order: Vec<String>,
    pub matched: bool,
}

impl PinMappingSummary {
    fn to_json(&self) -> String {
        format!(
            concat!(
                "    {{ \"symbol_file\": \"{}\", \"subckt\": \"{}\", ",
                "\"symbol_pin_count\": {}, \"subckt_pin_count\": {}, ",
                "\"symbol_order\": [{}], \"subckt_order\": [{}], \"matched\": {} }}"
            ),
            json_escape(&self.symbol_file),
            json_escape(&self.subckt),
            self.symbol_pin_count,
            self.subckt_pin_count,
            quoted_json_list(&self.symbol_order),
            quoted_json_list(&self.subckt_order),
            self.matched
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

    fn rank(self) -> usize {
        match self {
            Self::Error => 0,
            Self::Warning => 1,
            Self::Info => 2,
        }
    }
}


mod vendor_import;
pub use vendor_import::{VendorKind, ImportedSubckt, ImportedModel, VendorImportResult, VendorModelCatalog, ModelCatalogEntry, import_spice_model_file, import_spice_model_dir, is_spice_model_file, build_model_catalog};

include!("model_check_impl.rs");
