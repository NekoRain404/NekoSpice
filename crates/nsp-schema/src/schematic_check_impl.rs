// Schematic diagnostic checks.
// Covers: check_duplicate_references, check_symbols, check_wires,
// check_labels, check_sheets, check_no_connects, check_buses,
// check_spice_directives.

impl NspSchematic {
    fn check_duplicate_references(&self, diagnostics: &mut Vec<NspSchematicDiagnostic>) {
        let mut counts = BTreeMap::<String, usize>::new();
        for symbol in &self.symbols {
            if let Some(reference) = symbol.reference()
                && !reference.trim().is_empty()
                && !reference.starts_with('#')
            {
                *counts.entry(reference.to_string()).or_default() += 1;
            }
        }
        for (reference, count) in counts {
            if count > 1 {
                diagnostics.push(schema_diagnostic(
                    NspDiagnosticSeverity::Error,
                    "duplicate-reference",
                    &format!("symbol reference '{reference}' appears {count} times"),
                    Some(reference),
                    None,
                    None,
                ));
            }
        }
    }

    fn check_symbols(
        &self,
        graph: &NspNetGraph,
        diagnostics: &mut Vec<NspSchematicDiagnostic>,
    ) {
        for symbol in &self.symbols {
            let reference = symbol.reference().unwrap_or("<no-reference>").to_string();
            if symbol
                .reference()
                .is_none_or(|reference| reference.trim().is_empty())
            {
                diagnostics.push(schema_diagnostic(
                    NspDiagnosticSeverity::Error,
                    "missing-reference",
                    "symbol has no Reference property",
                    Some(symbol.lib_id.clone()),
                    None,
                    None,
                ));
            }
            if symbol.value().is_none_or(|value| value.trim().is_empty()) {
                diagnostics.push(schema_diagnostic(
                    NspDiagnosticSeverity::Warning,
                    "missing-value",
                    &format!("symbol '{reference}' has no Value property"),
                    Some(reference.clone()),
                    None,
                    None,
                ));
            }

            let Some(definition) = self.symbol_definition(&symbol.lib_id) else {
                diagnostics.push(schema_diagnostic(
                    NspDiagnosticSeverity::Error,
                    "missing-symbol-definition",
                    &format!(
                        "symbol '{reference}' uses missing library symbol '{}'",
                        symbol.lib_id
                    ),
                    Some(reference),
                    None,
                    None,
                ));
                continue;
            };
            let definition = self
                .resolved_symbol_definition_with_fallback(&symbol.lib_id, symbol.lib_name.as_deref())
                .unwrap_or_else(|| NspResolvedSymbolDef::from_symbol(definition));
            if symbol.sim_enabled(Some(&definition)) == Some(false) {
                diagnostics.push(schema_diagnostic(
                    NspDiagnosticSeverity::Info,
                    "simulation-disabled",
                    &format!("symbol '{reference}' is excluded from simulation"),
                    Some(reference),
                    None,
                    None,
                ));
                continue;
            }
            if let Some(device) = symbol.sim_device(Some(&definition))
                && spice_primitive_for_device(&device).is_none()
            {
                diagnostics.push(schema_diagnostic(
                    NspDiagnosticSeverity::Error,
                    "unsupported-sim-device",
                    &format!("symbol '{reference}' uses unsupported Sim.Device '{device}'"),
                    Some(reference.clone()),
                    None,
                    None,
                ));
            }
            let Some(symbol_at) = symbol.at else {
                diagnostics.push(schema_diagnostic(
                    NspDiagnosticSeverity::Error,
                    "missing-symbol-position",
                    &format!("symbol '{reference}' has no placement"),
                    Some(reference),
                    None,
                    None,
                ));
                continue;
            };

            let mut definition_pins = definition
                .scoped_pins(symbol.unit, symbol.body_style)
                .collect::<Vec<_>>();
            definition_pins.sort_by(compare_pin_numbers);
            if !definition_pins.is_empty() && symbol.pins.is_empty() {
                diagnostics.push(schema_diagnostic(
                    NspDiagnosticSeverity::Warning,
                    "missing-pin-refs",
                    &format!("symbol '{reference}' has no instance pin UUID references"),
                    Some(reference.clone()),
                    None,
                    None,
                ));
            }
            let sim_pin_order = symbol_sim_pin_order(symbol, &definition);
            for pin_number in &sim_pin_order {
                if !definition
                    .pins
                    .iter()
                    .filter(|pin| {
                        symbol_item_scope_matches(
                            pin.unit,
                            pin.body_style,
                            symbol.unit.unwrap_or(1),
                            symbol.body_style.unwrap_or(1),
                        )
                    })
                    .any(|pin| pin.number() == pin_number || pin.name() == pin_number)
                {
                    diagnostics.push(schema_diagnostic(
                        NspDiagnosticSeverity::Error,
                        "invalid-sim-pin",
                        &format!(
                            "symbol '{reference}' Sim.Pins entry '{pin_number}' does not match a library pin"
                        ),
                        Some(reference.clone()),
                        None,
                        Some(pin_number.clone()),
                    ));
                }
            }
            for pin in definition_pins {
                let pin_label = format!("{}:{}", reference, pin.number());
                let Some(pin_at) = pin.at else {
                    diagnostics.push(schema_diagnostic(
                        NspDiagnosticSeverity::Warning,
                        "missing-pin-position",
                        &format!(
                            "symbol '{reference}' pin '{}' has no position",
                            pin.number()
                        ),
                        Some(reference.clone()),
                        None,
                        Some(pin.number().to_string()),
                    ));
                    continue;
                };
                let point = transform_symbol_point(pin_at, symbol_at, symbol.mirror.as_deref());
                if self.has_no_connect_at(point) {
                    continue;
                }
                match graph.net_at(point) {
                    Some("unconnected") | None => diagnostics.push(schema_diagnostic(
                        NspDiagnosticSeverity::Warning,
                        "unconnected-pin",
                        &format!("symbol pin '{pin_label}' is not connected to a named net"),
                        Some(reference.clone()),
                        None,
                        Some(pin.number().to_string()),
                    )),
                    Some(net) if net.starts_with('n') => {
                        diagnostics.push(schema_diagnostic(
                            NspDiagnosticSeverity::Info,
                            "generated-net-name",
                            &format!("symbol pin '{pin_label}' is on generated net '{net}'"),
                            Some(reference.clone()),
                            Some(net.to_string()),
                            Some(pin.number().to_string()),
                        ))
                    }
                    Some(_) => {}
                }
            }
        }
    }

    fn check_wires(&self, diagnostics: &mut Vec<NspSchematicDiagnostic>) {
        for (index, wire) in self.wires.iter().enumerate() {
            if wire.points.len() < 2 {
                diagnostics.push(schema_diagnostic(
                    NspDiagnosticSeverity::Error,
                    "invalid-wire",
                    &format!("wire #{index} has fewer than two points"),
                    Some(format!("wire:{index}")),
                    None,
                    None,
                ));
            }
        }
    }

    fn check_labels(&self, graph: &NspNetGraph, diagnostics: &mut Vec<NspSchematicDiagnostic>) {
        for label in &self.labels {
            if label.text.trim().is_empty() {
                diagnostics.push(schema_diagnostic(
                    NspDiagnosticSeverity::Error,
                    "empty-label",
                    "label text is empty",
                    None,
                    None,
                    None,
                ));
            }
            if let Some(at) = label.at
                && graph.net_at(at.point()).is_none()
            {
                diagnostics.push(schema_diagnostic(
                    NspDiagnosticSeverity::Warning,
                    "floating-label",
                    &format!("label '{}' is not attached to any net", label.text),
                    Some(label.text.clone()),
                    None,
                    None,
                ));
            }
        }
    }

    fn check_sheets(&self, diagnostics: &mut Vec<NspSchematicDiagnostic>) {
        for (index, sheet) in self.sheets.iter().enumerate() {
            let item = sheet
                .sheet_name()
                .or_else(|| sheet.sheet_file())
                .map(str::to_string)
                .unwrap_or_else(|| format!("sheet:{index}"));
            if sheet.sheet_name().is_none_or(|name| name.trim().is_empty()) {
                diagnostics.push(schema_diagnostic(
                    NspDiagnosticSeverity::Error,
                    "missing-sheet-name",
                    &format!("hierarchical sheet #{index} has no Sheetname property"),
                    Some(item.clone()),
                    None,
                    None,
                ));
            }
            if sheet.sheet_file().is_none_or(|file| file.trim().is_empty()) {
                diagnostics.push(schema_diagnostic(
                    NspDiagnosticSeverity::Error,
                    "missing-sheet-file",
                    &format!("hierarchical sheet '{item}' has no Sheetfile property"),
                    Some(item.clone()),
                    None,
                    None,
                ));
            }
            if sheet.at.is_none() || sheet.size.is_none() {
                diagnostics.push(schema_diagnostic(
                    NspDiagnosticSeverity::Warning,
                    "missing-sheet-geometry",
                    &format!("hierarchical sheet '{item}' has incomplete placement geometry"),
                    Some(item.clone()),
                    None,
                    None,
                ));
            }
            if sheet.exclude_from_sim == Some(true) {
                diagnostics.push(schema_diagnostic(
                    NspDiagnosticSeverity::Info,
                    "simulation-disabled-sheet",
                    &format!("hierarchical sheet '{item}' is excluded from simulation"),
                    Some(item),
                    None,
                    None,
                ));
            } else {
                diagnostics.push(schema_diagnostic(
                    NspDiagnosticSeverity::Error,
                    "hierarchical-sheet-unsupported",
                    &format!(
                        "hierarchical sheet '{item}' is parsed but child sheet expansion is not implemented yet"
                    ),
                    Some(item),
                    None,
                    None,
                ));
            }
        }
    }

    fn check_no_connects(&self, diagnostics: &mut Vec<NspSchematicDiagnostic>) {
        let pin_points = self.symbol_pin_points();
        for marker in &self.no_connects {
            if !pin_points.iter().any(|point| same_point(*point, marker.at)) {
                diagnostics.push(schema_diagnostic(
                    NspDiagnosticSeverity::Warning,
                    "floating-no-connect",
                    &format!(
                        "no-connect marker at {},{} is not attached to a symbol pin",
                        marker.at.x, marker.at.y
                    ),
                    None,
                    None,
                    None,
                ));
            }
        }
    }

    fn check_buses(&self, diagnostics: &mut Vec<NspSchematicDiagnostic>) {
        for (index, bus) in self.buses.iter().enumerate() {
            if bus.points.len() < 2 {
                diagnostics.push(schema_diagnostic(
                    NspDiagnosticSeverity::Warning,
                    "empty-bus",
                    &format!("bus #{index} has fewer than two points"),
                    Some(format!("bus:{index}")),
                    None,
                    None,
                ));
            }
        }
        for (index, entry) in self.bus_entries.iter().enumerate() {
            if !is_valid_bus_entry_size(entry.size) {
                diagnostics.push(schema_diagnostic(
                    NspDiagnosticSeverity::Warning,
                    "invalid-bus-entry-size",
                    &format!(
                        "bus entry #{index} has invalid size {},{}",
                        entry.size.width, entry.size.height
                    ),
                    Some(format!("bus-entry:{index}")),
                    None,
                    None,
                ));
            }
        }
    }

    fn check_spice_directives(&self, diagnostics: &mut Vec<NspSchematicDiagnostic>) {
        let directives = self.spice_directives();
        if directives.is_empty() {
            diagnostics.push(schema_diagnostic(
                NspDiagnosticSeverity::Warning,
                "missing-spice-directive",
                "schematic has no SPICE directives such as .tran, .ac, .dc, or .op",
                None,
                None,
                None,
            ));
            return;
        }
        if !directives
            .iter()
            .any(|directive| is_spice_analysis_directive_text(&directive.text))
        {
            diagnostics.push(schema_diagnostic(
                NspDiagnosticSeverity::Warning,
                "missing-analysis-directive",
                "schematic has SPICE text but no analysis directive (.tran, .ac, .dc, .op)",
                None,
                None,
                None,
            ));
        }
    }

}
