// Symbol library resolution and definition merging.
// Covers: configured_symbol_pin_refs, connectivity_graph, canvas_scene,
// check_report, symbol_definition, resolve_project_symbol_libraries,
// merge_library_symbol.

impl NspSchematic {
    fn configured_symbol_pin_refs(
        &self,
        current_symbol: &NspSymbolInstance,
        definition: &NspResolvedSymbolDef,
        unit: u32,
        body_style: Option<u32>,
        pin_alternates: &BTreeMap<String, String>,
    ) -> OslResult<Vec<NspSymbolPinRef>> {
        let mut sorted_pins = definition
            .scoped_pins(Some(unit), body_style)
            .collect::<Vec<_>>();
        sorted_pins.sort_by(compare_pin_numbers);
        for pin_number in pin_alternates.keys() {
            let Some(pin) = sorted_pins
                .iter()
                .find(|pin| pin.number() == pin_number.as_str())
            else {
                return Err(OslError::InvalidInput(format!(
                    "schema symbol pin '{pin_number}' is not present in selected unit/body style"
                )));
            };
            let alternate = pin_alternates
                .get(pin_number)
                .expect("pin alternate was just looked up");
            if !pin
                .alternates
                .iter()
                .any(|candidate| candidate.name == *alternate)
            {
                return Err(OslError::InvalidInput(format!(
                    "schema symbol pin '{pin_number}' has no alternate '{alternate}'"
                )));
            }
        }

        let mut existing_by_number = current_symbol
            .pins
            .iter()
            .filter_map(|pin| Some((pin.number.clone()?, pin.uuid.clone())))
            .collect::<BTreeMap<_, _>>();
        let instance_uuid = current_symbol
            .uuid
            .as_deref()
            .unwrap_or(current_symbol.lib_id.as_str());
        let mut generated_pin_uuids = BTreeSet::new();
        let mut pins = Vec::new();
        for (index, pin) in sorted_pins.into_iter().enumerate() {
            let pin_number = pin.number().to_string();
            let pin_uuid = existing_by_number.remove(&pin_number).flatten();
            let pin_uuid = match pin_uuid {
                Some(pin_uuid) if generated_pin_uuids.insert(pin_uuid.clone()) => pin_uuid,
                _ => {
                    let pin_uuid = self.edit_uuid_excluding(
                        None,
                        "symbol-pin",
                        &format!("{instance_uuid}:{pin_number}:{index}"),
                        &generated_pin_uuids,
                    )?;
                    generated_pin_uuids.insert(pin_uuid.clone());
                    pin_uuid
                }
            };
            pins.push(NspSymbolPinRef {
                number: Some(pin_number.clone()),
                uuid: Some(pin_uuid),
                alternate: pin_alternates.get(&pin_number).cloned(),
            });
        }

        Ok(pins)
    }

    pub fn connectivity_graph(&self) -> NspNetGraph {
        NspNetGraph::build(self)
    }

    pub fn canvas_scene(&self) -> NspCanvasScene {
        NspCanvasScene::from_schematic(self)
    }

    pub fn check_report(&self) -> NspSchematicCheckReport {
        let graph = self.connectivity_graph();
        let mut diagnostics = Vec::new();

        self.check_duplicate_references(&mut diagnostics);
        self.check_symbols(&graph, &mut diagnostics);
        self.check_wires(&mut diagnostics);
        self.check_buses(&mut diagnostics);
        self.check_labels(&graph, &mut diagnostics);
        self.check_sheets(&mut diagnostics);
        self.check_no_connects(&mut diagnostics);
        self.check_spice_directives(&mut diagnostics);
        if !graph.nets.iter().any(|net| net.name == "0") {
            diagnostics.push(schema_diagnostic(
                NspDiagnosticSeverity::Error,
                "missing-ground",
                "schematic has no net labelled 0 or ground",
                None,
                None,
                None,
            ));
        }

        NspSchematicCheckReport {
            source: self.source.clone(),
            symbol_count: self.symbols.len(),
            sheet_count: self.sheets.len(),
            net_count: graph.nets.len(),
            spice_directive_count: self.spice_directives().len(),
            diagnostics,
        }
    }

    fn symbol_definition(&self, lib_id: &str) -> Option<&NspSymbolDef> {
        self.library_symbols
            .iter()
            .find(|symbol| symbol.name == lib_id)
    }

    fn resolved_symbol_definition(&self, lib_id: &str) -> Option<NspResolvedSymbolDef> {
        let definition = self.symbol_definition(lib_id)?;
        resolve_symbol_definition(definition, &self.library_symbols)
    }

    pub fn resolve_project_symbol_libraries(
        &mut self,
        project_dir: &Path,
    ) -> OslResult<Vec<NspLibraryDiagnostic>> {
        let table_path = project_dir.join("sym-lib-table");
        if !table_path.exists() {
            return Ok(Vec::new());
        }
        self.resolve_missing_symbol_definitions_from_table(&table_path)
    }

    pub fn resolve_missing_symbol_definitions_from_table(
        &mut self,
        table_path: &Path,
    ) -> OslResult<Vec<NspLibraryDiagnostic>> {
        let table = read_symbol_library_table(table_path)?;
        let base_dir = table_path.parent().unwrap_or_else(|| Path::new("."));
        let mut diagnostics = Vec::new();
        let mut missing = self.missing_symbol_lib_ids();

        for row in table.libraries {
            if missing.is_empty() {
                break;
            }
            if row.disabled {
                diagnostics.push(NspLibraryDiagnostic {
                    library: row.name.clone(),
                    severity: NspDiagnosticSeverity::Info,
                    message: "library row is disabled".to_string(),
                });
                continue;
            }
            if !row.library_type.eq_ignore_ascii_case("KiCad") {
                diagnostics.push(NspLibraryDiagnostic {
                    library: row.name.clone(),
                    severity: NspDiagnosticSeverity::Warning,
                    message: format!("unsupported symbol library type '{}'", row.library_type),
                });
                continue;
            }

            let resolved_path = resolve_uri(&row.uri, base_dir);
            match read_symbol_library(&resolved_path) {
                Ok(library) => {
                    let mut resolved = Vec::new();
                    for lib_id in &missing {
                        if let Some(definition) =
                            library_symbol_definition_for_lib_id(&library, &row.name, lib_id)
                        {
                            self.merge_library_symbol_with_parents(definition, &library, &row.name);
                            resolved.push(lib_id.clone());
                        }
                    }
                    for lib_id in resolved {
                        missing.remove(&lib_id);
                    }
                }
                Err(error) => diagnostics.push(NspLibraryDiagnostic {
                    library: row.name,
                    severity: NspDiagnosticSeverity::Error,
                    message: format!("failed to load {}: {}", resolved_path.display(), error),
                }),
            }
        }

        Ok(diagnostics)
    }

    fn missing_symbol_lib_ids(&self) -> BTreeSet<String> {
        self.symbols
            .iter()
            .map(|symbol| symbol.lib_id.clone())
            .filter(|lib_id| self.symbol_definition(lib_id).is_none())
            .collect()
    }

    fn merge_library_symbol(&mut self, definition: NspSymbolDef) -> bool {
        if self.symbol_definition(&definition.name).is_some() {
            return false;
        }
        self.library_symbols.push(definition);
        true
    }

    fn merge_symbol_placement_library_symbol(
        &mut self,
        definition: &NspSymbolDef,
    ) -> OslResult<()> {
        match self
            .library_symbols
            .iter()
            .find(|symbol| symbol.name == definition.name)
        {
            Some(existing) if !library_symbol_definitions_are_compatible(existing, definition) => {
                Err(OslError::InvalidInput(format!(
                    "schema embedded library symbol '{}' already exists with different content",
                    definition.name
                )))
            }
            Some(_) => Ok(()),
            None => {
                self.library_symbols.push(definition.clone());
                Ok(())
            }
        }
    }

    fn merge_library_symbol_with_parents(
        &mut self,
        mut definition: NspSymbolDef,
        library: &NspSymbolLibrary,
        library_name: &str,
    ) {
        qualify_library_symbol_name(&mut definition, library_name);
        let mut pending = vec![definition];
        let mut visited = BTreeSet::new();
        while let Some(definition) = pending.pop() {
            if !visited.insert(definition.name.clone()) {
                continue;
            }
            if let Some(parent_name) = definition.extends.as_deref()
                && let Some(parent) =
                    find_symbol_inheritance_parent(&definition, parent_name, &library.symbols)
            {
                let mut parent = parent.clone();
                qualify_library_symbol_name(&mut parent, library_name);
                pending.push(parent);
            }
            self.merge_library_symbol(definition);
        }
    }

}
