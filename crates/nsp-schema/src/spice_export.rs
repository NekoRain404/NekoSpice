//! schema to SPICE netlist export.

use crate::connectivity::normalize_net_name;
use crate::diagnostics::schema_diagnostic;
use crate::simulation::is_spice_analysis_directive_text;
use crate::spice_export_helpers::{
    child_sheet_scope, compose_spice_model_value, convert_spice_params,
    count_spice_directive_lines, expand_path_env_vars, expand_spice_template, extract_named_f64,
    extract_named_freq, infer_spice_primitive_from_lib_id, is_child_sheet_nonfatal_diagnostic,
    is_hierarchy_root_nonfatal_diagnostic, normalize_source_value, parse_spice_value,
    quote_spice_path, sanitize_spice_identifier, scoped_net_name, scoped_reference,
    scoped_symbol_instance, spice_item_name, spice_primitive_for_device,
};
use crate::symbols::{NspSymbolPropertySource, symbol_ordered_pins};
use crate::transform::transform_symbol_point;
use crate::{
    NspDiagnosticSeverity, NspHierarchyNetlist, NspNetGraph, NspSchematic, NspSchematicCheckReport,
    NspSchematicDiagnostic, NspSheet, NspSymbolInstance, read_schematic_with_libraries,
};
use nsp_core::OslResult;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

impl NspSchematic {
    /// check report with hierarchy。
    pub fn check_report_with_hierarchy(
        &self,
        base_dir: &Path,
    ) -> OslResult<NspSchematicCheckReport> {
        let graph = self.connectivity_graph();
        let exported = self.to_spice_netlist_with_hierarchy(base_dir)?;
        Ok(NspSchematicCheckReport {
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
        let mut lines = vec![format!("* Imported from schema schematic: {}", self.source)];

        lines.extend(self.spice_include_directives());

        for sheet in &self.sheets {
            if sheet.exclude_from_sim == Some(true) {
                continue;
            }
            lines.push(format!(
                "* Unsupported schema hierarchical sheet {} {}",
                sheet.sheet_name().unwrap_or("<unnamed-sheet>"),
                sheet.sheet_file().unwrap_or("<no-sheetfile>")
            ));
        }

        let mut ref_usage: std::collections::HashMap<String, u32> =
            std::collections::HashMap::new();
        for symbol in &self.symbols {
            let definition = self.resolved_symbol_definition_with_fallback(
                &symbol.lib_id,
                symbol.lib_name.as_deref(),
            );
            match self.symbol_to_spice_line_dedup(symbol, &graph, &mut ref_usage) {
                Some(line) => lines.push(line),
                None if symbol.sim_enabled(definition.as_ref()) == Some(false) => {}
                None => {
                    let ref_name = symbol.reference().unwrap_or_default().trim();
                    let lib = symbol.lib_id.trim();
                    if ref_name.starts_with('#')
                        || lib.contains("power:")
                        || lib.starts_with("power:")
                    {
                        continue;
                    }
                    if let Some(line) =
                        self.symbol_to_spice_line_legacy_dedup(symbol, &graph, &mut ref_usage)
                    {
                        lines.push(line);
                    } else {
                        lines.push(format!("* Unsupported schema symbol {} {}", ref_name, lib));
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

        // Inject default model definitions for commonly used device types
        // that appear in the netlist but have no corresponding .model or .include
        let netlist_text = lines.join("\n");
        let default_models = [
            ("NMOS", ".model NMOS NMOS LEVEL=1"),
            ("PMOS", ".model PMOS PMOS LEVEL=1"),
            ("NPN", ".model NPN NPN LEVEL=1"),
            ("PNP", ".model PNP PNP LEVEL=1"),
            ("D", ".model D D LEVEL=1"),
        ];
        // Built-in subcircuit models for common KiCad symbols
        let builtin_subcircuits = [(
            "kicad_builtin_opamp",
            ".subckt kicad_builtin_opamp in+ in- v+ v- out PARAMS: POLE=100k GAIN=100k VOFF=0 ROUT=75\n                 E_op out 0 in+ in- 1\n                 ROUT out 0 ROUT\n                 .ends kicad_builtin_opamp",
        )];
        let mut injected_subs = Vec::new();
        for (name, def) in &builtin_subcircuits {
            let used = netlist_text
                .lines()
                .any(|line| line.trim().contains(name) && !line.trim().starts_with('*'));
            let has_subckt = netlist_text.lines().any(|line| {
                let t = line.trim().to_ascii_lowercase();
                t.starts_with(".subckt") && t.contains(name)
            });
            if used && !has_subckt {
                injected_subs.push(def.to_string());
            }
        }
        let mut injected = Vec::new();
        for (name, def) in &default_models {
            // Check if model name appears as a component value (after node names)
            // e.g., "Q1 0 gg sout NMOS" or "D1 anode cathode D"
            let used = netlist_text.lines().any(|line| {
                let trimmed = line.trim();
                if trimmed.starts_with('*') || trimmed.starts_with('.') {
                    return false;
                }
                // Check if the last token matches the model name
                trimmed.split_whitespace().last() == Some(name)
            });
            let has_model = netlist_text
                .lines()
                .any(|line| line.trim().starts_with(&format!(".model {}", name)));
            if used && !has_model {
                injected.push(def.to_string());
            }
        }
        if !has_end {
            lines.push(".end".to_string());
        }
        // Inject built-in subcircuit definitions before .end
        if !injected_subs.is_empty()
            && let Some(end_idx) = lines
                .iter()
                .rposition(|l| l.trim().eq_ignore_ascii_case(".end"))
        {
            for (i, sub) in injected_subs.iter().enumerate() {
                lines.insert(end_idx + i, sub.clone());
            }
        }
        // Inject default model definitions before .end
        if !injected.is_empty()
            && let Some(end_idx) = lines
                .iter()
                .rposition(|l| l.trim().eq_ignore_ascii_case(".end"))
        {
            for (i, model) in injected.iter().enumerate() {
                lines.insert(end_idx + i, model.clone());
            }
        }
        Ok(format!("{}\n", lines.join("\n")))
    }

    /// to spice netlist with hierarchy。
    pub fn to_spice_netlist_with_hierarchy(
        &self,
        base_dir: &Path,
    ) -> OslResult<NspHierarchyNetlist> {
        let mut export = NspHierarchyExport::new();
        let root_diagnostics = self.check_report().diagnostics;
        export.export_schematic(self, base_dir, "root", &BTreeMap::new())?;

        let has_spice_directive = !export.directives.is_empty();
        let has_analysis_directive = export
            .directives
            .iter()
            .any(|directive| is_spice_analysis_directive_text(directive));
        let mut lines = vec![format!("* Imported from schema schematic: {}", self.source)];
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

        Ok(NspHierarchyNetlist {
            netlist: format!("{}\n", lines.join("\n")),
            diagnostics,
        })
    }

    /// spice include directives。
    pub fn spice_include_directives(&self) -> Vec<String> {
        let mut includes = BTreeSet::new();
        for symbol in &self.symbols {
            let definition = self.resolved_symbol_definition_with_fallback(
                &symbol.lib_id,
                symbol.lib_name.as_deref(),
            );
            if symbol.sim_enabled(definition.as_ref()) == Some(false) {
                continue;
            }
            if let Some(path) = symbol
                .sim_library(definition.as_ref())
                .filter(|path| !path.trim().is_empty())
            {
                includes.insert(path.trim().to_string());
            }
            // Also extract lib= from Sim.Params for symbols that use
            // the KiCad Sim.Params format: type="X" model="..." lib="..."
            if let Some(raw) = symbol.property("Sim.Params").or_else(|| {
                definition
                    .as_ref()
                    .and_then(|d| d.property_value("Sim.Params"))
            }) {
                for token in raw.split(|c: char| c.is_ascii_whitespace()) {
                    if let Some((key, val)) = token.split_once('=') {
                        if key.trim().eq_ignore_ascii_case("lib") {
                            let lib_path = val.trim_matches('"').trim();
                            if !lib_path.is_empty() {
                                includes.insert(lib_path.to_string());
                            }
                        }
                    }
                }
            }
        }
        includes
            .into_iter()
            .map(|path| expand_path_env_vars(&path))
            .filter(|path| !path.is_empty())
            .map(|path| format!(".include {}", quote_spice_path(&path)))
            .collect()
    }

    fn symbol_to_spice_line(
        &self,
        symbol: &NspSymbolInstance,
        graph: &NspNetGraph,
    ) -> Option<String> {
        let nodes = self.symbol_pin_nets(symbol, graph)?;
        self.symbol_to_spice_line_with_nodes(symbol, &nodes)
    }

    pub(crate) fn symbol_to_spice_line_with_nodes(
        &self,
        symbol: &NspSymbolInstance,
        nodes: &[String],
    ) -> Option<String> {
        let definition = self
            .resolved_symbol_definition_with_fallback(&symbol.lib_id, symbol.lib_name.as_deref());
        if symbol.sim_enabled(definition.as_ref()) == Some(false) {
            return None;
        }

        let reference = symbol.reference()?.trim();
        if reference.is_empty() || reference.starts_with('#') {
            return None;
        }

        let has_explicit_sim_model = symbol.has_explicit_sim_model(definition.as_ref());
        let model = symbol
            .sim_model_value(definition.as_ref())
            .map(|m| normalize_source_value(&m));
        // Read raw Sim.Params before stripping — needed for type= extraction.
        // First check the symbol's own properties, then the definition.
        let raw_params = symbol
            .property("Sim.Params")
            .or_else(|| {
                definition
                    .as_ref()
                    .and_then(|d| d.property_value("Sim.Params"))
            })
            .map(str::to_string);
        if symbol.value().unwrap_or_default().contains("2ED2109") {
            eprintln!(
                "[DEBUG] ref={} lib_id={} has_raw={:?} raw={:?} has_explicit={}",
                reference,
                symbol.lib_id,
                raw_params.is_some(),
                raw_params,
                has_explicit_sim_model
            );
        }
        let params = symbol.sim_params_value(definition.as_ref());
        // Get Sim.Type for SPICE parameter conversion
        let sim_type = symbol.sim_type(definition.as_ref());

        // Convert named params to SPICE positional format
        let converted_params = if let Some(ref ptype) = sim_type {
            params
                .as_ref()
                .map(|p| convert_spice_params(Some(ptype.as_str()), p))
        } else {
            params.clone()
        };

        let value = compose_spice_model_value(
            model.as_deref(),
            converted_params.as_deref(),
            has_explicit_sim_model.then(|| symbol.value().unwrap_or_default().trim()),
        );
        let explicit_device = symbol.sim_device(definition.as_ref());
        let mut device = explicit_device
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
        // Track whether device was derived from Sim.Params type= so we can
        // distinguish it from the reference-prefix fallback below.
        let mut device_from_params = false;
        // Fallback: extract type= from raw Sim.Params when device is still just
        // the reference prefix (e.g. "U" or "Q") and not a known SPICE type.
        // KiCad Sim.Params format: type="X" model="2ED2109S06F" lib="..."
        if device.len() <= 1 || spice_primitive_for_device(&device).is_none() {
            if let Some(ref raw) = raw_params {
                for token in raw.split(|c: char| c.is_ascii_whitespace()) {
                    if let Some((key, val)) = token.split_once('=') {
                        if key.trim().eq_ignore_ascii_case("type") {
                            let val_upper = val.trim_matches('"').trim().to_ascii_uppercase();
                            if spice_primitive_for_device(&val_upper).is_some() {
                                device = val_upper;
                                device_from_params = true;
                                break;
                            }
                        }
                    }
                }
            }
        }
        // When Sim.Device = "SPICE", it means a custom template. But if the
        // resolved value doesn't contain SPICE template placeholders, fall back
        // to Spice_Primitive from the definition (which gives the actual type).
        if device == "SPICE" && !value.contains("${REFERENCE}") && !value.contains("${N") {
            if let Some(def) = &definition
                && let Some(prim) = def
                    .property("Spice_Primitive")
                    .map(str::trim)
                    .filter(|v| !v.is_empty())
            {
                device = prim.to_ascii_uppercase();
            }
        }
        // Use spice_primitive_for_device when the device was explicitly set,
        // or when it was derived from Sim.Device / Sim.Params type= / Spice_Primitive.
        // The reference prefix fallback only applies when device == reference_prefix.
        let use_explicit_primitive =
            explicit_device.is_some() || device == "SPICE" || device_from_params;
        let primitive = if use_explicit_primitive {
            spice_primitive_for_device(&device)?
        } else {
            // No explicit Sim.Device — infer from lib_id when possible.
            // KiCad uses Q prefix for all transistors including MOSFETs,
            // but SPICE needs M for MOSFETs and J for JFETs.
            let lib_id_upper = symbol.lib_id.to_ascii_uppercase();
            if let Some(inferred) = spice_primitive_for_device(&lib_id_upper) {
                inferred
            } else if let Some(prim) = infer_spice_primitive_from_lib_id(&symbol.lib_id) {
                prim.to_string()
            } else {
                reference
                    .chars()
                    .next()
                    .map(|character| character.to_ascii_uppercase().to_string())
                    .unwrap_or_default()
            }
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
            // If the template has no SPICE placeholders, try to resolve the
            // actual device type from raw Sim.Params type= field.
            if !value.contains("${REFERENCE}") && !value.contains("${N") {
                if let Some(ref raw) = raw_params {
                    for token in raw.split(|c: char| c.is_ascii_whitespace()) {
                        if let Some((key, val)) = token.split_once('=') {
                            if key.trim().eq_ignore_ascii_case("type") {
                                let val_upper = val.trim_matches('"').trim().to_ascii_uppercase();
                                if let Some(resolved_prim) = spice_primitive_for_device(&val_upper)
                                {
                                    // Build proper SPICE subcircuit call for X prefix
                                    if resolved_prim == "X" {
                                        let model_name = model.as_deref().unwrap_or(&value);
                                        let x_ref = spice_item_name(reference, "X");
                                        if !nodes.is_empty() && !model_name.is_empty() {
                                            return Some(format!(
                                                "{} {} {}",
                                                x_ref,
                                                nodes.join(" "),
                                                model_name
                                            ));
                                        }
                                    }
                                    return Some(expand_spice_template(
                                        &value,
                                        &spice_reference,
                                        nodes,
                                    ));
                                }
                            }
                        }
                    }
                }
            }
            return Some(expand_spice_template(&value, &spice_reference, nodes));
        }

        // When Sim.Device is set but no model name, provide sensible defaults
        let effective_value = if value.is_empty() && explicit_device.is_some() {
            match device.as_str() {
                "D" => "D".to_string(),
                "Q" | "NPN" | "PNP" | "NMOS" | "PMOS" => device.clone(),
                "M" => "NMOS".to_string(),
                _ => value.clone(),
            }
        } else {
            value.clone()
        };

        match primitive.as_str() {
            "R" | "C" | "L" | "V" | "I" | "D"
                if nodes.len() >= 2 && !effective_value.is_empty() =>
            {
                Some(format!(
                    "{spice_reference} {} {} {effective_value}",
                    nodes[0], nodes[1]
                ))
            }
            "Q" | "J" if nodes.len() >= 3 && !effective_value.is_empty() => Some(format!(
                "{spice_reference} {} {} {} {effective_value}",
                nodes[0], nodes[1], nodes[2]
            )),
            "M" if nodes.len() >= 3 && !effective_value.is_empty() => {
                // MOSFET requires 4 nodes: drain gate source bulk
                // KiCad symbols often omit bulk pin; default to ground (0)
                let bulk = if nodes.len() >= 4 {
                    nodes[3].clone()
                } else {
                    "0".to_string()
                };
                Some(format!(
                    "{spice_reference} {} {} {} {} {effective_value}",
                    nodes[0], nodes[1], nodes[2], bulk
                ))
            }
            "S" | "W" if nodes.len() >= 4 && !effective_value.is_empty() => Some(format!(
                "{spice_reference} {} {} {} {} {effective_value}",
                nodes[0], nodes[1], nodes[2], nodes[3]
            )),
            "E" | "F" | "G" | "H" if nodes.len() >= 4 && !effective_value.is_empty() => {
                Some(format!(
                    "{spice_reference} {} {} {} {} {effective_value}",
                    nodes[0], nodes[1], nodes[2], nodes[3]
                ))
            }
            "T" if nodes.len() >= 4 && !effective_value.is_empty() => Some(format!(
                "{spice_reference} {} {} {} {} {effective_value}",
                nodes[0], nodes[1], nodes[2], nodes[3]
            )),
            "K" if !effective_value.is_empty() => {
                Some(format!("{spice_reference} {effective_value}"))
            }
            _ => None,
        }
    }

    fn symbol_to_spice_line_legacy(
        &self,
        symbol: &NspSymbolInstance,
        graph: &NspNetGraph,
    ) -> Option<String> {
        let nodes = self.symbol_pin_nets(symbol, graph)?;
        self.symbol_to_spice_line_legacy_with_nodes(symbol, &nodes)
    }

    pub(crate) fn symbol_to_spice_line_legacy_with_nodes(
        &self,
        symbol: &NspSymbolInstance,
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

    /// SPICE line with reference deduplication for multi-unit symbols.
    fn symbol_to_spice_line_dedup(
        &self,
        symbol: &NspSymbolInstance,
        graph: &NspNetGraph,
        ref_usage: &mut std::collections::HashMap<String, u32>,
    ) -> Option<String> {
        let nodes = self.symbol_pin_nets(symbol, graph)?;
        let line = self.symbol_to_spice_line_with_nodes(symbol, &nodes)?;
        let ref_key = symbol.reference().unwrap_or_default().trim().to_string();
        if ref_key.is_empty() || ref_key.starts_with('#') {
            return Some(line);
        }
        let count = ref_usage.entry(ref_key).or_insert(0);
        *count += 1;
        if *count > 1 {
            let suffix = format!("_{}", count);
            let parts: Vec<&str> = line.splitn(2, ' ').collect();
            if parts.len() == 2 {
                return Some(format!("{}{} {}", parts[0], suffix, parts[1]));
            }
        }
        Some(line)
    }

    /// Legacy SPICE line with reference deduplication.
    fn symbol_to_spice_line_legacy_dedup(
        &self,
        symbol: &NspSymbolInstance,
        graph: &NspNetGraph,
        ref_usage: &mut std::collections::HashMap<String, u32>,
    ) -> Option<String> {
        let line = self.symbol_to_spice_line_legacy(symbol, graph)?;
        let ref_key = symbol.reference().unwrap_or_default().trim().to_string();
        if ref_key.is_empty() || ref_key.starts_with('#') {
            return Some(line);
        }
        let count = ref_usage.entry(ref_key).or_insert(0);
        *count += 1;
        if *count > 1 {
            let suffix = format!("_{}", count);
            let parts: Vec<&str> = line.splitn(2, ' ').collect();
            if parts.len() == 2 {
                return Some(format!("{}{} {}", parts[0], suffix, parts[1]));
            }
        }
        Some(line)
    }

    fn symbol_pin_nets(
        &self,
        symbol: &NspSymbolInstance,
        graph: &NspNetGraph,
    ) -> Option<Vec<String>> {
        let symbol_at = symbol.at?;
        let definition = self
            .resolved_symbol_definition_with_fallback(&symbol.lib_id, symbol.lib_name.as_deref())?;
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

struct NspHierarchyExport {
    includes: BTreeSet<String>,
    components: Vec<String>,
    directives: Vec<String>,
    diagnostics: Vec<NspSchematicDiagnostic>,
    visited: BTreeSet<PathBuf>,
}

impl NspHierarchyExport {
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
        schematic: &NspSchematic,
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
            let definition = schematic.resolved_symbol_definition_with_fallback(
                &symbol.lib_id,
                symbol.lib_name.as_deref(),
            );
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
                            "* Unsupported schema symbol {} {}",
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
                self.diagnostics.push(schema_diagnostic(
                    NspDiagnosticSeverity::Error,
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

            match read_schematic_with_libraries(&sheet_path) {
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
                Err(error) => self.diagnostics.push(schema_diagnostic(
                    NspDiagnosticSeverity::Error,
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
        schematic: &NspSchematic,
        sheet: &NspSheet,
        graph: &NspNetGraph,
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
                None => self.diagnostics.push(schema_diagnostic(
                    NspDiagnosticSeverity::Warning,
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
            self.diagnostics.push(schema_diagnostic(
                NspDiagnosticSeverity::Warning,
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
            self.diagnostics.push(schema_diagnostic(
                NspDiagnosticSeverity::Info,
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
