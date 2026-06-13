//! KiCad to SPICE netlist export.

use crate::connectivity::normalize_net_name;
use crate::diagnostics::kicad_schematic_diagnostic;
use crate::simulation::is_spice_analysis_directive_text;
use crate::symbols::symbol_ordered_pins;
use crate::transform::transform_symbol_point;
use crate::{
    KicadDiagnosticSeverity, KicadHierarchyNetlist, KicadNetGraph, KicadSchematic,
    KicadSchematicCheckReport, KicadSchematicDiagnostic, KicadSheet, KicadSymbolInstance,
    read_kicad_schematic_with_libraries,
};
use osl_core::OslResult;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

impl KicadSchematic {
    /// check report with hierarchy。
    pub fn check_report_with_hierarchy(
        &self,
        base_dir: &Path,
    ) -> OslResult<KicadSchematicCheckReport> {
        let graph = self.connectivity_graph();
        let exported = self.to_spice_netlist_with_hierarchy(base_dir)?;
        Ok(KicadSchematicCheckReport {
            source: self.source.clone(),
            symbol_count: self.symbols.len(),
            sheet_count: self.sheets.len(),
            net_count: graph.nets.len(),
            spice_directive_count: count_spice_directive_lines(&exported.netlist),
            diagnostics: exported.diagnostics,
        })
    }

    /// to spice netlist。
    pub fn to_spice_netlist(&self) -> OslResult<String> {
        let graph = self.connectivity_graph();
        let mut lines = vec![format!("* Imported from KiCad schematic: {}", self.source)];

        lines.extend(self.spice_include_directives());

        for sheet in &self.sheets {
            if sheet.exclude_from_sim == Some(true) {
                continue;
            }
            lines.push(format!(
                "* Unsupported KiCad hierarchical sheet {} {}",
                sheet.sheet_name().unwrap_or("<unnamed-sheet>"),
                sheet.sheet_file().unwrap_or("<no-sheetfile>")
            ));
        }

        for symbol in &self.symbols {
            let definition = self.resolved_symbol_definition(&symbol.lib_id);
            match self.symbol_to_spice_line(symbol, &graph) {
                Some(line) => lines.push(line),
                None if symbol.sim_enabled(definition.as_ref()) == Some(false) => {}
                None => {
                    if let Some(line) = self.symbol_to_spice_line_legacy(symbol, &graph) {
                        lines.push(line);
                    } else {
                        lines.push(format!(
                            "* Unsupported KiCad symbol {} {}",
                            symbol.reference().unwrap_or("<no-reference>"),
                            symbol.lib_id
                        ));
                    }
                }
            }
        }

        let mut has_end = false;
        for directive in self.spice_directives() {
            let directive = directive.text.trim();
            if directive.eq_ignore_ascii_case(".end") {
                has_end = true;
            }
            lines.push(directive.to_string());
        }
        if !has_end {
            lines.push(".end".to_string());
        }
        Ok(format!("{}\n", lines.join("\n")))
    }

    /// to spice netlist with hierarchy。
    pub fn to_spice_netlist_with_hierarchy(
        &self,
        base_dir: &Path,
    ) -> OslResult<KicadHierarchyNetlist> {
        let mut export = KicadHierarchyExport::new();
        let root_diagnostics = self.check_report().diagnostics;
        export.export_schematic(self, base_dir, "root", &BTreeMap::new())?;

        let has_spice_directive = !export.directives.is_empty();
        let has_analysis_directive = export
            .directives
            .iter()
            .any(|directive| is_spice_analysis_directive_text(directive));
        let mut lines = vec![format!("* Imported from KiCad schematic: {}", self.source)];
        lines.extend(export.includes);
        lines.extend(export.components);
        lines.extend(export.directives);
        if !lines
            .iter()
            .any(|line| line.trim().eq_ignore_ascii_case(".end"))
        {
            lines.push(".end".to_string());
        }

        let mut diagnostics = root_diagnostics
            .into_iter()
            .filter(|diagnostic| {
                !is_hierarchy_root_nonfatal_diagnostic(
                    diagnostic,
                    has_spice_directive,
                    has_analysis_directive,
                )
            })
            .collect::<Vec<_>>();
        diagnostics.extend(export.diagnostics);

        Ok(KicadHierarchyNetlist {
            netlist: format!("{}\n", lines.join("\n")),
            diagnostics,
        })
    }

    /// spice include directives。
    pub fn spice_include_directives(&self) -> Vec<String> {
        let mut includes = BTreeSet::new();
        for symbol in &self.symbols {
            let definition = self.resolved_symbol_definition(&symbol.lib_id);
            if symbol.sim_enabled(definition.as_ref()) == Some(false) {
                continue;
            }
            if let Some(path) = symbol
                .sim_library(definition.as_ref())
                .filter(|path| !path.trim().is_empty())
            {
                includes.insert(path.trim().to_string());
            }
        }
        includes
            .into_iter()
            .map(|path| format!(".include {}", quote_spice_path(&path)))
            .collect()
    }

    fn symbol_to_spice_line(
        &self,
        symbol: &KicadSymbolInstance,
        graph: &KicadNetGraph,
    ) -> Option<String> {
        let nodes = self.symbol_pin_nets(symbol, graph)?;
        self.symbol_to_spice_line_with_nodes(symbol, &nodes)
    }

    fn symbol_to_spice_line_with_nodes(
        &self,
        symbol: &KicadSymbolInstance,
        nodes: &[String],
    ) -> Option<String> {
        let definition = self.resolved_symbol_definition(&symbol.lib_id);
        if symbol.sim_enabled(definition.as_ref()) == Some(false) {
            return None;
        }

        let reference = symbol.reference()?.trim();
        if reference.is_empty() || reference.starts_with('#') {
            return None;
        }

        let has_explicit_sim_model = symbol.has_explicit_sim_model(definition.as_ref());
        let model = symbol.sim_model_value(definition.as_ref());
        let params = symbol.sim_params_value(definition.as_ref());
        let value = compose_spice_model_value(
            model.as_deref(),
            params.as_deref(),
            has_explicit_sim_model.then(|| symbol.value().unwrap_or_default().trim()),
        );
        let explicit_device = symbol.sim_device(definition.as_ref());
        let device = explicit_device
            .clone()
            .or_else(|| {
                has_explicit_sim_model.then(|| {
                    reference
                        .chars()
                        .next()
                        .map(|character| character.to_ascii_uppercase().to_string())
                        .unwrap_or_default()
                })
            })?
            .to_ascii_uppercase();
        let primitive = if explicit_device.is_some() {
            spice_primitive_for_device(&device)?
        } else {
            reference
                .chars()
                .next()
                .map(|character| character.to_ascii_uppercase().to_string())
                .unwrap_or_default()
        };
        if primitive.is_empty() {
            return None;
        }
        let spice_reference = spice_item_name(reference, &primitive);

        if primitive == "X" || device == "SUBCKT" {
            if nodes.is_empty() || value.is_empty() {
                return None;
            }
            return Some(format!("{} {} {}", spice_reference, nodes.join(" "), value));
        }
        if primitive == "SPICE" {
            if value.is_empty() {
                return None;
            }
            return Some(expand_spice_template(&value, &spice_reference, nodes));
        }

        match primitive.as_str() {
            "R" | "C" | "L" | "V" | "I" | "D" if nodes.len() >= 2 && !value.is_empty() => Some(
                format!("{spice_reference} {} {} {value}", nodes[0], nodes[1]),
            ),
            "Q" | "J" if nodes.len() >= 3 && !value.is_empty() => Some(format!(
                "{spice_reference} {} {} {} {value}",
                nodes[0], nodes[1], nodes[2]
            )),
            "M" if nodes.len() >= 4 && !value.is_empty() => Some(format!(
                "{spice_reference} {} {} {} {} {value}",
                nodes[0], nodes[1], nodes[2], nodes[3]
            )),
            "S" | "W" if nodes.len() >= 4 && !value.is_empty() => Some(format!(
                "{spice_reference} {} {} {} {} {value}",
                nodes[0], nodes[1], nodes[2], nodes[3]
            )),
            "E" | "F" | "G" | "H" if nodes.len() >= 4 && !value.is_empty() => Some(format!(
                "{spice_reference} {} {} {} {} {value}",
                nodes[0], nodes[1], nodes[2], nodes[3]
            )),
            "T" if nodes.len() >= 4 && !value.is_empty() => Some(format!(
                "{spice_reference} {} {} {} {} {value}",
                nodes[0], nodes[1], nodes[2], nodes[3]
            )),
            "K" if !value.is_empty() => Some(format!("{spice_reference} {value}")),
            _ => None,
        }
    }

    fn symbol_to_spice_line_legacy(
        &self,
        symbol: &KicadSymbolInstance,
        graph: &KicadNetGraph,
    ) -> Option<String> {
        let nodes = self.symbol_pin_nets(symbol, graph)?;
        self.symbol_to_spice_line_legacy_with_nodes(symbol, &nodes)
    }

    fn symbol_to_spice_line_legacy_with_nodes(
        &self,
        symbol: &KicadSymbolInstance,
        nodes: &[String],
    ) -> Option<String> {
        let reference = symbol.reference()?.trim();
        if reference.is_empty() || reference.starts_with('#') {
            return None;
        }

        let value = symbol.value().unwrap_or_default().trim();
        let designator = reference
            .chars()
            .next()
            .map(|character| character.to_ascii_uppercase())?;

        match designator {
            'R' | 'C' | 'L' | 'V' | 'I' if nodes.len() >= 2 && !value.is_empty() => {
                Some(format!("{reference} {} {} {value}", nodes[0], nodes[1]))
            }
            'D' if nodes.len() >= 2 && !value.is_empty() => {
                Some(format!("{reference} {} {} {value}", nodes[0], nodes[1]))
            }
            'Q' | 'J' if nodes.len() >= 3 && !value.is_empty() => Some(format!(
                "{reference} {} {} {} {value}",
                nodes[0], nodes[1], nodes[2]
            )),
            'M' if nodes.len() >= 4 && !value.is_empty() => Some(format!(
                "{reference} {} {} {} {} {value}",
                nodes[0], nodes[1], nodes[2], nodes[3]
            )),
            'X' if !nodes.is_empty() && !value.is_empty() => {
                Some(format!("{reference} {} {value}", nodes.join(" ")))
            }
            _ => None,
        }
    }

    fn symbol_pin_nets(
        &self,
        symbol: &KicadSymbolInstance,
        graph: &KicadNetGraph,
    ) -> Option<Vec<String>> {
        let symbol_at = symbol.at?;
        let definition = self.resolved_symbol_definition(&symbol.lib_id)?;
        let pins = symbol_ordered_pins(symbol, &definition);

        Some(
            pins.into_iter()
                .map(|pin| {
                    pin.at
                        .map(|pin_at| {
                            transform_symbol_point(pin_at, symbol_at, symbol.mirror.as_deref())
                        })
                        .and_then(|point| graph.net_at(point).map(str::to_string))
                        .unwrap_or_else(|| "unconnected".to_string())
                })
                .collect(),
        )
    }
}

struct KicadHierarchyExport {
    includes: BTreeSet<String>,
    components: Vec<String>,
    directives: Vec<String>,
    diagnostics: Vec<KicadSchematicDiagnostic>,
    visited: BTreeSet<PathBuf>,
}

impl KicadHierarchyExport {
    fn new() -> Self {
        Self {
            includes: BTreeSet::new(),
            components: Vec::new(),
            directives: Vec::new(),
            diagnostics: Vec::new(),
            visited: BTreeSet::new(),
        }
    }

    fn export_schematic(
        &mut self,
        schematic: &KicadSchematic,
        base_dir: &Path,
        scope: &str,
        net_aliases: &BTreeMap<String, String>,
    ) -> OslResult<()> {
        let graph = schematic.connectivity_graph();
        self.includes.extend(schematic.spice_include_directives());
        for symbol in &schematic.symbols {
            let Some(nodes) = schematic.symbol_pin_nets(symbol, &graph) else {
                continue;
            };
            let mapped_nodes = nodes
                .iter()
                .map(|node| scoped_net_name(scope, node, net_aliases))
                .collect::<Vec<_>>();
            let scoped_symbol = scoped_symbol_instance(symbol, scope);
            let definition = schematic.resolved_symbol_definition(&symbol.lib_id);
            match schematic.symbol_to_spice_line_with_nodes(&scoped_symbol, &mapped_nodes) {
                Some(line) => self.components.push(line),
                None if scoped_symbol.sim_enabled(definition.as_ref()) == Some(false) => {}
                None => {
                    if let Some(line) = schematic
                        .symbol_to_spice_line_legacy_with_nodes(&scoped_symbol, &mapped_nodes)
                    {
                        self.components.push(line);
                    } else {
                        self.components.push(format!(
                            "* Unsupported KiCad symbol {} {}",
                            scoped_symbol.reference().unwrap_or("<no-reference>"),
                            scoped_symbol.lib_id
                        ));
                    }
                }
            }
        }

        for sheet in &schematic.sheets {
            if sheet.exclude_from_sim == Some(true) {
                continue;
            }
            let Some(sheet_file) = sheet.sheet_file().filter(|file| !file.trim().is_empty()) else {
                continue;
            };
            let sheet_path = base_dir.join(sheet_file);
            let visit_key = fs::canonicalize(&sheet_path).unwrap_or_else(|_| sheet_path.clone());
            if !self.visited.insert(visit_key.clone()) {
                self.diagnostics.push(kicad_schematic_diagnostic(
                    KicadDiagnosticSeverity::Error,
                    "hierarchical-sheet-cycle",
                    &format!(
                        "hierarchical sheet '{}' was already visited",
                        sheet_path.display()
                    ),
                    sheet.sheet_name().map(str::to_string),
                    None,
                    None,
                ));
                continue;
            }

            match read_kicad_schematic_with_libraries(&sheet_path) {
                Ok(child) => {
                    self.diagnostics.extend(
                        child
                            .check_report()
                            .diagnostics
                            .into_iter()
                            .filter(|diagnostic| !is_child_sheet_nonfatal_diagnostic(diagnostic)),
                    );
                    let aliases =
                        self.sheet_net_aliases(schematic, sheet, &graph, scope, net_aliases);
                    let child_scope = child_sheet_scope(scope, sheet);
                    let child_base_dir = sheet_path.parent().unwrap_or(base_dir);
                    self.export_schematic(&child, child_base_dir, &child_scope, &aliases)?;
                }
                Err(error) => self.diagnostics.push(kicad_schematic_diagnostic(
                    KicadDiagnosticSeverity::Error,
                    "missing-child-sheet",
                    &format!(
                        "failed to load hierarchical sheet {}: {}",
                        sheet_path.display(),
                        error
                    ),
                    sheet.sheet_name().map(str::to_string),
                    None,
                    None,
                )),
            }
            self.visited.remove(&visit_key);
        }

        for directive in schematic.spice_directives() {
            let directive = directive.text.trim();
            if directive.eq_ignore_ascii_case(".end") {
                continue;
            }
            if !self
                .directives
                .iter()
                .any(|existing| existing.eq_ignore_ascii_case(directive))
            {
                self.directives.push(directive.to_string());
            }
        }

        Ok(())
    }

    fn sheet_net_aliases(
        &mut self,
        schematic: &KicadSchematic,
        sheet: &KicadSheet,
        graph: &KicadNetGraph,
        scope: &str,
        parent_aliases: &BTreeMap<String, String>,
    ) -> BTreeMap<String, String> {
        let mut aliases = BTreeMap::new();
        for pin in &sheet.pins {
            let Some(at) = pin.at else {
                continue;
            };
            match graph.net_at(at.point()) {
                Some(net) => {
                    aliases.insert(
                        normalize_net_name(&pin.name),
                        scoped_net_name(scope, net, parent_aliases),
                    );
                }
                None => self.diagnostics.push(kicad_schematic_diagnostic(
                    KicadDiagnosticSeverity::Warning,
                    "unconnected-sheet-pin",
                    &format!(
                        "hierarchical sheet '{}' pin '{}' is not connected to a parent net",
                        sheet.sheet_name().unwrap_or("<unnamed-sheet>"),
                        pin.name
                    ),
                    sheet.sheet_name().map(str::to_string),
                    None,
                    Some(pin.name.clone()),
                )),
            }
        }
        if aliases.is_empty() && !sheet.pins.is_empty() {
            self.diagnostics.push(kicad_schematic_diagnostic(
                KicadDiagnosticSeverity::Warning,
                "unmapped-sheet-pins",
                &format!(
                    "hierarchical sheet '{}' has pins but no parent net aliases were mapped",
                    sheet.sheet_name().unwrap_or("<unnamed-sheet>")
                ),
                sheet.sheet_name().map(str::to_string),
                None,
                None,
            ));
        }
        if sheet.pins.is_empty() && !schematic.sheets.is_empty() {
            self.diagnostics.push(kicad_schematic_diagnostic(
                KicadDiagnosticSeverity::Info,
                "sheet-without-pins",
                &format!(
                    "hierarchical sheet '{}' has no sheet pins",
                    sheet.sheet_name().unwrap_or("<unnamed-sheet>")
                ),
                sheet.sheet_name().map(str::to_string),
                None,
                None,
            ));
        }
        aliases
    }
}

fn count_spice_directive_lines(netlist: &str) -> usize {
    netlist
        .lines()
        .filter(|line| {
            let line = line.trim_start();
            line.starts_with('.') && !line.eq_ignore_ascii_case(".end")
        })
        .count()
}

fn is_child_sheet_nonfatal_diagnostic(diagnostic: &KicadSchematicDiagnostic) -> bool {
    matches!(
        diagnostic.code.as_str(),
        "hierarchical-sheet-unsupported"
            | "simulation-disabled-sheet"
            | "missing-spice-directive"
            | "missing-analysis-directive"
            | "missing-ground"
    )
}

fn is_hierarchy_root_nonfatal_diagnostic(
    diagnostic: &KicadSchematicDiagnostic,
    has_spice_directive: bool,
    has_analysis_directive: bool,
) -> bool {
    matches!(
        diagnostic.code.as_str(),
        "hierarchical-sheet-unsupported" | "simulation-disabled-sheet"
    ) || (diagnostic.code == "missing-spice-directive" && has_spice_directive)
        || (diagnostic.code == "missing-analysis-directive" && has_analysis_directive)
}

fn scoped_net_name(scope: &str, net: &str, aliases: &BTreeMap<String, String>) -> String {
    if net == "0" || net.eq_ignore_ascii_case("gnd") {
        return "0".to_string();
    }
    if let Some(alias) = aliases.get(&normalize_net_name(net)) {
        return alias.clone();
    }
    if scope == "root" || net == "unconnected" {
        return net.to_string();
    }
    format!(
        "{}_{}",
        sanitize_spice_identifier(scope),
        sanitize_spice_identifier(net)
    )
}

fn child_sheet_scope(parent_scope: &str, sheet: &KicadSheet) -> String {
    let sheet_name = sheet
        .sheet_name()
        .or_else(|| sheet.sheet_file())
        .unwrap_or("sheet");
    let sheet_name = sanitize_spice_identifier(sheet_name);
    if parent_scope == "root" {
        sheet_name
    } else {
        format!("{}_{}", sanitize_spice_identifier(parent_scope), sheet_name)
    }
}

fn scoped_symbol_instance(symbol: &KicadSymbolInstance, scope: &str) -> KicadSymbolInstance {
    if scope == "root" {
        return symbol.clone();
    }

    let mut symbol = symbol.clone();
    if let Some(reference) = symbol.reference().map(str::to_string)
        && !reference.trim().is_empty()
        && !reference.starts_with('#')
    {
        let scoped_reference = scoped_reference(&reference, scope);
        if let Some(property) = symbol
            .properties
            .iter_mut()
            .find(|property| property.name == "Reference")
        {
            property.value = scoped_reference;
        }
    }
    symbol
}

fn scoped_reference(reference: &str, scope: &str) -> String {
    let mut chars = reference.chars();
    let Some(prefix) = chars.next() else {
        return reference.to_string();
    };
    let suffix = chars.collect::<String>();
    format!(
        "{}{}_{}",
        prefix,
        sanitize_spice_identifier(scope),
        sanitize_spice_identifier(&suffix)
    )
}

fn sanitize_spice_identifier(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '_' {
                character
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string();
    if sanitized.is_empty() {
        "item".to_string()
    } else {
        sanitized
    }
}

/// spice primitive for device。
pub(crate) fn spice_primitive_for_device(device: &str) -> Option<String> {
    let device = device.to_ascii_uppercase();
    let primitive = match device.as_str() {
        "R" | "RES" | "RESISTOR" => "R",
        "C" | "CAP" | "CAPACITOR" => "C",
        "L" | "IND" | "INDUCTOR" => "L",
        "V" | "VSOURCE" | "VOLTAGE" => "V",
        "I" | "ISOURCE" | "CURRENT" => "I",
        "D" | "DIODE" => "D",
        "NPN" | "PNP" | "BJT" => "Q",
        "NJFET" | "PJFET" | "JFET" => "J",
        "NMOS" | "PMOS" | "NMES" | "PMES" | "MOSFET" => "M",
        "SW" | "SWITCH" => "S",
        "CSW" | "CURRENT_SWITCH" => "W",
        "VCVS" => "E",
        "CCCS" => "F",
        "VCCS" => "G",
        "CCVS" => "H",
        "TLINE" | "TRANSMISSION_LINE" => "T",
        "K" | "COUPLED_INDUCTOR" => "K",
        "SUBCKT" => "X",
        "SPICE" => "SPICE",
        "" => return None,
        other if other.len() == 1 => other,
        _ => return None,
    };
    Some(primitive.to_string())
}

fn compose_spice_model_value(
    model: Option<&str>,
    params: Option<&str>,
    fallback: Option<&str>,
) -> String {
    match (
        model.filter(|value| !value.is_empty()),
        params.filter(|value| !value.is_empty()),
    ) {
        (Some(model), Some(params)) => format!("{model} {params}"),
        (Some(model), None) => model.to_string(),
        (None, Some(params)) => params.to_string(),
        (None, None) => fallback.unwrap_or_default().to_string(),
    }
}

fn spice_item_name(reference: &str, primitive: &str) -> String {
    let Some(first) = primitive.chars().next() else {
        return reference.to_string();
    };
    if reference
        .chars()
        .next()
        .is_some_and(|character| character.eq_ignore_ascii_case(&first))
    {
        reference.to_string()
    } else {
        format!("{first}{reference}")
    }
}

fn expand_spice_template(template: &str, reference: &str, nodes: &[String]) -> String {
    let mut expanded = template.replace("${REFERENCE}", reference);
    for (index, node) in nodes.iter().enumerate() {
        expanded = expanded.replace(&format!("${{N{}}}", index + 1), node);
    }
    expanded
}

fn quote_spice_path(path: &str) -> String {
    if path
        .bytes()
        .any(|byte| byte.is_ascii_whitespace() || byte == b'"')
    {
        format!("\"{}\"", path.replace('"', "\\\""))
    } else {
        format!("\"{}\"", path)
    }
}
