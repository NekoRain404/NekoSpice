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
    pub fn scan(root: &Path) -> OslResult<Self> {
        Self::scan_with_options(root, &ModelCheckOptions::default())
    }

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

fn parse_ltspice_symbol(path: &Path) -> OslResult<SymbolSummary> {
    let content = read_text(path)?;
    let file = path.display().to_string();
    let mut symbol = SymbolSummary {
        file: file.clone(),
        name: path
            .file_stem()
            .and_then(|name| name.to_str())
            .map(str::to_string),
        pins: Vec::new(),
        pin_count: 0,
    };

    for (index, raw_line) in content.lines().enumerate() {
        let line_number = index + 1;
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let tokens = line.split_whitespace().collect::<Vec<_>>();
        if tokens.is_empty() {
            continue;
        }

        match tokens[0] {
            "SYMATTR" if tokens.len() >= 3 && tokens[1] == "Prefix" && symbol.name.is_none() => {
                symbol.name = Some(tokens[2].to_string());
            }
            "PIN" => {
                let x = tokens.get(1).and_then(|value| value.parse::<i64>().ok());
                let y = tokens.get(2).and_then(|value| value.parse::<i64>().ok());
                symbol.pins.push(SymbolPin {
                    line: line_number,
                    name: None,
                    spice_order: None,
                    x,
                    y,
                });
            }
            "PINATTR" if tokens.len() >= 3 => {
                let Some(pin) = symbol.pins.last_mut() else {
                    continue;
                };
                match tokens[1] {
                    "PinName" => {
                        pin.name = Some(tokens[2..].join(" "));
                    }
                    "SpiceOrder" => {
                        pin.spice_order = tokens[2].parse::<usize>().ok();
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    symbol.pin_count = symbol.pins.len();
    Ok(symbol)
}

fn check_symbol_pin_mapping(symbol: &SymbolSummary, report: &mut ModelCheckReport) {
    if symbol.pins.is_empty() {
        push_model_diagnostic(
            report,
            &symbol.file,
            1,
            DiagnosticSeverity::Warning,
            "symbol_no_pins",
            "symbol contains no LTspice PIN entries",
            "Check that the --symbol path points to an LTspice .asy symbol file.",
        );
        return;
    }

    let Some(subckt) = select_subckt_for_symbol(symbol, &report.subckts) else {
        push_model_diagnostic(
            report,
            &symbol.file,
            1,
            DiagnosticSeverity::Warning,
            "symbol_without_subckt",
            "symbol was provided but no .subckt was found in the checked model files",
            "Pass a model library containing the target .subckt.",
        );
        return;
    };
    let subckt_name = subckt.name.clone();
    let subckt_pins = subckt.pins.clone();
    let subckt_pin_count = subckt.pin_count;

    let symbol_order = symbol.ordered_pin_names();
    let missing_order = symbol
        .pins
        .iter()
        .filter(|pin| pin.spice_order.is_none())
        .count();
    if missing_order > 0 {
        push_model_diagnostic(
            report,
            &symbol.file,
            1,
            DiagnosticSeverity::Error,
            "symbol_missing_spice_order",
            &format!(
                "{} symbol pins are missing PINATTR SpiceOrder",
                missing_order
            ),
            "Add SpiceOrder attributes to every LTspice symbol pin before mapping to a .subckt.",
        );
    }

    let matched = symbol_order == subckt_pins;
    report.pin_mappings.push(PinMappingSummary {
        symbol_file: symbol.file.clone(),
        subckt: subckt_name.clone(),
        symbol_pin_count: symbol_order.len(),
        subckt_pin_count,
        symbol_order: symbol_order.clone(),
        subckt_order: subckt_pins.clone(),
        matched,
    });

    if symbol_order.len() != subckt_pin_count {
        push_model_diagnostic(
            report,
            &symbol.file,
            1,
            DiagnosticSeverity::Error,
            "pin_count_mismatch",
            &format!(
                "symbol has {} ordered pins but .subckt '{}' has {} pins",
                symbol_order.len(),
                subckt_name,
                subckt_pin_count
            ),
            "Align LTspice SpiceOrder attributes with the .subckt pin list.",
        );
        return;
    }

    if !matched {
        push_model_diagnostic(
            report,
            &symbol.file,
            1,
            DiagnosticSeverity::Error,
            "pin_order_mismatch",
            &format!(
                "symbol SpiceOrder [{}] does not match .subckt '{}' pins [{}]",
                symbol_order.join(", "),
                subckt_name,
                subckt_pins.join(", ")
            ),
            "Reorder symbol SpiceOrder attributes or remap pins before simulation.",
        );
    }
}

fn select_subckt_for_symbol<'a>(
    symbol: &SymbolSummary,
    subckts: &'a [SubcktSummary],
) -> Option<&'a SubcktSummary> {
    if subckts.len() == 1 {
        return subckts.first();
    }
    let symbol_name = symbol.name.as_ref()?.to_ascii_lowercase();
    subckts
        .iter()
        .find(|subckt| subckt.name.to_ascii_lowercase() == symbol_name)
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

fn option_string_json(value: Option<&str>) -> String {
    value
        .map(|value| format!("\"{}\"", json_escape(value)))
        .unwrap_or_else(|| "null".to_string())
}

fn option_usize_json(value: Option<usize>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_string())
}

fn option_i64_json(value: Option<i64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_string())
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
    use super::{
        ModelCheckOptions, ModelCheckReport, normalized_spice_statement, parse_model_kind,
        spice_logical_lines,
    };
    use std::path::{Path, PathBuf};

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

    #[test]
    fn maps_ltspice_symbol_pins_to_subckt_order() {
        let root = repo_root();
        let report = ModelCheckReport::scan_with_options(
            &root.join("examples/pin_mapping/good_opamp.lib"),
            &ModelCheckOptions {
                symbol_path: Some(root.join("examples/pin_mapping/good_opamp.asy")),
            },
        )
        .unwrap();

        assert_eq!(report.error_count(), 0);
        assert_eq!(report.pin_mappings.len(), 1);
        assert!(report.pin_mappings[0].matched);
        assert_eq!(
            report.pin_mappings[0].symbol_order,
            ["IN+", "IN-", "OUT", "VCC", "VEE"]
        );
    }

    #[test]
    fn reports_ltspice_symbol_pin_order_mismatch() {
        let root = repo_root();
        let report = ModelCheckReport::scan_with_options(
            &root.join("examples/pin_mapping/good_opamp.lib"),
            &ModelCheckOptions {
                symbol_path: Some(root.join("examples/pin_mapping/bad_opamp.asy")),
            },
        )
        .unwrap();

        assert_eq!(report.error_count(), 1);
        assert!(!report.pin_mappings[0].matched);
        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "pin_order_mismatch")
        );
    }

    fn repo_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("crate should live under crates/osl-model")
            .to_path_buf()
    }
}
