use osl_core::{OslError, OslResult, json_escape, read_text, write_text};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub enum Sexp {
    Atom(String),
    List(Vec<Sexp>),
}

pub fn parse_sexpr(input: &str) -> OslResult<Sexp> {
    let mut parser = SexpParser { input, offset: 0 };
    let expr = parser.parse_expr()?;
    parser.skip_ws_and_comments();
    if parser.offset != input.len() {
        return Err(OslError::InvalidInput(format!(
            "unexpected trailing KiCad S-expression data at byte {}",
            parser.offset
        )));
    }
    Ok(expr)
}

pub fn read_kicad_schematic(path: &Path) -> OslResult<KicadSchematic> {
    let content = read_text(path)?;
    parse_kicad_schematic(&content, &path.display().to_string())
}

pub fn read_kicad_schematic_with_libraries(path: &Path) -> OslResult<KicadSchematic> {
    let mut schematic = read_kicad_schematic(path)?;
    if let Some(project_dir) = path.parent() {
        schematic.resolve_project_symbol_libraries(project_dir)?;
    }
    Ok(schematic)
}

pub fn read_kicad_schematic_hierarchy_netlist(path: &Path) -> OslResult<KicadHierarchyNetlist> {
    let schematic = read_kicad_schematic_with_libraries(path)?;
    let base_dir = path.parent().unwrap_or_else(|| Path::new("."));
    schematic.to_spice_netlist_with_hierarchy(base_dir)
}

pub fn read_kicad_project(path: &Path) -> OslResult<KicadProject> {
    let content = read_text(path)?;
    parse_kicad_project(&content, &path.display().to_string())
}

pub fn write_kicad_schematic(path: &Path, schematic: &KicadSchematic) -> OslResult<()> {
    write_text(path, &schematic.to_kicad_schematic_sexpr())
}

pub fn read_kicad_symbol_library(path: &Path) -> OslResult<KicadSymbolLibrary> {
    let content = read_text(path)?;
    parse_kicad_symbol_library(&content, &path.display().to_string())
}

pub fn write_kicad_symbol_library(path: &Path, library: &KicadSymbolLibrary) -> OslResult<()> {
    write_text(path, &library.to_kicad_symbol_library_sexpr())
}

pub fn read_kicad_symbol_library_table(path: &Path) -> OslResult<KicadSymbolLibraryTable> {
    let content = read_text(path)?;
    parse_kicad_symbol_library_table(&content, &path.display().to_string())
}

pub fn read_kicad_symbol_library_index(path: &Path) -> OslResult<KicadSymbolLibraryIndex> {
    let table = read_kicad_symbol_library_table(path)?;
    let base_dir = path.parent().unwrap_or_else(|| Path::new("."));
    Ok(KicadSymbolLibraryIndex::from_table(table, base_dir))
}

pub fn parse_kicad_schematic(input: &str, source: &str) -> OslResult<KicadSchematic> {
    let root = parse_sexpr(input)?;
    let root_list = expect_root_list(&root, "kicad_sch")?;
    let library_symbols = direct_children(root_list, "lib_symbols")
        .flat_map(|lib_symbols| direct_children(list_items(lib_symbols), "symbol"))
        .filter_map(parse_symbol_def)
        .collect::<Vec<_>>();

    Ok(KicadSchematic {
        source: source.to_string(),
        version: child_value(root_list, "version"),
        generator: child_value(root_list, "generator"),
        uuid: child_value(root_list, "uuid"),
        paper: child_value(root_list, "paper"),
        library_symbols,
        symbols: direct_children(root_list, "symbol")
            .filter_map(parse_symbol_instance)
            .collect(),
        wires: direct_children(root_list, "wire")
            .map(parse_wire)
            .collect::<Vec<_>>(),
        labels: direct_children(root_list, "label")
            .filter_map(|node| parse_label(node, KicadLabelKind::Local))
            .chain(
                direct_children(root_list, "global_label")
                    .filter_map(|node| parse_label(node, KicadLabelKind::Global)),
            )
            .chain(
                direct_children(root_list, "hierarchical_label")
                    .filter_map(|node| parse_label(node, KicadLabelKind::Hierarchical)),
            )
            .collect(),
        sheets: direct_children(root_list, "sheet")
            .filter_map(parse_sheet)
            .collect(),
        text_items: direct_children(root_list, "text")
            .filter_map(parse_text_item)
            .collect(),
        junctions: direct_children(root_list, "junction")
            .filter_map(parse_junction)
            .collect(),
    })
}

pub fn parse_kicad_symbol_library(input: &str, source: &str) -> OslResult<KicadSymbolLibrary> {
    let root = parse_sexpr(input)?;
    let root_list = expect_root_list(&root, "kicad_symbol_lib")?;

    Ok(KicadSymbolLibrary {
        source: source.to_string(),
        version: child_value(root_list, "version"),
        generator: child_value(root_list, "generator"),
        symbols: direct_children(root_list, "symbol")
            .filter_map(parse_symbol_def)
            .collect(),
    })
}

pub fn parse_kicad_symbol_library_table(
    input: &str,
    source: &str,
) -> OslResult<KicadSymbolLibraryTable> {
    let root = parse_sexpr(input)?;
    let root_list = expect_root_list(&root, "sym_lib_table")?;

    Ok(KicadSymbolLibraryTable {
        source: source.to_string(),
        version: child_value(root_list, "version"),
        libraries: direct_children(root_list, "lib")
            .filter_map(parse_symbol_library_table_row)
            .collect(),
    })
}

pub fn parse_kicad_project(input: &str, source: &str) -> OslResult<KicadProject> {
    let root: serde_json::Value = serde_json::from_str(input).map_err(|error| {
        OslError::InvalidInput(format!(
            "failed to parse KiCad project JSON {source}: {error}"
        ))
    })?;
    if !root.is_object() {
        return Err(OslError::InvalidInput(format!(
            "expected KiCad project JSON object in {source}"
        )));
    }

    Ok(KicadProject {
        source: source.to_string(),
        meta_filename: json_path_string(&root, &["meta", "filename"]),
        meta_version: json_path_u64(&root, &["meta", "version"]),
        project_name: json_path_string(&root, &["project", "name"]),
        schematic_page_layout_descr_file: json_path_string(
            &root,
            &["schematic", "page_layout_descr_file"],
        ),
        sheets: parse_kicad_project_sheets(&root),
        text_variable_count: root
            .get("text_variables")
            .and_then(|value| value.as_object())
            .map(|variables| variables.len())
            .unwrap_or(0),
    })
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadSchematic {
    pub source: String,
    pub version: Option<String>,
    pub generator: Option<String>,
    pub uuid: Option<String>,
    pub paper: Option<String>,
    pub library_symbols: Vec<KicadSymbolDef>,
    pub symbols: Vec<KicadSymbolInstance>,
    pub wires: Vec<KicadWire>,
    pub labels: Vec<KicadLabel>,
    pub sheets: Vec<KicadSheet>,
    pub text_items: Vec<KicadTextItem>,
    pub junctions: Vec<KicadJunction>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadProject {
    pub source: String,
    pub meta_filename: Option<String>,
    pub meta_version: Option<u64>,
    pub project_name: Option<String>,
    pub schematic_page_layout_descr_file: Option<String>,
    pub sheets: Vec<KicadProjectSheet>,
    pub text_variable_count: usize,
}

impl KicadProject {
    pub fn schematic_stem_candidates(&self) -> Vec<String> {
        let mut candidates = Vec::new();
        push_unique_nonempty(&mut candidates, self.project_name.as_deref());
        push_unique_nonempty(
            &mut candidates,
            self.meta_filename
                .as_deref()
                .and_then(path_stem_from_string)
                .as_deref(),
        );
        push_unique_nonempty(
            &mut candidates,
            path_stem_from_string(&self.source).as_deref(),
        );
        candidates
    }

    pub fn to_summary_json(&self) -> String {
        let sheet_names = self
            .sheets
            .iter()
            .map(|sheet| format!("\"{}\"", json_escape(&sheet.name)))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            concat!(
                "{{\n",
                "  \"source\": \"{}\",\n",
                "  \"meta_filename\": {},\n",
                "  \"meta_version\": {},\n",
                "  \"project_name\": {},\n",
                "  \"schematic_page_layout_descr_file\": {},\n",
                "  \"sheet_count\": {},\n",
                "  \"sheet_names\": [{}],\n",
                "  \"text_variable_count\": {}\n",
                "}}"
            ),
            json_escape(&self.source),
            json_option(self.meta_filename.as_deref()),
            json_u64_option(self.meta_version),
            json_option(self.project_name.as_deref()),
            json_option(self.schematic_page_layout_descr_file.as_deref()),
            self.sheets.len(),
            sheet_names,
            self.text_variable_count,
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadProjectSheet {
    pub uuid: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum KicadSchematicEdit {
    MoveSymbol {
        reference: String,
        to: KicadPoint,
        rotation: Option<f64>,
    },
    SetSymbolProperty {
        reference: String,
        name: String,
        value: String,
        at: Option<KicadAt>,
    },
    PlaceSymbol {
        definition: KicadSymbolDef,
        reference: String,
        value: String,
        at: KicadAt,
        unit: Option<u32>,
        uuid: Option<String>,
    },
    AddWire {
        points: Vec<KicadPoint>,
        uuid: Option<String>,
    },
    AddLabel {
        text: String,
        kind: KicadLabelKind,
        at: KicadAt,
        uuid: Option<String>,
    },
    AddText {
        text: String,
        at: KicadAt,
        uuid: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadEditSummary {
    pub operation: String,
    pub target: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadSchematicCheckReport {
    pub source: String,
    pub symbol_count: usize,
    pub sheet_count: usize,
    pub net_count: usize,
    pub spice_directive_count: usize,
    pub diagnostics: Vec<KicadSchematicDiagnostic>,
}

impl KicadSchematicCheckReport {
    pub fn error_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == KicadDiagnosticSeverity::Error)
            .count()
    }

    pub fn warning_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == KicadDiagnosticSeverity::Warning)
            .count()
    }

    pub fn info_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == KicadDiagnosticSeverity::Info)
            .count()
    }

    pub fn to_json(&self) -> String {
        let diagnostics = self
            .diagnostics
            .iter()
            .map(|diagnostic| {
                format!(
                    concat!(
                        "    {{ \"severity\": \"{}\", \"code\": \"{}\", ",
                        "\"message\": \"{}\", \"item\": {}, \"net\": {}, \"pin\": {} }}"
                    ),
                    diagnostic.severity.as_str(),
                    json_escape(&diagnostic.code),
                    json_escape(&diagnostic.message),
                    json_option(diagnostic.item.as_deref()),
                    json_option(diagnostic.net.as_deref()),
                    json_option(diagnostic.pin.as_deref())
                )
            })
            .collect::<Vec<_>>()
            .join(",\n");

        format!(
            concat!(
                "{{\n",
                "  \"source\": \"{}\",\n",
                "  \"symbol_count\": {},\n",
                "  \"sheet_count\": {},\n",
                "  \"net_count\": {},\n",
                "  \"spice_directive_count\": {},\n",
                "  \"diagnostic_count\": {},\n",
                "  \"error_count\": {},\n",
                "  \"warning_count\": {},\n",
                "  \"info_count\": {},\n",
                "  \"diagnostics\": [\n",
                "{}\n",
                "  ]\n",
                "}}"
            ),
            json_escape(&self.source),
            self.symbol_count,
            self.sheet_count,
            self.net_count,
            self.spice_directive_count,
            self.diagnostics.len(),
            self.error_count(),
            self.warning_count(),
            self.info_count(),
            diagnostics
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadHierarchyNetlist {
    pub netlist: String,
    pub diagnostics: Vec<KicadSchematicDiagnostic>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadSchematicDiagnostic {
    pub severity: KicadDiagnosticSeverity,
    pub code: String,
    pub message: String,
    pub item: Option<String>,
    pub net: Option<String>,
    pub pin: Option<String>,
}

impl KicadSchematic {
    pub fn apply_edit(&mut self, edit: KicadSchematicEdit) -> OslResult<KicadEditSummary> {
        match edit {
            KicadSchematicEdit::MoveSymbol {
                reference,
                to,
                rotation,
            } => self.move_symbol(&reference, to, rotation),
            KicadSchematicEdit::SetSymbolProperty {
                reference,
                name,
                value,
                at,
            } => self.set_symbol_property(&reference, &name, &value, at),
            KicadSchematicEdit::PlaceSymbol {
                definition,
                reference,
                value,
                at,
                unit,
                uuid,
            } => self.place_symbol(definition, &reference, &value, at, unit, uuid),
            KicadSchematicEdit::AddWire { points, uuid } => self.add_wire(points, uuid),
            KicadSchematicEdit::AddLabel {
                text,
                kind,
                at,
                uuid,
            } => self.add_label(text, kind, at, uuid),
            KicadSchematicEdit::AddText { text, at, uuid } => self.add_text(text, at, uuid),
        }
    }

    pub fn move_symbol(
        &mut self,
        reference: &str,
        to: KicadPoint,
        rotation: Option<f64>,
    ) -> OslResult<KicadEditSummary> {
        validate_point(to, "symbol target")?;
        let index = self.symbol_index_by_reference(reference)?;
        let symbol = &mut self.symbols[index];
        let old_at = symbol.at.unwrap_or(KicadAt {
            x: 0.0,
            y: 0.0,
            rotation: 0.0,
        });
        let dx = to.x - old_at.x;
        let dy = to.y - old_at.y;
        symbol.at = Some(KicadAt {
            x: to.x,
            y: to.y,
            rotation: rotation.unwrap_or(old_at.rotation),
        });

        for property in &mut symbol.properties {
            if let Some(at) = &mut property.at {
                at.x += dx;
                at.y += dy;
            }
        }

        Ok(KicadEditSummary {
            operation: "move-symbol".to_string(),
            target: reference.to_string(),
        })
    }

    pub fn set_symbol_property(
        &mut self,
        reference: &str,
        name: &str,
        value: &str,
        at: Option<KicadAt>,
    ) -> OslResult<KicadEditSummary> {
        if name.trim().is_empty() {
            return Err(OslError::InvalidInput(
                "KiCad symbol property name must not be empty".to_string(),
            ));
        }
        if let Some(at) = at {
            validate_at(at, "symbol property")?;
        }

        let index = self.symbol_index_by_reference(reference)?;
        let symbol = &mut self.symbols[index];
        if let Some(property) = symbol
            .properties
            .iter_mut()
            .find(|property| property.name == name)
        {
            property.value = value.to_string();
            if let Some(at) = at {
                property.at = Some(at);
            }
        } else {
            symbol.properties.push(KicadProperty {
                name: name.to_string(),
                value: value.to_string(),
                at,
            });
        }

        Ok(KicadEditSummary {
            operation: "set-property".to_string(),
            target: format!("{reference}.{name}"),
        })
    }

    pub fn place_symbol(
        &mut self,
        definition: KicadSymbolDef,
        reference: &str,
        value: &str,
        at: KicadAt,
        unit: Option<u32>,
        uuid: Option<String>,
    ) -> OslResult<KicadEditSummary> {
        validate_at(at, "symbol placement")?;
        if reference.trim().is_empty() {
            return Err(OslError::InvalidInput(
                "KiCad placed symbol reference must not be empty".to_string(),
            ));
        }
        if self
            .symbols
            .iter()
            .any(|symbol| symbol.reference() == Some(reference))
        {
            return Err(OslError::InvalidInput(format!(
                "KiCad symbol reference '{reference}' already exists"
            )));
        }

        let lib_id = definition.name.clone();
        match self
            .library_symbols
            .iter()
            .find(|symbol| symbol.name == lib_id)
        {
            Some(existing) if existing != &definition => {
                return Err(OslError::InvalidInput(format!(
                    "KiCad embedded library symbol '{lib_id}' already exists with different content"
                )));
            }
            Some(_) => {}
            None => self.library_symbols.push(definition.clone()),
        }

        let instance_payload = format!(
            "{}:{}:{}@{},{},{}",
            lib_id, reference, value, at.x, at.y, at.rotation
        );
        let instance_uuid = self.edit_uuid(uuid, "symbol", &instance_payload)?;
        let properties = symbol_instance_properties(&definition, reference, value, at);
        let mut sorted_pins = definition.pins.iter().collect::<Vec<_>>();
        sorted_pins.sort_by(compare_pin_numbers);
        let mut generated_pin_uuids = BTreeSet::new();
        let mut pins = Vec::new();
        for (index, pin) in sorted_pins.into_iter().enumerate() {
            let pin_uuid = self.edit_uuid_excluding(
                None,
                "symbol-pin",
                &format!("{instance_uuid}:{}:{index}", pin.number),
                &generated_pin_uuids,
            )?;
            generated_pin_uuids.insert(pin_uuid.clone());
            pins.push(KicadSymbolPinRef {
                number: Some(pin.number.clone()),
                uuid: Some(pin_uuid),
            });
        }

        self.symbols.push(KicadSymbolInstance {
            lib_id: lib_id.clone(),
            at: Some(at),
            unit: Some(unit.unwrap_or(1)),
            uuid: Some(instance_uuid),
            exclude_from_sim: None,
            properties,
            pins,
        });

        Ok(KicadEditSummary {
            operation: "place-symbol".to_string(),
            target: format!("{reference} {lib_id}"),
        })
    }

    pub fn add_wire(
        &mut self,
        points: Vec<KicadPoint>,
        uuid: Option<String>,
    ) -> OslResult<KicadEditSummary> {
        if points.len() < 2 {
            return Err(OslError::InvalidInput(
                "KiCad wire edit requires at least two points".to_string(),
            ));
        }
        for point in &points {
            validate_point(*point, "wire point")?;
        }

        let payload = points_payload(&points);
        let uuid = Some(self.edit_uuid(uuid, "wire", &payload)?);
        self.wires.push(KicadWire { points, uuid });

        Ok(KicadEditSummary {
            operation: "add-wire".to_string(),
            target: payload,
        })
    }

    pub fn add_label(
        &mut self,
        text: impl Into<String>,
        kind: KicadLabelKind,
        at: KicadAt,
        uuid: Option<String>,
    ) -> OslResult<KicadEditSummary> {
        validate_at(at, "label")?;
        let text = text.into();
        if text.trim().is_empty() {
            return Err(OslError::InvalidInput(
                "KiCad label text must not be empty".to_string(),
            ));
        }

        let payload = format!("{}@{},{},{}", text, at.x, at.y, at.rotation);
        let uuid = Some(self.edit_uuid(uuid, kind.sexpr_name(), &payload)?);
        self.labels.push(KicadLabel {
            text: text.clone(),
            kind,
            at: Some(at),
            uuid,
        });

        Ok(KicadEditSummary {
            operation: "add-label".to_string(),
            target: text,
        })
    }

    pub fn add_text(
        &mut self,
        text: impl Into<String>,
        at: KicadAt,
        uuid: Option<String>,
    ) -> OslResult<KicadEditSummary> {
        validate_at(at, "text")?;
        let text = text.into();
        if text.trim().is_empty() {
            return Err(OslError::InvalidInput(
                "KiCad text item must not be empty".to_string(),
            ));
        }

        let payload = format!("{}@{},{},{}", text, at.x, at.y, at.rotation);
        let uuid = Some(self.edit_uuid(uuid, "text", &payload)?);
        self.text_items.push(KicadTextItem {
            text: text.clone(),
            at: Some(at),
            uuid,
        });

        Ok(KicadEditSummary {
            operation: "add-text".to_string(),
            target: text,
        })
    }

    pub fn connectivity_graph(&self) -> KicadNetGraph {
        KicadNetGraph::build(self)
    }

    pub fn canvas_scene(&self) -> KicadCanvasScene {
        KicadCanvasScene::from_schematic(self)
    }

    pub fn check_report(&self) -> KicadSchematicCheckReport {
        let graph = self.connectivity_graph();
        let mut diagnostics = Vec::new();

        self.check_duplicate_references(&mut diagnostics);
        self.check_symbols(&graph, &mut diagnostics);
        self.check_wires(&mut diagnostics);
        self.check_labels(&graph, &mut diagnostics);
        self.check_sheets(&mut diagnostics);
        self.check_spice_directives(&mut diagnostics);
        if !graph.nets.iter().any(|net| net.name == "0") {
            diagnostics.push(kicad_schematic_diagnostic(
                KicadDiagnosticSeverity::Error,
                "missing-ground",
                "schematic has no net labelled 0 or ground",
                None,
                None,
                None,
            ));
        }

        KicadSchematicCheckReport {
            source: self.source.clone(),
            symbol_count: self.symbols.len(),
            sheet_count: self.sheets.len(),
            net_count: graph.nets.len(),
            spice_directive_count: self.spice_directives().len(),
            diagnostics,
        }
    }

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
            match self.symbol_to_spice_line(symbol, &graph) {
                Some(line) => lines.push(line),
                None if symbol.sim_enabled(self.symbol_definition(&symbol.lib_id))
                    == Some(false) => {}
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
            .any(|directive| is_spice_analysis_directive(directive));
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

    pub fn spice_directives(&self) -> Vec<&KicadTextItem> {
        self.text_items
            .iter()
            .filter(|item| item.text.trim_start().starts_with('.'))
            .collect()
    }

    pub fn to_kicad_schematic_sexpr(&self) -> String {
        let mut output = String::new();
        output.push_str("(kicad_sch\n");
        if let Some(version) = &self.version {
            output.push_str(&format!("  (version {})\n", sexpr_atom_or_string(version)));
        }
        if let Some(generator) = &self.generator {
            output.push_str(&format!("  (generator {})\n", sexpr_string(generator)));
        }
        if let Some(uuid) = &self.uuid {
            output.push_str(&format!("  (uuid {})\n", sexpr_string(uuid)));
        }
        output.push_str(&format!(
            "  (paper {})\n",
            sexpr_string(self.paper.as_deref().unwrap_or("A4"))
        ));
        output.push_str("  (lib_symbols\n");
        for symbol in &self.library_symbols {
            symbol.write_symbol_sexpr(&mut output, 4);
        }
        output.push_str("  )\n");
        for wire in &self.wires {
            wire.write_wire_sexpr(&mut output, 2);
        }
        for label in &self.labels {
            label.write_label_sexpr(&mut output, 2);
        }
        for sheet in &self.sheets {
            sheet.write_sheet_sexpr(&mut output, 2);
        }
        for text in &self.text_items {
            text.write_text_sexpr(&mut output, 2);
        }
        for symbol in &self.symbols {
            symbol.write_instance_sexpr(&mut output, 2);
        }
        output.push_str(")\n");
        output
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
        let definition = self.symbol_definition(&symbol.lib_id);
        if symbol.sim_enabled(definition) == Some(false) {
            return None;
        }

        let reference = symbol.reference()?.trim();
        if reference.is_empty() || reference.starts_with('#') {
            return None;
        }

        let has_explicit_sim_model = symbol.has_explicit_sim_model(definition);
        let model = symbol.sim_model_value(definition);
        let params = symbol.sim_params_value(definition);
        let value = compose_spice_model_value(
            model.as_deref(),
            params.as_deref(),
            has_explicit_sim_model.then(|| symbol.value().unwrap_or_default().trim()),
        );
        let explicit_device = symbol.sim_device(definition);
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

    pub fn spice_include_directives(&self) -> Vec<String> {
        let mut includes = BTreeSet::new();
        for symbol in &self.symbols {
            let definition = self.symbol_definition(&symbol.lib_id);
            if symbol.sim_enabled(definition) == Some(false) {
                continue;
            }
            if let Some(path) = symbol
                .sim_library(definition)
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
        let definition = self.symbol_definition(&symbol.lib_id)?;
        let pins = symbol_ordered_pins(symbol, definition);

        Some(
            pins.into_iter()
                .map(|pin| {
                    pin.at
                        .map(|pin_at| transform_symbol_point(pin_at, symbol_at))
                        .and_then(|point| graph.net_at(point).map(str::to_string))
                        .unwrap_or_else(|| "unconnected".to_string())
                })
                .collect(),
        )
    }

    fn symbol_definition(&self, lib_id: &str) -> Option<&KicadSymbolDef> {
        self.library_symbols
            .iter()
            .find(|symbol| symbol.name == lib_id)
    }

    pub fn resolve_project_symbol_libraries(
        &mut self,
        project_dir: &Path,
    ) -> OslResult<Vec<KicadLibraryDiagnostic>> {
        let table_path = project_dir.join("sym-lib-table");
        if !table_path.exists() {
            return Ok(Vec::new());
        }
        self.resolve_missing_symbol_definitions_from_table(&table_path)
    }

    pub fn resolve_missing_symbol_definitions_from_table(
        &mut self,
        table_path: &Path,
    ) -> OslResult<Vec<KicadLibraryDiagnostic>> {
        let table = read_kicad_symbol_library_table(table_path)?;
        let base_dir = table_path.parent().unwrap_or_else(|| Path::new("."));
        let mut diagnostics = Vec::new();
        let mut missing = self.missing_symbol_lib_ids();

        for row in table.libraries {
            if missing.is_empty() {
                break;
            }
            if row.disabled {
                diagnostics.push(KicadLibraryDiagnostic {
                    library: row.name.clone(),
                    severity: KicadDiagnosticSeverity::Info,
                    message: "library row is disabled".to_string(),
                });
                continue;
            }
            if !row.library_type.eq_ignore_ascii_case("KiCad") {
                diagnostics.push(KicadLibraryDiagnostic {
                    library: row.name.clone(),
                    severity: KicadDiagnosticSeverity::Warning,
                    message: format!("unsupported symbol library type '{}'", row.library_type),
                });
                continue;
            }

            let resolved_path = resolve_kicad_uri(&row.uri, base_dir);
            match read_kicad_symbol_library(&resolved_path) {
                Ok(library) => {
                    let mut resolved = Vec::new();
                    for lib_id in &missing {
                        if let Some(definition) =
                            library_symbol_definition_for_lib_id(&library, &row.name, lib_id)
                        {
                            self.merge_library_symbol(definition);
                            resolved.push(lib_id.clone());
                        }
                    }
                    for lib_id in resolved {
                        missing.remove(&lib_id);
                    }
                }
                Err(error) => diagnostics.push(KicadLibraryDiagnostic {
                    library: row.name,
                    severity: KicadDiagnosticSeverity::Error,
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

    fn merge_library_symbol(&mut self, definition: KicadSymbolDef) -> bool {
        if self.symbol_definition(&definition.name).is_some() {
            return false;
        }
        self.library_symbols.push(definition);
        true
    }

    fn check_duplicate_references(&self, diagnostics: &mut Vec<KicadSchematicDiagnostic>) {
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
                diagnostics.push(kicad_schematic_diagnostic(
                    KicadDiagnosticSeverity::Error,
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
        graph: &KicadNetGraph,
        diagnostics: &mut Vec<KicadSchematicDiagnostic>,
    ) {
        for symbol in &self.symbols {
            let reference = symbol.reference().unwrap_or("<no-reference>").to_string();
            if symbol
                .reference()
                .is_none_or(|reference| reference.trim().is_empty())
            {
                diagnostics.push(kicad_schematic_diagnostic(
                    KicadDiagnosticSeverity::Error,
                    "missing-reference",
                    "symbol has no Reference property",
                    Some(symbol.lib_id.clone()),
                    None,
                    None,
                ));
            }
            if symbol.value().is_none_or(|value| value.trim().is_empty()) {
                diagnostics.push(kicad_schematic_diagnostic(
                    KicadDiagnosticSeverity::Warning,
                    "missing-value",
                    &format!("symbol '{reference}' has no Value property"),
                    Some(reference.clone()),
                    None,
                    None,
                ));
            }

            let Some(definition) = self.symbol_definition(&symbol.lib_id) else {
                diagnostics.push(kicad_schematic_diagnostic(
                    KicadDiagnosticSeverity::Error,
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
            if symbol.sim_enabled(Some(definition)) == Some(false) {
                diagnostics.push(kicad_schematic_diagnostic(
                    KicadDiagnosticSeverity::Info,
                    "simulation-disabled",
                    &format!("symbol '{reference}' is excluded from simulation"),
                    Some(reference),
                    None,
                    None,
                ));
                continue;
            }
            if let Some(device) = symbol.sim_device(Some(definition))
                && spice_primitive_for_device(&device).is_none()
            {
                diagnostics.push(kicad_schematic_diagnostic(
                    KicadDiagnosticSeverity::Error,
                    "unsupported-sim-device",
                    &format!("symbol '{reference}' uses unsupported Sim.Device '{device}'"),
                    Some(reference.clone()),
                    None,
                    None,
                ));
            }
            let Some(symbol_at) = symbol.at else {
                diagnostics.push(kicad_schematic_diagnostic(
                    KicadDiagnosticSeverity::Error,
                    "missing-symbol-position",
                    &format!("symbol '{reference}' has no placement"),
                    Some(reference),
                    None,
                    None,
                ));
                continue;
            };

            let mut definition_pins = definition.pins.iter().collect::<Vec<_>>();
            definition_pins.sort_by(compare_pin_numbers);
            if !definition_pins.is_empty() && symbol.pins.is_empty() {
                diagnostics.push(kicad_schematic_diagnostic(
                    KicadDiagnosticSeverity::Warning,
                    "missing-pin-refs",
                    &format!("symbol '{reference}' has no instance pin UUID references"),
                    Some(reference.clone()),
                    None,
                    None,
                ));
            }
            let sim_pin_order = symbol_sim_pin_order(symbol, definition);
            for pin_number in &sim_pin_order {
                if !definition
                    .pins
                    .iter()
                    .any(|pin| pin.number == *pin_number || pin.name == *pin_number)
                {
                    diagnostics.push(kicad_schematic_diagnostic(
                        KicadDiagnosticSeverity::Error,
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
                let pin_label = format!("{}:{}", reference, pin.number);
                let Some(pin_at) = pin.at else {
                    diagnostics.push(kicad_schematic_diagnostic(
                        KicadDiagnosticSeverity::Warning,
                        "missing-pin-position",
                        &format!("symbol '{reference}' pin '{}' has no position", pin.number),
                        Some(reference.clone()),
                        None,
                        Some(pin.number.clone()),
                    ));
                    continue;
                };
                let point = transform_symbol_point(pin_at, symbol_at);
                match graph.net_at(point) {
                    Some("unconnected") | None => diagnostics.push(kicad_schematic_diagnostic(
                        KicadDiagnosticSeverity::Warning,
                        "unconnected-pin",
                        &format!("symbol pin '{pin_label}' is not connected to a named net"),
                        Some(reference.clone()),
                        None,
                        Some(pin.number.clone()),
                    )),
                    Some(net) if net.starts_with('n') => {
                        diagnostics.push(kicad_schematic_diagnostic(
                            KicadDiagnosticSeverity::Info,
                            "generated-net-name",
                            &format!("symbol pin '{pin_label}' is on generated net '{net}'"),
                            Some(reference.clone()),
                            Some(net.to_string()),
                            Some(pin.number.clone()),
                        ))
                    }
                    Some(_) => {}
                }
            }
        }
    }

    fn check_wires(&self, diagnostics: &mut Vec<KicadSchematicDiagnostic>) {
        for (index, wire) in self.wires.iter().enumerate() {
            if wire.points.len() < 2 {
                diagnostics.push(kicad_schematic_diagnostic(
                    KicadDiagnosticSeverity::Error,
                    "invalid-wire",
                    &format!("wire #{index} has fewer than two points"),
                    Some(format!("wire:{index}")),
                    None,
                    None,
                ));
            }
        }
    }

    fn check_labels(&self, graph: &KicadNetGraph, diagnostics: &mut Vec<KicadSchematicDiagnostic>) {
        for label in &self.labels {
            if label.text.trim().is_empty() {
                diagnostics.push(kicad_schematic_diagnostic(
                    KicadDiagnosticSeverity::Error,
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
                diagnostics.push(kicad_schematic_diagnostic(
                    KicadDiagnosticSeverity::Warning,
                    "floating-label",
                    &format!("label '{}' is not attached to any net", label.text),
                    Some(label.text.clone()),
                    None,
                    None,
                ));
            }
        }
    }

    fn check_sheets(&self, diagnostics: &mut Vec<KicadSchematicDiagnostic>) {
        for (index, sheet) in self.sheets.iter().enumerate() {
            let item = sheet
                .sheet_name()
                .or_else(|| sheet.sheet_file())
                .map(str::to_string)
                .unwrap_or_else(|| format!("sheet:{index}"));
            if sheet.sheet_name().is_none_or(|name| name.trim().is_empty()) {
                diagnostics.push(kicad_schematic_diagnostic(
                    KicadDiagnosticSeverity::Error,
                    "missing-sheet-name",
                    &format!("hierarchical sheet #{index} has no Sheetname property"),
                    Some(item.clone()),
                    None,
                    None,
                ));
            }
            if sheet.sheet_file().is_none_or(|file| file.trim().is_empty()) {
                diagnostics.push(kicad_schematic_diagnostic(
                    KicadDiagnosticSeverity::Error,
                    "missing-sheet-file",
                    &format!("hierarchical sheet '{item}' has no Sheetfile property"),
                    Some(item.clone()),
                    None,
                    None,
                ));
            }
            if sheet.at.is_none() || sheet.size.is_none() {
                diagnostics.push(kicad_schematic_diagnostic(
                    KicadDiagnosticSeverity::Warning,
                    "missing-sheet-geometry",
                    &format!("hierarchical sheet '{item}' has incomplete placement geometry"),
                    Some(item.clone()),
                    None,
                    None,
                ));
            }
            if sheet.exclude_from_sim == Some(true) {
                diagnostics.push(kicad_schematic_diagnostic(
                    KicadDiagnosticSeverity::Info,
                    "simulation-disabled-sheet",
                    &format!("hierarchical sheet '{item}' is excluded from simulation"),
                    Some(item),
                    None,
                    None,
                ));
            } else {
                diagnostics.push(kicad_schematic_diagnostic(
                    KicadDiagnosticSeverity::Error,
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

    fn check_spice_directives(&self, diagnostics: &mut Vec<KicadSchematicDiagnostic>) {
        let directives = self.spice_directives();
        if directives.is_empty() {
            diagnostics.push(kicad_schematic_diagnostic(
                KicadDiagnosticSeverity::Warning,
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
            .any(|directive| is_spice_analysis_directive(&directive.text))
        {
            diagnostics.push(kicad_schematic_diagnostic(
                KicadDiagnosticSeverity::Warning,
                "missing-analysis-directive",
                "schematic has SPICE text but no analysis directive (.tran, .ac, .dc, .op)",
                None,
                None,
                None,
            ));
        }
    }

    fn symbol_index_by_reference(&self, reference: &str) -> OslResult<usize> {
        if reference.trim().is_empty() {
            return Err(OslError::InvalidInput(
                "KiCad symbol reference must not be empty".to_string(),
            ));
        }
        self.symbols
            .iter()
            .position(|symbol| symbol.reference() == Some(reference))
            .ok_or_else(|| {
                OslError::InvalidInput(format!(
                    "KiCad symbol reference '{reference}' was not found"
                ))
            })
    }

    fn edit_uuid(&self, uuid: Option<String>, namespace: &str, payload: &str) -> OslResult<String> {
        self.edit_uuid_excluding(uuid, namespace, payload, &BTreeSet::new())
    }

    fn edit_uuid_excluding(
        &self,
        uuid: Option<String>,
        namespace: &str,
        payload: &str,
        reserved: &BTreeSet<String>,
    ) -> OslResult<String> {
        let used = self.used_uuids();
        if let Some(uuid) = uuid.filter(|uuid| !uuid.trim().is_empty()) {
            if used.contains(&uuid) || reserved.contains(&uuid) {
                return Err(OslError::InvalidInput(format!(
                    "KiCad UUID '{uuid}' is already used in this schematic"
                )));
            }
            return Ok(uuid);
        }

        for counter in 0.. {
            let seed = format!(
                "{}:{namespace}:{payload}:{}:{}:{}:{counter}",
                self.source,
                self.symbols.len(),
                self.wires.len(),
                self.labels.len()
            );
            let candidate = uuid_from_hashes(fnv1a64(&seed), fnv1a64(&format!("{seed}:b")));
            if !used.contains(&candidate) && !reserved.contains(&candidate) {
                return Ok(candidate);
            }
        }
        unreachable!("unbounded UUID search should always find a free candidate")
    }

    fn used_uuids(&self) -> BTreeSet<String> {
        let mut uuids = BTreeSet::new();
        if let Some(uuid) = &self.uuid {
            uuids.insert(uuid.clone());
        }
        for wire in &self.wires {
            if let Some(uuid) = &wire.uuid {
                uuids.insert(uuid.clone());
            }
        }
        for label in &self.labels {
            if let Some(uuid) = &label.uuid {
                uuids.insert(uuid.clone());
            }
        }
        for sheet in &self.sheets {
            if let Some(uuid) = &sheet.uuid {
                uuids.insert(uuid.clone());
            }
            for pin in &sheet.pins {
                if let Some(uuid) = &pin.uuid {
                    uuids.insert(uuid.clone());
                }
            }
        }
        for text in &self.text_items {
            if let Some(uuid) = &text.uuid {
                uuids.insert(uuid.clone());
            }
        }
        for symbol in &self.symbols {
            if let Some(uuid) = &symbol.uuid {
                uuids.insert(uuid.clone());
            }
            for pin in &symbol.pins {
                if let Some(uuid) = &pin.uuid {
                    uuids.insert(uuid.clone());
                }
            }
        }
        uuids
    }

    fn symbol_pin_points(&self) -> Vec<KicadPoint> {
        self.symbols
            .iter()
            .flat_map(|symbol| {
                let Some(symbol_at) = symbol.at else {
                    return Vec::new();
                };
                self.symbol_definition(&symbol.lib_id)
                    .map(|definition| {
                        definition
                            .pins
                            .iter()
                            .filter_map(|pin| pin.at)
                            .map(|pin_at| transform_symbol_point(pin_at, symbol_at))
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default()
            })
            .collect()
    }

    fn sheet_pin_points(&self) -> Vec<KicadPoint> {
        self.sheets
            .iter()
            .flat_map(|sheet| {
                sheet
                    .pins
                    .iter()
                    .filter_map(|pin| pin.at.map(|at| at.point()))
            })
            .collect()
    }

    pub fn to_summary_json(&self) -> String {
        format!(
            concat!(
                "{{\n",
                "  \"source\": \"{}\",\n",
                "  \"version\": {},\n",
                "  \"generator\": {},\n",
                "  \"symbol_count\": {},\n",
                "  \"library_symbol_count\": {},\n",
                "  \"wire_count\": {},\n",
                "  \"label_count\": {},\n",
                "  \"sheet_count\": {},\n",
                "  \"sheet_pin_count\": {},\n",
                "  \"text_count\": {},\n",
                "  \"spice_directive_count\": {},\n",
                "  \"library_graphic_count\": {}\n",
                "}}"
            ),
            json_escape(&self.source),
            json_option(self.version.as_deref()),
            json_option(self.generator.as_deref()),
            self.symbols.len(),
            self.library_symbols.len(),
            self.wires.len(),
            self.labels.len(),
            self.sheets.len(),
            self.sheets
                .iter()
                .map(|sheet| sheet.pins.len())
                .sum::<usize>(),
            self.text_items.len(),
            self.spice_directives().len(),
            self.library_symbols
                .iter()
                .map(|symbol| symbol.graphics.len())
                .sum::<usize>()
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
            match schematic.symbol_to_spice_line_with_nodes(&scoped_symbol, &mapped_nodes) {
                Some(line) => self.components.push(line),
                None if scoped_symbol.sim_enabled(schematic.symbol_definition(&symbol.lib_id))
                    == Some(false) => {}
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

fn is_spice_analysis_directive(text: &str) -> bool {
    let text = text.trim_start().to_ascii_lowercase();
    text.starts_with(".tran")
        || text.starts_with(".ac")
        || text.starts_with(".dc")
        || text.starts_with(".op")
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

#[derive(Debug, Clone, PartialEq)]
pub struct KicadCanvasScene {
    pub source: String,
    pub symbols: Vec<KicadCanvasSymbol>,
    pub sheets: Vec<KicadCanvasSheet>,
    pub wires: Vec<KicadCanvasWire>,
    pub labels: Vec<KicadCanvasLabel>,
    pub junctions: Vec<KicadCanvasJunction>,
    pub bounds: Option<KicadBoundingBox>,
}

impl KicadCanvasScene {
    pub fn from_schematic(schematic: &KicadSchematic) -> Self {
        let mut bounds = KicadBoundingBoxBuilder::default();

        let symbols = schematic
            .symbols
            .iter()
            .filter_map(|symbol| {
                let definition = schematic.symbol_definition(&symbol.lib_id)?;
                let at = symbol.at.unwrap_or(KicadAt {
                    x: 0.0,
                    y: 0.0,
                    rotation: 0.0,
                });
                let graphics = definition
                    .graphics
                    .iter()
                    .map(|graphic| graphic.transformed(at))
                    .collect::<Vec<_>>();
                let pins = definition
                    .pins
                    .iter()
                    .filter_map(|pin| KicadCanvasPin::from_pin_def(pin, at))
                    .collect::<Vec<_>>();
                let symbol_bounds = canvas_symbol_bounds(&graphics, &pins);
                if let Some(symbol_bounds) = symbol_bounds {
                    bounds.include_box(symbol_bounds);
                }

                Some(KicadCanvasSymbol {
                    lib_id: symbol.lib_id.clone(),
                    reference: symbol.reference().unwrap_or_default().to_string(),
                    value: symbol.value().unwrap_or_default().to_string(),
                    at,
                    graphics,
                    pins,
                    bounds: symbol_bounds,
                })
            })
            .collect::<Vec<_>>();

        let sheets = schematic
            .sheets
            .iter()
            .map(|sheet| {
                let mut sheet_bounds = KicadBoundingBoxBuilder::default();
                if let Some(sheet_box) = sheet.bounding_box() {
                    sheet_bounds.include_box(sheet_box);
                    bounds.include_box(sheet_box);
                }
                let pins = sheet
                    .pins
                    .iter()
                    .map(|pin| {
                        if let Some(at) = pin.at {
                            sheet_bounds.include(at.point());
                            bounds.include(at.point());
                        }
                        KicadCanvasSheetPin {
                            name: pin.name.clone(),
                            pin_type: pin.pin_type.clone(),
                            at: pin.at,
                        }
                    })
                    .collect();
                KicadCanvasSheet {
                    name: sheet.sheet_name().unwrap_or_default().to_string(),
                    file: sheet.sheet_file().unwrap_or_default().to_string(),
                    at: sheet.at,
                    size: sheet.size,
                    pins,
                    bounds: sheet_bounds.finish(),
                }
            })
            .collect::<Vec<_>>();

        let wires = schematic
            .wires
            .iter()
            .map(|wire| {
                for point in &wire.points {
                    bounds.include(*point);
                }
                KicadCanvasWire {
                    points: wire.points.clone(),
                }
            })
            .collect::<Vec<_>>();

        let labels = schematic
            .labels
            .iter()
            .map(|label| {
                if let Some(at) = label.at {
                    bounds.include(at.point());
                }
                KicadCanvasLabel {
                    text: label.text.clone(),
                    kind: label.kind,
                    at: label.at,
                }
            })
            .collect::<Vec<_>>();

        let junctions = schematic
            .junctions
            .iter()
            .map(|junction| {
                bounds.include(junction.at);
                KicadCanvasJunction { at: junction.at }
            })
            .collect::<Vec<_>>();

        Self {
            source: schematic.source.clone(),
            symbols,
            sheets,
            wires,
            labels,
            junctions,
            bounds: bounds.finish(),
        }
    }

    pub fn to_summary_json(&self) -> String {
        let bounds = self
            .bounds
            .map(kicad_bounding_box_json)
            .unwrap_or_else(|| "null".to_string());
        let graphic_count = self
            .symbols
            .iter()
            .map(|symbol| symbol.graphics.len())
            .sum::<usize>();
        let pin_count = self
            .symbols
            .iter()
            .map(|symbol| symbol.pins.len())
            .sum::<usize>();

        format!(
            concat!(
                "{{\n",
                "  \"source\": \"{}\",\n",
                "  \"symbol_count\": {},\n",
                "  \"sheet_count\": {},\n",
                "  \"graphic_count\": {},\n",
                "  \"pin_count\": {},\n",
                "  \"sheet_pin_count\": {},\n",
                "  \"wire_count\": {},\n",
                "  \"label_count\": {},\n",
                "  \"junction_count\": {},\n",
                "  \"bounds\": {}\n",
                "}}"
            ),
            json_escape(&self.source),
            self.symbols.len(),
            self.sheets.len(),
            graphic_count,
            pin_count,
            self.sheets
                .iter()
                .map(|sheet| sheet.pins.len())
                .sum::<usize>(),
            self.wires.len(),
            self.labels.len(),
            self.junctions.len(),
            bounds
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadCanvasSymbol {
    pub lib_id: String,
    pub reference: String,
    pub value: String,
    pub at: KicadAt,
    pub graphics: Vec<KicadCanvasGraphic>,
    pub pins: Vec<KicadCanvasPin>,
    pub bounds: Option<KicadBoundingBox>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadCanvasSheet {
    pub name: String,
    pub file: String,
    pub at: Option<KicadAt>,
    pub size: Option<KicadSize>,
    pub pins: Vec<KicadCanvasSheetPin>,
    pub bounds: Option<KicadBoundingBox>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadCanvasSheetPin {
    pub name: String,
    pub pin_type: String,
    pub at: Option<KicadAt>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum KicadCanvasGraphic {
    Polyline {
        points: Vec<KicadPoint>,
    },
    Rectangle {
        start: KicadPoint,
        end: KicadPoint,
    },
    Circle {
        center: KicadPoint,
        radius: f64,
    },
    Arc {
        start: KicadPoint,
        mid: Option<KicadPoint>,
        end: KicadPoint,
    },
    Text {
        text: String,
        at: Option<KicadAt>,
    },
}

impl KicadCanvasGraphic {
    fn include_in_bounds(&self, bounds: &mut KicadBoundingBoxBuilder) {
        match self {
            Self::Polyline { points } => {
                for point in points {
                    bounds.include(*point);
                }
            }
            Self::Rectangle { start, end } => {
                bounds.include(*start);
                bounds.include(*end);
            }
            Self::Circle { center, radius } => {
                bounds.include(KicadPoint {
                    x: center.x - radius,
                    y: center.y - radius,
                });
                bounds.include(KicadPoint {
                    x: center.x + radius,
                    y: center.y + radius,
                });
            }
            Self::Arc { start, mid, end } => {
                bounds.include(*start);
                if let Some(mid) = mid {
                    bounds.include(*mid);
                }
                bounds.include(*end);
            }
            Self::Text { at, .. } => {
                if let Some(at) = at {
                    bounds.include(at.point());
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadCanvasPin {
    pub number: String,
    pub name: String,
    pub electrical_type: String,
    pub start: KicadPoint,
    pub end: KicadPoint,
}

impl KicadCanvasPin {
    fn from_pin_def(pin: &KicadPinDef, symbol_at: KicadAt) -> Option<Self> {
        let pin_at = pin.at?;
        let local_start = pin_at.point();
        let local_end = pin_body_end(pin_at, pin.length.unwrap_or(0.0));

        Some(Self {
            number: pin.number.clone(),
            name: pin.name.clone(),
            electrical_type: pin.electrical_type.clone(),
            start: transform_local_point(local_start, symbol_at),
            end: transform_local_point(local_end, symbol_at),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadCanvasWire {
    pub points: Vec<KicadPoint>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadCanvasLabel {
    pub text: String,
    pub kind: KicadLabelKind,
    pub at: Option<KicadAt>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadCanvasJunction {
    pub at: KicadPoint,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadNetGraph {
    pub nets: Vec<KicadNet>,
    nets_by_point: BTreeMap<PointKey, String>,
}

impl KicadNetGraph {
    fn build(schematic: &KicadSchematic) -> Self {
        let mut points = BTreeMap::<PointKey, KicadPoint>::new();
        for wire in &schematic.wires {
            for point in &wire.points {
                insert_point(&mut points, *point);
            }
        }
        for label in &schematic.labels {
            if let Some(at) = label.at {
                insert_point(&mut points, at.point());
            }
        }
        for junction in &schematic.junctions {
            insert_point(&mut points, junction.at);
        }
        for point in schematic.symbol_pin_points() {
            insert_point(&mut points, point);
        }
        for point in schematic.sheet_pin_points() {
            insert_point(&mut points, point);
        }

        let ordered_keys = points.keys().copied().collect::<Vec<_>>();
        let indexes = ordered_keys
            .iter()
            .enumerate()
            .map(|(index, key)| (*key, index))
            .collect::<BTreeMap<_, _>>();
        let mut graph = DisjointSet::new(ordered_keys.len());

        for wire in &schematic.wires {
            for segment in wire.points.windows(2) {
                let mut segment_indexes = ordered_keys
                    .iter()
                    .filter(|key| {
                        points.get(key).is_some_and(|point| {
                            segment_contains_point(segment[0], segment[1], *point)
                        })
                    })
                    .filter_map(|key| indexes.get(key).copied())
                    .collect::<Vec<_>>();
                segment_indexes.sort_unstable();
                if let Some(first) = segment_indexes.first().copied() {
                    for index in segment_indexes.into_iter().skip(1) {
                        graph.union(first, index);
                    }
                }
            }
        }

        let mut labels_by_name = BTreeMap::<String, Vec<usize>>::new();
        for label in &schematic.labels {
            if let Some(at) = label.at
                && let Some(index) = indexes.get(&PointKey::from(at.point())).copied()
            {
                labels_by_name
                    .entry(normalize_net_name(&label.text))
                    .or_default()
                    .push(index);
            }
        }
        for label_indexes in labels_by_name.values() {
            if let Some(first) = label_indexes.first().copied() {
                for index in label_indexes.iter().copied().skip(1) {
                    graph.union(first, index);
                }
            }
        }

        let mut labels_by_root = BTreeMap::<usize, BTreeSet<String>>::new();
        for label in &schematic.labels {
            if let Some(at) = label.at
                && let Some(index) = indexes.get(&PointKey::from(at.point())).copied()
            {
                labels_by_root
                    .entry(graph.find(index))
                    .or_default()
                    .insert(normalize_net_name(&label.text));
            }
        }

        let mut names_by_root = BTreeMap::<usize, String>::new();
        let mut generated_index = 1;
        for index in 0..ordered_keys.len() {
            let root = graph.find(index);
            names_by_root.entry(root).or_insert_with(|| {
                preferred_net_label(labels_by_root.get(&root)).unwrap_or_else(|| {
                    let name = format!("n{generated_index:03}");
                    generated_index += 1;
                    name
                })
            });
        }

        let mut nets_by_point = BTreeMap::new();
        let mut points_by_net = BTreeMap::<String, Vec<KicadPoint>>::new();
        for (index, key) in ordered_keys.iter().enumerate() {
            let root = graph.find(index);
            let name = names_by_root
                .get(&root)
                .cloned()
                .unwrap_or_else(|| "n000".to_string());
            nets_by_point.insert(*key, name.clone());
            if let Some(point) = points.get(key).copied() {
                points_by_net.entry(name).or_default().push(point);
            }
        }

        let nets = points_by_net
            .into_iter()
            .map(|(name, points)| KicadNet { name, points })
            .collect();

        Self {
            nets,
            nets_by_point,
        }
    }

    pub fn net_at(&self, point: KicadPoint) -> Option<&str> {
        self.nets_by_point
            .get(&PointKey::from(point))
            .map(String::as_str)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadNet {
    pub name: String,
    pub points: Vec<KicadPoint>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadSymbolLibrary {
    pub source: String,
    pub version: Option<String>,
    pub generator: Option<String>,
    pub symbols: Vec<KicadSymbolDef>,
}

impl KicadSymbolLibrary {
    pub fn symbol(&self, name: &str) -> Option<&KicadSymbolDef> {
        self.symbols.iter().find(|symbol| symbol.name == name)
    }

    pub fn to_kicad_symbol_library_sexpr(&self) -> String {
        let mut output = String::new();
        output.push_str("(kicad_symbol_lib\n");
        if let Some(version) = &self.version {
            output.push_str(&format!("  (version {})\n", sexpr_atom_or_string(version)));
        }
        if let Some(generator) = &self.generator {
            output.push_str(&format!("  (generator {})\n", sexpr_string(generator)));
        }
        for symbol in &self.symbols {
            symbol.write_symbol_sexpr(&mut output, 2);
        }
        output.push_str(")\n");
        output
    }

    pub fn to_summary_json(&self) -> String {
        format!(
            concat!(
                "{{\n",
                "  \"source\": \"{}\",\n",
                "  \"version\": {},\n",
                "  \"generator\": {},\n",
                "  \"symbol_count\": {},\n",
                "  \"graphic_count\": {}\n",
                "}}"
            ),
            json_escape(&self.source),
            json_option(self.version.as_deref()),
            json_option(self.generator.as_deref()),
            self.symbols.len(),
            self.symbols
                .iter()
                .map(|symbol| symbol.graphics.len())
                .sum::<usize>()
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadSymbolLibraryTable {
    pub source: String,
    pub version: Option<String>,
    pub libraries: Vec<KicadSymbolLibraryTableRow>,
}

impl KicadSymbolLibraryTable {
    pub fn enabled_kicad_libraries(&self) -> impl Iterator<Item = &KicadSymbolLibraryTableRow> {
        self.libraries
            .iter()
            .filter(|row| !row.disabled && row.library_type.eq_ignore_ascii_case("KiCad"))
    }

    pub fn to_summary_json(&self) -> String {
        format!(
            concat!(
                "{{\n",
                "  \"source\": \"{}\",\n",
                "  \"version\": {},\n",
                "  \"library_count\": {},\n",
                "  \"enabled_kicad_library_count\": {},\n",
                "  \"disabled_library_count\": {},\n",
                "  \"hidden_library_count\": {}\n",
                "}}"
            ),
            json_escape(&self.source),
            json_option(self.version.as_deref()),
            self.libraries.len(),
            self.enabled_kicad_libraries().count(),
            self.libraries.iter().filter(|row| row.disabled).count(),
            self.libraries.iter().filter(|row| row.hidden).count(),
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadSymbolLibraryTableRow {
    pub name: String,
    pub library_type: String,
    pub uri: String,
    pub options: Option<String>,
    pub description: Option<String>,
    pub hidden: bool,
    pub disabled: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadSymbolLibraryIndex {
    pub source: String,
    pub libraries: Vec<KicadIndexedLibrary>,
    pub symbols: Vec<KicadIndexedSymbol>,
    pub diagnostics: Vec<KicadLibraryDiagnostic>,
}

impl KicadSymbolLibraryIndex {
    pub fn from_table(table: KicadSymbolLibraryTable, base_dir: &Path) -> Self {
        let mut libraries = Vec::new();
        let mut symbols = Vec::new();
        let mut diagnostics = Vec::new();

        for row in table.libraries {
            if row.disabled {
                diagnostics.push(KicadLibraryDiagnostic {
                    library: row.name.clone(),
                    severity: KicadDiagnosticSeverity::Info,
                    message: "library row is disabled".to_string(),
                });
                continue;
            }
            if !row.library_type.eq_ignore_ascii_case("KiCad") {
                diagnostics.push(KicadLibraryDiagnostic {
                    library: row.name.clone(),
                    severity: KicadDiagnosticSeverity::Warning,
                    message: format!("unsupported symbol library type '{}'", row.library_type),
                });
                continue;
            }

            let resolved_path = resolve_kicad_uri(&row.uri, base_dir);
            match read_kicad_symbol_library(&resolved_path) {
                Ok(library) => {
                    let symbol_count = library.symbols.len();
                    for symbol in &library.symbols {
                        symbols.push(KicadIndexedSymbol {
                            id: format!("{}:{}", row.name, symbol.local_name()),
                            library: row.name.clone(),
                            name: symbol.local_name().to_string(),
                            source: resolved_path.display().to_string(),
                            pin_count: symbol.pins.len(),
                            graphic_count: symbol.graphics.len(),
                            bounding_box: symbol.bounding_box(),
                        });
                    }
                    libraries.push(KicadIndexedLibrary {
                        name: row.name,
                        source: resolved_path.display().to_string(),
                        symbol_count,
                    });
                }
                Err(error) => {
                    diagnostics.push(KicadLibraryDiagnostic {
                        library: row.name,
                        severity: KicadDiagnosticSeverity::Error,
                        message: format!("failed to load {}: {}", resolved_path.display(), error),
                    });
                }
            }
        }

        Self {
            source: table.source,
            libraries,
            symbols,
            diagnostics,
        }
    }

    pub fn symbol(&self, lib_id: &str) -> Option<&KicadIndexedSymbol> {
        self.symbols.iter().find(|symbol| symbol.id == lib_id)
    }

    pub fn to_summary_json(&self) -> String {
        let diagnostics = self
            .diagnostics
            .iter()
            .map(|diagnostic| {
                format!(
                    "    {{ \"severity\": \"{}\", \"library\": \"{}\", \"message\": \"{}\" }}",
                    diagnostic.severity.as_str(),
                    json_escape(&diagnostic.library),
                    json_escape(&diagnostic.message)
                )
            })
            .collect::<Vec<_>>()
            .join(",\n");

        format!(
            concat!(
                "{{\n",
                "  \"source\": \"{}\",\n",
                "  \"library_count\": {},\n",
                "  \"symbol_count\": {},\n",
                "  \"diagnostic_count\": {},\n",
                "  \"diagnostics\": [\n",
                "{}\n",
                "  ]\n",
                "}}"
            ),
            json_escape(&self.source),
            self.libraries.len(),
            self.symbols.len(),
            self.diagnostics.len(),
            diagnostics
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadIndexedLibrary {
    pub name: String,
    pub source: String,
    pub symbol_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadIndexedSymbol {
    pub id: String,
    pub library: String,
    pub name: String,
    pub source: String,
    pub pin_count: usize,
    pub graphic_count: usize,
    pub bounding_box: Option<KicadBoundingBox>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadLibraryDiagnostic {
    pub library: String,
    pub severity: KicadDiagnosticSeverity,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KicadDiagnosticSeverity {
    Info,
    Warning,
    Error,
}

impl KicadDiagnosticSeverity {
    fn as_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadSymbolInstance {
    pub lib_id: String,
    pub at: Option<KicadAt>,
    pub unit: Option<u32>,
    pub uuid: Option<String>,
    pub exclude_from_sim: Option<bool>,
    pub properties: Vec<KicadProperty>,
    pub pins: Vec<KicadSymbolPinRef>,
}

impl KicadSymbolInstance {
    pub fn property(&self, name: &str) -> Option<&str> {
        self.properties
            .iter()
            .find(|property| property.name == name)
            .map(|property| property.value.as_str())
    }

    pub fn reference(&self) -> Option<&str> {
        self.property("Reference")
    }

    pub fn value(&self) -> Option<&str> {
        self.property("Value")
    }

    fn inherited_property<'a>(
        &'a self,
        definition: Option<&'a KicadSymbolDef>,
        name: &str,
    ) -> Option<&'a str> {
        self.property(name)
            .or_else(|| definition.and_then(|definition| definition.property(name)))
    }

    fn sim_enabled(&self, definition: Option<&KicadSymbolDef>) -> Option<bool> {
        if let Some(exclude_from_sim) = self.exclude_from_sim {
            return Some(!exclude_from_sim);
        }
        if let Some(exclude_from_sim) =
            definition.and_then(|definition| definition.exclude_from_sim)
        {
            return Some(!exclude_from_sim);
        }
        self.inherited_property(definition, "Sim.Enable")
            .or_else(|| self.inherited_property(definition, "Spice_Netlist_Enabled"))
            .and_then(parse_kicad_enable_value)
    }

    fn sim_device(&self, definition: Option<&KicadSymbolDef>) -> Option<String> {
        self.inherited_property(definition, "Sim.Device")
            .or_else(|| self.inherited_property(definition, "Spice_Primitive"))
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    }

    fn sim_model_value(&self, definition: Option<&KicadSymbolDef>) -> Option<String> {
        if let Some(value) = self
            .inherited_property(definition, "Sim.Name")
            .or_else(|| self.inherited_property(definition, "Spice_Model"))
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            return Some(value.to_string());
        }
        self.inherited_property(definition, "Sim.Params")
            .and_then(|value| extract_named_sim_param(value, "model"))
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    }

    fn sim_params_value(&self, definition: Option<&KicadSymbolDef>) -> Option<String> {
        self.inherited_property(definition, "Sim.Params")
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(strip_kicad_sim_model_params)
            .filter(|value| !value.is_empty())
    }

    fn sim_library<'a>(&'a self, definition: Option<&'a KicadSymbolDef>) -> Option<&'a str> {
        self.inherited_property(definition, "Sim.Library")
            .or_else(|| self.inherited_property(definition, "Spice_Lib_File"))
    }

    fn sim_pins<'a>(&'a self, definition: Option<&'a KicadSymbolDef>) -> Option<&'a str> {
        self.inherited_property(definition, "Sim.Pins")
            .or_else(|| self.inherited_property(definition, "Spice_Node_Sequence"))
    }

    fn has_explicit_sim_model(&self, definition: Option<&KicadSymbolDef>) -> bool {
        self.inherited_property(definition, "Sim.Device").is_some()
            || self.inherited_property(definition, "Sim.Params").is_some()
            || self.inherited_property(definition, "Sim.Name").is_some()
            || self
                .inherited_property(definition, "Spice_Primitive")
                .is_some()
            || self.inherited_property(definition, "Spice_Model").is_some()
    }

    fn write_instance_sexpr(&self, output: &mut String, indent: usize) {
        let pad = " ".repeat(indent);
        output.push_str(&format!("{}(symbol\n", pad));
        output.push_str(&format!(
            "{}  (lib_id {})\n",
            pad,
            sexpr_string(&self.lib_id)
        ));
        if let Some(at) = self.at {
            output.push_str(&format!(
                "{}  (at {} {} {})\n",
                pad,
                format_number(at.x),
                format_number(at.y),
                format_number(at.rotation)
            ));
        }
        if let Some(unit) = self.unit {
            output.push_str(&format!("{}  (unit {})\n", pad, unit));
        }
        if let Some(uuid) = &self.uuid {
            output.push_str(&format!("{}  (uuid {})\n", pad, sexpr_string(uuid)));
        }
        if let Some(exclude_from_sim) = self.exclude_from_sim {
            output.push_str(&format!(
                "{}  (exclude_from_sim {})\n",
                pad,
                if exclude_from_sim { "yes" } else { "no" }
            ));
        }
        for property in &self.properties {
            property.write_property_sexpr(output, indent + 2);
        }
        for pin in &self.pins {
            pin.write_pin_ref_sexpr(output, indent + 2);
        }
        output.push_str(&format!("{})\n", pad));
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadSymbolPinRef {
    pub number: Option<String>,
    pub uuid: Option<String>,
}

impl KicadSymbolPinRef {
    fn write_pin_ref_sexpr(&self, output: &mut String, indent: usize) {
        let pad = " ".repeat(indent);
        let number = self
            .number
            .as_deref()
            .or(self.uuid.as_deref())
            .unwrap_or("?");
        output.push_str(&format!("{}(pin {}", pad, sexpr_string(number)));
        if let Some(uuid) = &self.uuid {
            output.push_str(&format!(" (uuid {})", sexpr_string(uuid)));
        }
        output.push_str(")\n");
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadSymbolDef {
    pub name: String,
    pub exclude_from_sim: Option<bool>,
    pub properties: Vec<KicadProperty>,
    pub graphics: Vec<KicadGraphic>,
    pub pins: Vec<KicadPinDef>,
}

impl KicadSymbolDef {
    pub fn property(&self, name: &str) -> Option<&str> {
        self.properties
            .iter()
            .find(|property| property.name == name)
            .map(|property| property.value.as_str())
    }

    pub fn bounding_box(&self) -> Option<KicadBoundingBox> {
        let mut bounds = KicadBoundingBoxBuilder::default();
        for graphic in &self.graphics {
            graphic.include_in_bounds(&mut bounds);
        }
        for pin in &self.pins {
            if let Some(at) = pin.at {
                bounds.include(at.point());
                if let Some(length) = pin.length {
                    bounds.include(pin_body_end(at, length));
                }
            }
        }
        bounds.finish()
    }

    pub fn local_name(&self) -> &str {
        self.name
            .rsplit_once(':')
            .map(|(_, local_name)| local_name)
            .unwrap_or(&self.name)
    }

    fn write_symbol_sexpr(&self, output: &mut String, indent: usize) {
        let pad = " ".repeat(indent);
        output.push_str(&format!("{}(symbol {}\n", pad, sexpr_string(&self.name)));
        if let Some(exclude_from_sim) = self.exclude_from_sim {
            output.push_str(&format!(
                "{}  (exclude_from_sim {})\n",
                pad,
                if exclude_from_sim { "yes" } else { "no" }
            ));
        }
        for property in &self.properties {
            property.write_property_sexpr(output, indent + 2);
        }
        output.push_str(&format!(
            "{}  (symbol {}\n",
            pad,
            sexpr_string(&format!("{}_0_1", self.local_name()))
        ));
        for graphic in &self.graphics {
            graphic.write_graphic_sexpr(output, indent + 4);
        }
        for pin in &self.pins {
            pin.write_pin_sexpr(output, indent + 4);
        }
        output.push_str(&format!("{}  )\n", pad));
        output.push_str(&format!("{})\n", pad));
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum KicadGraphic {
    Polyline {
        points: Vec<KicadPoint>,
    },
    Rectangle {
        start: KicadPoint,
        end: KicadPoint,
    },
    Circle {
        center: KicadPoint,
        radius: f64,
    },
    Arc {
        start: KicadPoint,
        mid: Option<KicadPoint>,
        end: KicadPoint,
    },
    Text {
        text: String,
        at: Option<KicadAt>,
    },
}

impl KicadGraphic {
    fn include_in_bounds(&self, bounds: &mut KicadBoundingBoxBuilder) {
        match self {
            Self::Polyline { points } => {
                for point in points {
                    bounds.include(*point);
                }
            }
            Self::Rectangle { start, end } => {
                bounds.include(*start);
                bounds.include(*end);
            }
            Self::Circle { center, radius } => {
                bounds.include(KicadPoint {
                    x: center.x - radius,
                    y: center.y - radius,
                });
                bounds.include(KicadPoint {
                    x: center.x + radius,
                    y: center.y + radius,
                });
            }
            Self::Arc { start, mid, end } => {
                bounds.include(*start);
                if let Some(mid) = mid {
                    bounds.include(*mid);
                }
                bounds.include(*end);
            }
            Self::Text { at, .. } => {
                if let Some(at) = at {
                    bounds.include(at.point());
                }
            }
        }
    }

    fn transformed(&self, symbol_at: KicadAt) -> KicadCanvasGraphic {
        match self {
            Self::Polyline { points } => KicadCanvasGraphic::Polyline {
                points: points
                    .iter()
                    .map(|point| transform_local_point(*point, symbol_at))
                    .collect(),
            },
            Self::Rectangle { start, end } => KicadCanvasGraphic::Rectangle {
                start: transform_local_point(*start, symbol_at),
                end: transform_local_point(*end, symbol_at),
            },
            Self::Circle { center, radius } => KicadCanvasGraphic::Circle {
                center: transform_local_point(*center, symbol_at),
                radius: *radius,
            },
            Self::Arc { start, mid, end } => KicadCanvasGraphic::Arc {
                start: transform_local_point(*start, symbol_at),
                mid: mid.map(|point| transform_local_point(point, symbol_at)),
                end: transform_local_point(*end, symbol_at),
            },
            Self::Text { text, at } => KicadCanvasGraphic::Text {
                text: text.clone(),
                at: at.map(|at| transform_local_at(at, symbol_at)),
            },
        }
    }

    fn write_graphic_sexpr(&self, output: &mut String, indent: usize) {
        let pad = " ".repeat(indent);
        match self {
            Self::Polyline { points } => {
                let points = points
                    .iter()
                    .map(|point| {
                        format!("(xy {} {})", format_number(point.x), format_number(point.y))
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                output.push_str(&format!(
                    "{}(polyline (pts {}) (stroke (width 0.254) (type default)) (fill (type none)))\n",
                    pad, points
                ));
            }
            Self::Rectangle { start, end } => {
                output.push_str(&format!(
                    "{}(rectangle (start {} {}) (end {} {}) (stroke (width 0.254) (type default)) (fill (type none)))\n",
                    pad,
                    format_number(start.x),
                    format_number(start.y),
                    format_number(end.x),
                    format_number(end.y)
                ));
            }
            Self::Circle { center, radius } => {
                output.push_str(&format!(
                    "{}(circle (center {} {}) (radius {}) (stroke (width 0.254) (type default)) (fill (type none)))\n",
                    pad,
                    format_number(center.x),
                    format_number(center.y),
                    format_number(*radius)
                ));
            }
            Self::Arc { start, mid, end } => {
                let mid = mid.unwrap_or(KicadPoint {
                    x: (start.x + end.x) / 2.0,
                    y: (start.y + end.y) / 2.0,
                });
                output.push_str(&format!(
                    "{}(arc (start {} {}) (mid {} {}) (end {} {}) (stroke (width 0.254) (type default)) (fill (type none)))\n",
                    pad,
                    format_number(start.x),
                    format_number(start.y),
                    format_number(mid.x),
                    format_number(mid.y),
                    format_number(end.x),
                    format_number(end.y)
                ));
            }
            Self::Text { text, at } => {
                output.push_str(&format!("{}(text {}", pad, sexpr_string(text)));
                if let Some(at) = at {
                    output.push_str(&format!(
                        " (at {} {} {})",
                        format_number(at.x),
                        format_number(at.y),
                        format_number(at.rotation)
                    ));
                }
                output.push_str(" (effects (font (size 1.27 1.27))))\n");
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KicadBoundingBox {
    pub min: KicadPoint,
    pub max: KicadPoint,
}

impl KicadBoundingBox {
    pub fn width(self) -> f64 {
        self.max.x - self.min.x
    }

    pub fn height(self) -> f64 {
        self.max.y - self.min.y
    }
}

#[derive(Debug, Default)]
struct KicadBoundingBoxBuilder {
    min: Option<KicadPoint>,
    max: Option<KicadPoint>,
}

impl KicadBoundingBoxBuilder {
    fn include(&mut self, point: KicadPoint) {
        self.min = Some(match self.min {
            Some(min) => KicadPoint {
                x: min.x.min(point.x),
                y: min.y.min(point.y),
            },
            None => point,
        });
        self.max = Some(match self.max {
            Some(max) => KicadPoint {
                x: max.x.max(point.x),
                y: max.y.max(point.y),
            },
            None => point,
        });
    }

    fn include_box(&mut self, bounds: KicadBoundingBox) {
        self.include(bounds.min);
        self.include(bounds.max);
    }

    fn finish(self) -> Option<KicadBoundingBox> {
        Some(KicadBoundingBox {
            min: self.min?,
            max: self.max?,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadPinDef {
    pub number: String,
    pub name: String,
    pub electrical_type: String,
    pub shape: String,
    pub at: Option<KicadAt>,
    pub length: Option<f64>,
}

impl KicadPinDef {
    fn write_pin_sexpr(&self, output: &mut String, indent: usize) {
        let pad = " ".repeat(indent);
        output.push_str(&format!(
            "{}(pin {} {}",
            pad,
            sexpr_atom_or_string(&self.electrical_type),
            sexpr_atom_or_string(&self.shape)
        ));
        if let Some(at) = self.at {
            output.push_str(&format!(
                " (at {} {} {})",
                format_number(at.x),
                format_number(at.y),
                format_number(at.rotation)
            ));
        }
        if let Some(length) = self.length {
            output.push_str(&format!(" (length {})", format_number(length)));
        }
        output.push_str(&format!(
            " (name {} (effects (font (size 1.27 1.27))))",
            sexpr_string(&self.name)
        ));
        output.push_str(&format!(
            " (number {} (effects (font (size 1.27 1.27))))",
            sexpr_string(&self.number)
        ));
        output.push_str(")\n");
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadProperty {
    pub name: String,
    pub value: String,
    pub at: Option<KicadAt>,
}

impl KicadProperty {
    fn write_property_sexpr(&self, output: &mut String, indent: usize) {
        let pad = " ".repeat(indent);
        output.push_str(&format!(
            "{}(property {} {}",
            pad,
            sexpr_string(&self.name),
            sexpr_string(&self.value)
        ));
        if let Some(at) = self.at {
            output.push_str(&format!(
                " (at {} {} {})",
                format_number(at.x),
                format_number(at.y),
                format_number(at.rotation)
            ));
        }
        output.push_str(" (effects (font (size 1.27 1.27))))\n");
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadWire {
    pub points: Vec<KicadPoint>,
    pub uuid: Option<String>,
}

impl KicadWire {
    fn write_wire_sexpr(&self, output: &mut String, indent: usize) {
        let pad = " ".repeat(indent);
        output.push_str(&format!("{}(wire", pad));
        write_points_sexpr(output, &self.points);
        output.push_str(" (stroke (width 0) (type default))");
        if let Some(uuid) = &self.uuid {
            output.push_str(&format!(" (uuid {})", sexpr_string(uuid)));
        }
        output.push_str(")\n");
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadLabel {
    pub text: String,
    pub kind: KicadLabelKind,
    pub at: Option<KicadAt>,
    pub uuid: Option<String>,
}

impl KicadLabel {
    fn write_label_sexpr(&self, output: &mut String, indent: usize) {
        let pad = " ".repeat(indent);
        output.push_str(&format!(
            "{}({} {}",
            pad,
            self.kind.sexpr_name(),
            sexpr_string(&self.text)
        ));
        if let Some(at) = self.at {
            output.push_str(&format!(
                " (at {} {} {})",
                format_number(at.x),
                format_number(at.y),
                format_number(at.rotation)
            ));
        }
        output.push_str(" (effects (font (size 1.27 1.27)))");
        if let Some(uuid) = &self.uuid {
            output.push_str(&format!(" (uuid {})", sexpr_string(uuid)));
        }
        output.push_str(")\n");
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KicadLabelKind {
    Local,
    Global,
    Hierarchical,
}

impl KicadLabelKind {
    fn sexpr_name(self) -> &'static str {
        match self {
            Self::Local => "label",
            Self::Global => "global_label",
            Self::Hierarchical => "hierarchical_label",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadSheet {
    pub at: Option<KicadAt>,
    pub size: Option<KicadSize>,
    pub uuid: Option<String>,
    pub exclude_from_sim: Option<bool>,
    pub properties: Vec<KicadProperty>,
    pub pins: Vec<KicadSheetPin>,
}

impl KicadSheet {
    pub fn property(&self, name: &str) -> Option<&str> {
        self.properties
            .iter()
            .find(|property| property.name == name)
            .map(|property| property.value.as_str())
    }

    pub fn sheet_name(&self) -> Option<&str> {
        self.property("Sheetname")
    }

    pub fn sheet_file(&self) -> Option<&str> {
        self.property("Sheetfile")
    }

    pub fn bounding_box(&self) -> Option<KicadBoundingBox> {
        let at = self.at?;
        let size = self.size?;
        Some(KicadBoundingBox {
            min: at.point(),
            max: KicadPoint {
                x: at.x + size.width,
                y: at.y + size.height,
            },
        })
    }

    fn write_sheet_sexpr(&self, output: &mut String, indent: usize) {
        let pad = " ".repeat(indent);
        output.push_str(&format!("{}(sheet\n", pad));
        if let Some(at) = self.at {
            output.push_str(&format!(
                "{}  (at {} {})\n",
                pad,
                format_number(at.x),
                format_number(at.y)
            ));
        }
        if let Some(size) = self.size {
            output.push_str(&format!(
                "{}  (size {} {})\n",
                pad,
                format_number(size.width),
                format_number(size.height)
            ));
        }
        if let Some(exclude_from_sim) = self.exclude_from_sim {
            output.push_str(&format!(
                "{}  (exclude_from_sim {})\n",
                pad,
                if exclude_from_sim { "yes" } else { "no" }
            ));
        }
        if let Some(uuid) = &self.uuid {
            output.push_str(&format!("{}  (uuid {})\n", pad, sexpr_string(uuid)));
        }
        for property in &self.properties {
            property.write_property_sexpr(output, indent + 2);
        }
        for pin in &self.pins {
            pin.write_sheet_pin_sexpr(output, indent + 2);
        }
        output.push_str(&format!("{})\n", pad));
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadSheetPin {
    pub name: String,
    pub pin_type: String,
    pub at: Option<KicadAt>,
    pub uuid: Option<String>,
}

impl KicadSheetPin {
    fn write_sheet_pin_sexpr(&self, output: &mut String, indent: usize) {
        let pad = " ".repeat(indent);
        output.push_str(&format!(
            "{}(pin {} {}",
            pad,
            sexpr_string(&self.name),
            sexpr_atom_or_string(&self.pin_type)
        ));
        if let Some(at) = self.at {
            output.push_str(&format!(
                " (at {} {} {})",
                format_number(at.x),
                format_number(at.y),
                format_number(at.rotation)
            ));
        }
        if let Some(uuid) = &self.uuid {
            output.push_str(&format!(" (uuid {})", sexpr_string(uuid)));
        }
        output.push_str(" (effects (font (size 1.27 1.27))))\n");
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadTextItem {
    pub text: String,
    pub at: Option<KicadAt>,
    pub uuid: Option<String>,
}

impl KicadTextItem {
    fn write_text_sexpr(&self, output: &mut String, indent: usize) {
        let pad = " ".repeat(indent);
        output.push_str(&format!("{}(text {}", pad, sexpr_string(&self.text)));
        if let Some(at) = self.at {
            output.push_str(&format!(
                " (at {} {} {})",
                format_number(at.x),
                format_number(at.y),
                format_number(at.rotation)
            ));
        }
        output.push_str(" (effects (font (size 1.27 1.27)))");
        if let Some(uuid) = &self.uuid {
            output.push_str(&format!(" (uuid {})", sexpr_string(uuid)));
        }
        output.push_str(")\n");
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadJunction {
    pub at: KicadPoint,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KicadPoint {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KicadSize {
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KicadAt {
    pub x: f64,
    pub y: f64,
    pub rotation: f64,
}

impl KicadAt {
    fn point(self) -> KicadPoint {
        KicadPoint {
            x: self.x,
            y: self.y,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct PointKey {
    x: i64,
    y: i64,
}

impl From<KicadPoint> for PointKey {
    fn from(point: KicadPoint) -> Self {
        Self {
            x: coordinate_key(point.x),
            y: coordinate_key(point.y),
        }
    }
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

struct SexpParser<'a> {
    input: &'a str,
    offset: usize,
}

impl SexpParser<'_> {
    fn parse_expr(&mut self) -> OslResult<Sexp> {
        self.skip_ws_and_comments();
        match self.peek_byte() {
            Some(b'(') => self.parse_list(),
            Some(b'"') => self.parse_string().map(Sexp::Atom),
            Some(_) => self.parse_atom().map(Sexp::Atom),
            None => Err(OslError::InvalidInput(
                "expected KiCad S-expression, found end of input".to_string(),
            )),
        }
    }

    fn parse_list(&mut self) -> OslResult<Sexp> {
        self.bump_byte();
        let mut items = Vec::new();
        loop {
            self.skip_ws_and_comments();
            match self.peek_byte() {
                Some(b')') => {
                    self.bump_byte();
                    return Ok(Sexp::List(items));
                }
                Some(_) => items.push(self.parse_expr()?),
                None => {
                    return Err(OslError::InvalidInput(
                        "unterminated KiCad S-expression list".to_string(),
                    ));
                }
            }
        }
    }

    fn parse_atom(&mut self) -> OslResult<String> {
        let start = self.offset;
        while let Some(byte) = self.peek_byte() {
            if byte.is_ascii_whitespace() || matches!(byte, b'(' | b')' | b';') {
                break;
            }
            self.bump_byte();
        }
        if self.offset == start {
            Err(OslError::InvalidInput(format!(
                "expected KiCad atom at byte {}",
                self.offset
            )))
        } else {
            Ok(self.input[start..self.offset].to_string())
        }
    }

    fn parse_string(&mut self) -> OslResult<String> {
        self.bump_byte();
        let mut value = String::new();
        while let Some(character) = self.bump_char() {
            match character {
                '"' => return Ok(value),
                '\\' => match self.bump_char() {
                    Some('"') => value.push('"'),
                    Some('\\') => value.push('\\'),
                    Some('n') => value.push('\n'),
                    Some('r') => value.push('\r'),
                    Some('t') => value.push('\t'),
                    Some(other) => value.push(other),
                    None => {
                        return Err(OslError::InvalidInput(
                            "unterminated KiCad string escape".to_string(),
                        ));
                    }
                },
                other => value.push(other),
            }
        }
        Err(OslError::InvalidInput(
            "unterminated KiCad quoted string".to_string(),
        ))
    }

    fn skip_ws_and_comments(&mut self) {
        loop {
            while self
                .peek_byte()
                .is_some_and(|byte| byte.is_ascii_whitespace())
            {
                self.bump_byte();
            }
            if self.peek_byte() == Some(b';') {
                while let Some(byte) = self.peek_byte() {
                    self.bump_byte();
                    if byte == b'\n' {
                        break;
                    }
                }
            } else {
                break;
            }
        }
    }

    fn peek_byte(&self) -> Option<u8> {
        self.input.as_bytes().get(self.offset).copied()
    }

    fn bump_byte(&mut self) -> Option<u8> {
        let byte = self.peek_byte()?;
        self.offset += 1;
        Some(byte)
    }

    fn bump_char(&mut self) -> Option<char> {
        let character = self.input[self.offset..].chars().next()?;
        self.offset += character.len_utf8();
        Some(character)
    }
}

fn expect_root_list<'a>(root: &'a Sexp, expected: &str) -> OslResult<&'a [Sexp]> {
    let items = list_items(root);
    if head(root) == Some(expected) {
        Ok(items)
    } else {
        Err(OslError::InvalidInput(format!(
            "expected KiCad root ({expected} ...)"
        )))
    }
}

fn parse_symbol_instance(node: &Sexp) -> Option<KicadSymbolInstance> {
    let items = list_items(node);
    Some(KicadSymbolInstance {
        lib_id: child_value(items, "lib_id")?,
        at: child(items, "at").and_then(parse_at),
        unit: child_value(items, "unit").and_then(|value| value.parse().ok()),
        uuid: child_value(items, "uuid"),
        exclude_from_sim: child_value(items, "exclude_from_sim").and_then(parse_kicad_bool_value),
        properties: direct_children(items, "property")
            .filter_map(parse_property)
            .collect(),
        pins: direct_children(items, "pin")
            .filter_map(parse_symbol_pin_ref)
            .collect(),
    })
}

fn parse_symbol_pin_ref(node: &Sexp) -> Option<KicadSymbolPinRef> {
    let items = list_items(node);
    Some(KicadSymbolPinRef {
        number: list_value(node, 1),
        uuid: child_value(items, "uuid"),
    })
}

fn parse_symbol_def(node: &Sexp) -> Option<KicadSymbolDef> {
    let items = list_items(node);
    Some(KicadSymbolDef {
        name: list_value(node, 1)?,
        exclude_from_sim: child_value(items, "exclude_from_sim").and_then(parse_kicad_bool_value),
        properties: direct_children(items, "property")
            .filter_map(parse_property)
            .collect(),
        graphics: collect_graphics(node),
        pins: collect_pin_defs(node),
    })
}

fn parse_symbol_library_table_row(node: &Sexp) -> Option<KicadSymbolLibraryTableRow> {
    let items = list_items(node);
    Some(KicadSymbolLibraryTableRow {
        name: child_value(items, "name")?,
        library_type: child_value(items, "type")?,
        uri: child_value(items, "uri")?,
        options: child_value(items, "options"),
        description: child_value(items, "descr"),
        hidden: child(items, "hidden").is_some(),
        disabled: child(items, "disabled").is_some(),
    })
}

fn parse_pin_def(node: &Sexp) -> Option<KicadPinDef> {
    let items = list_items(node);
    Some(KicadPinDef {
        number: child_value(items, "number")?,
        name: child_value(items, "name").unwrap_or_else(|| "~".to_string()),
        electrical_type: list_value(node, 1).unwrap_or_else(|| "unspecified".to_string()),
        shape: list_value(node, 2).unwrap_or_else(|| "line".to_string()),
        at: child(items, "at").and_then(parse_at),
        length: child_value(items, "length").and_then(|value| value.parse().ok()),
    })
}

fn parse_property(node: &Sexp) -> Option<KicadProperty> {
    let items = list_items(node);
    Some(KicadProperty {
        name: list_value(node, 1)?,
        value: list_value(node, 2)?,
        at: child(items, "at").and_then(parse_at),
    })
}

fn parse_wire(node: &Sexp) -> KicadWire {
    let items = list_items(node);
    KicadWire {
        points: child(items, "pts").map(parse_points).unwrap_or_default(),
        uuid: child_value(items, "uuid"),
    }
}

fn parse_label(node: &Sexp, kind: KicadLabelKind) -> Option<KicadLabel> {
    let items = list_items(node);
    Some(KicadLabel {
        text: list_value(node, 1)?,
        kind,
        at: child(items, "at").and_then(parse_at),
        uuid: child_value(items, "uuid"),
    })
}

fn parse_sheet(node: &Sexp) -> Option<KicadSheet> {
    let items = list_items(node);
    Some(KicadSheet {
        at: child(items, "at").and_then(parse_at),
        size: child(items, "size").and_then(parse_size),
        uuid: child_value(items, "uuid"),
        exclude_from_sim: child_value(items, "exclude_from_sim").and_then(parse_kicad_bool_value),
        properties: direct_children(items, "property")
            .filter_map(parse_property)
            .collect(),
        pins: direct_children(items, "pin")
            .filter_map(parse_sheet_pin)
            .collect(),
    })
}

fn parse_sheet_pin(node: &Sexp) -> Option<KicadSheetPin> {
    let items = list_items(node);
    Some(KicadSheetPin {
        name: list_value(node, 1)?,
        pin_type: list_value(node, 2).unwrap_or_else(|| "unspecified".to_string()),
        at: child(items, "at").and_then(parse_at),
        uuid: child_value(items, "uuid"),
    })
}

fn parse_text_item(node: &Sexp) -> Option<KicadTextItem> {
    let items = list_items(node);
    Some(KicadTextItem {
        text: list_value(node, 1)?,
        at: child(items, "at").and_then(parse_at),
        uuid: child_value(items, "uuid"),
    })
}

fn parse_junction(node: &Sexp) -> Option<KicadJunction> {
    child(list_items(node), "at").and_then(parse_point_at)
}

fn parse_points(node: &Sexp) -> Vec<KicadPoint> {
    direct_children(list_items(node), "xy")
        .filter_map(parse_xy)
        .collect()
}

fn parse_xy(node: &Sexp) -> Option<KicadPoint> {
    let items = list_items(node);
    Some(KicadPoint {
        x: atom_text(items.get(1)?)?.parse().ok()?,
        y: atom_text(items.get(2)?)?.parse().ok()?,
    })
}

fn parse_size(node: &Sexp) -> Option<KicadSize> {
    let items = list_items(node);
    Some(KicadSize {
        width: atom_text(items.get(1)?)?.parse().ok()?,
        height: atom_text(items.get(2)?)?.parse().ok()?,
    })
}

fn parse_point_at(node: &Sexp) -> Option<KicadJunction> {
    let at = parse_at(node)?;
    Some(KicadJunction {
        at: KicadPoint { x: at.x, y: at.y },
    })
}

fn parse_at(node: &Sexp) -> Option<KicadAt> {
    let items = list_items(node);
    Some(KicadAt {
        x: atom_text(items.get(1)?)?.parse().ok()?,
        y: atom_text(items.get(2)?)?.parse().ok()?,
        rotation: items
            .get(3)
            .and_then(atom_text)
            .and_then(|value| value.parse().ok())
            .unwrap_or(0.0),
    })
}

fn collect_pin_defs(node: &Sexp) -> Vec<KicadPinDef> {
    let mut pins = Vec::new();
    collect_pin_defs_into(node, &mut pins);
    pins
}

fn collect_pin_defs_into(node: &Sexp, pins: &mut Vec<KicadPinDef>) {
    if head(node) == Some("pin")
        && let Some(pin) = parse_pin_def(node)
    {
        pins.push(pin);
    }
    for child in list_items(node) {
        if matches!(child, Sexp::List(_)) {
            collect_pin_defs_into(child, pins);
        }
    }
}

fn collect_graphics(node: &Sexp) -> Vec<KicadGraphic> {
    let mut graphics = Vec::new();
    collect_graphics_into(node, &mut graphics);
    graphics
}

fn collect_graphics_into(node: &Sexp, graphics: &mut Vec<KicadGraphic>) {
    if let Some(graphic) = parse_graphic(node) {
        graphics.push(graphic);
    }
    for child in list_items(node) {
        if matches!(child, Sexp::List(_)) {
            collect_graphics_into(child, graphics);
        }
    }
}

fn parse_graphic(node: &Sexp) -> Option<KicadGraphic> {
    let items = list_items(node);
    match head(node)? {
        "polyline" => {
            let points = child(items, "pts").map(parse_points).unwrap_or_default();
            (!points.is_empty()).then_some(KicadGraphic::Polyline { points })
        }
        "rectangle" => Some(KicadGraphic::Rectangle {
            start: child(items, "start").and_then(parse_xy)?,
            end: child(items, "end").and_then(parse_xy)?,
        }),
        "circle" => {
            let center = child(items, "center").and_then(parse_xy)?;
            let radius = child_value(items, "radius")
                .and_then(|value| value.parse().ok())
                .or_else(|| {
                    child(items, "end")
                        .and_then(parse_xy)
                        .map(|end| ((end.x - center.x).powi(2) + (end.y - center.y).powi(2)).sqrt())
                })?;
            Some(KicadGraphic::Circle { center, radius })
        }
        "arc" => Some(KicadGraphic::Arc {
            start: child(items, "start").and_then(parse_xy)?,
            mid: child(items, "mid").and_then(parse_xy),
            end: child(items, "end").and_then(parse_xy)?,
        }),
        "text" => Some(KicadGraphic::Text {
            text: list_value(node, 1)?,
            at: child(items, "at").and_then(parse_at),
        }),
        _ => None,
    }
}

fn direct_children<'a>(items: &'a [Sexp], name: &str) -> impl Iterator<Item = &'a Sexp> + 'a {
    let name = name.to_string();
    items
        .iter()
        .filter(move |item| matches!(item, Sexp::List(_)) && head(item) == Some(name.as_str()))
}

fn child<'a>(items: &'a [Sexp], name: &str) -> Option<&'a Sexp> {
    direct_children(items, name).next()
}

fn child_value(items: &[Sexp], name: &str) -> Option<String> {
    child(items, name).and_then(|node| list_value(node, 1))
}

fn list_value(node: &Sexp, index: usize) -> Option<String> {
    list_items(node)
        .get(index)
        .and_then(atom_text)
        .map(str::to_string)
}

fn list_items(node: &Sexp) -> &[Sexp] {
    match node {
        Sexp::List(items) => items,
        Sexp::Atom(_) => &[],
    }
}

fn head(node: &Sexp) -> Option<&str> {
    list_items(node).first().and_then(atom_text)
}

fn atom_text(node: &Sexp) -> Option<&str> {
    match node {
        Sexp::Atom(value) => Some(value),
        Sexp::List(_) => None,
    }
}

fn json_option(value: Option<&str>) -> String {
    match value {
        Some(value) => format!("\"{}\"", json_escape(value)),
        None => "null".to_string(),
    }
}

fn json_u64_option(value: Option<u64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_string())
}

fn json_path_string(root: &serde_json::Value, path: &[&str]) -> Option<String> {
    let mut current = root;
    for key in path {
        current = current.get(*key)?;
    }
    current
        .as_str()
        .map(str::to_string)
        .filter(|value| !value.is_empty())
}

fn json_path_u64(root: &serde_json::Value, path: &[&str]) -> Option<u64> {
    let mut current = root;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_u64()
}

fn parse_kicad_project_sheets(root: &serde_json::Value) -> Vec<KicadProjectSheet> {
    root.get("sheets")
        .and_then(|value| value.as_array())
        .map(|sheets| {
            sheets
                .iter()
                .filter_map(|sheet| {
                    let values = sheet.as_array()?;
                    let uuid = values.first()?.as_str()?;
                    let name = values.get(1)?.as_str()?;
                    Some(KicadProjectSheet {
                        uuid: uuid.to_string(),
                        name: name.to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn path_stem_from_string(path: &str) -> Option<String> {
    Path::new(path)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(str::to_string)
        .filter(|stem| !stem.is_empty())
}

fn push_unique_nonempty(values: &mut Vec<String>, value: Option<&str>) {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return;
    };
    if !values.iter().any(|existing| existing == value) {
        values.push(value.to_string());
    }
}

fn sexpr_string(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len() + 2);
    escaped.push('"');
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            character => escaped.push(character),
        }
    }
    escaped.push('"');
    escaped
}

fn sexpr_atom_or_string(value: &str) -> String {
    if is_plain_sexpr_atom(value) {
        value.to_string()
    } else {
        sexpr_string(value)
    }
}

fn write_points_sexpr(output: &mut String, points: &[KicadPoint]) {
    let points = points
        .iter()
        .map(|point| format!("(xy {} {})", format_number(point.x), format_number(point.y)))
        .collect::<Vec<_>>()
        .join(" ");
    output.push_str(&format!(" (pts {})", points));
}

fn is_plain_sexpr_atom(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| !byte.is_ascii_whitespace() && !matches!(byte, b'(' | b')' | b'"' | b';'))
}

fn format_number(value: f64) -> String {
    let normalized = if value == -0.0 { 0.0 } else { value };
    let mut formatted = format!("{normalized:.12}");
    while formatted.contains('.') && formatted.ends_with('0') {
        formatted.pop();
    }
    if formatted.ends_with('.') {
        formatted.pop();
    }
    formatted
}

fn kicad_bounding_box_json(bounds: KicadBoundingBox) -> String {
    format!(
        concat!(
            "{{ ",
            "\"min\": {{ \"x\": {}, \"y\": {} }}, ",
            "\"max\": {{ \"x\": {}, \"y\": {} }}, ",
            "\"width\": {}, ",
            "\"height\": {} ",
            "}}"
        ),
        bounds.min.x,
        bounds.min.y,
        bounds.max.x,
        bounds.max.y,
        bounds.width(),
        bounds.height()
    )
}

fn resolve_kicad_uri(uri: &str, base_dir: &Path) -> PathBuf {
    let base_dir = normalize_base_dir(base_dir);
    let expanded = expand_kicad_uri(uri, &base_dir);
    let path = PathBuf::from(expanded);
    if path.is_absolute() {
        path
    } else {
        base_dir.join(path)
    }
}

fn normalize_base_dir(base_dir: &Path) -> PathBuf {
    if base_dir.is_absolute() {
        base_dir.to_path_buf()
    } else {
        env::current_dir()
            .map(|cwd| cwd.join(base_dir))
            .unwrap_or_else(|_| base_dir.to_path_buf())
    }
}

fn expand_kicad_uri(uri: &str, base_dir: &Path) -> String {
    let mut expanded = String::new();
    let mut remaining = uri;

    while let Some(start) = remaining.find("${") {
        expanded.push_str(&remaining[..start]);
        let after_start = &remaining[start + 2..];
        let Some(end) = after_start.find('}') else {
            expanded.push_str(&remaining[start..]);
            return expanded;
        };

        let name = &after_start[..end];
        if name == "KIPRJMOD" {
            expanded.push_str(&base_dir.display().to_string());
        } else if let Ok(value) = env::var(name) {
            expanded.push_str(&value);
        } else {
            expanded.push_str("${");
            expanded.push_str(name);
            expanded.push('}');
        }
        remaining = &after_start[end + 1..];
    }

    expanded.push_str(remaining);
    expanded
}

fn transform_symbol_point(pin_at: KicadAt, symbol_at: KicadAt) -> KicadPoint {
    transform_local_point(pin_at.point(), symbol_at)
}

fn transform_local_point(local: KicadPoint, symbol_at: KicadAt) -> KicadPoint {
    let rotated = rotate_point(local, symbol_at.rotation);
    KicadPoint {
        x: symbol_at.x + rotated.x,
        y: symbol_at.y + rotated.y,
    }
}

fn transform_local_at(local_at: KicadAt, symbol_at: KicadAt) -> KicadAt {
    let point = transform_local_point(local_at.point(), symbol_at);
    KicadAt {
        x: point.x,
        y: point.y,
        rotation: normalized_rotation(local_at.rotation + symbol_at.rotation),
    }
}

fn pin_body_end(at: KicadAt, length: f64) -> KicadPoint {
    let radians = at.rotation.to_radians();
    KicadPoint {
        x: at.x + length * radians.cos(),
        y: at.y + length * radians.sin(),
    }
}

fn canvas_symbol_bounds(
    graphics: &[KicadCanvasGraphic],
    pins: &[KicadCanvasPin],
) -> Option<KicadBoundingBox> {
    let mut bounds = KicadBoundingBoxBuilder::default();
    for graphic in graphics {
        graphic.include_in_bounds(&mut bounds);
    }
    for pin in pins {
        bounds.include(pin.start);
        bounds.include(pin.end);
    }
    bounds.finish()
}

fn rotate_point(point: KicadPoint, rotation: f64) -> KicadPoint {
    let normalized = normalized_rotation(rotation).round() as i32;
    match normalized {
        0 => point,
        90 => KicadPoint {
            x: -point.y,
            y: point.x,
        },
        180 => KicadPoint {
            x: -point.x,
            y: -point.y,
        },
        270 => KicadPoint {
            x: point.y,
            y: -point.x,
        },
        _ => {
            let radians = rotation.to_radians();
            KicadPoint {
                x: point.x * radians.cos() - point.y * radians.sin(),
                y: point.x * radians.sin() + point.y * radians.cos(),
            }
        }
    }
}

fn normalized_rotation(rotation: f64) -> f64 {
    let normalized = rotation % 360.0;
    if normalized < 0.0 {
        normalized + 360.0
    } else {
        normalized
    }
}

fn compare_pin_numbers(left: &&KicadPinDef, right: &&KicadPinDef) -> Ordering {
    match (left.number.parse::<u32>(), right.number.parse::<u32>()) {
        (Ok(left), Ok(right)) => left.cmp(&right),
        _ => left.number.cmp(&right.number),
    }
}

fn insert_point(points: &mut BTreeMap<PointKey, KicadPoint>, point: KicadPoint) {
    points.entry(PointKey::from(point)).or_insert(point);
}

fn segment_contains_point(start: KicadPoint, end: KicadPoint, point: KicadPoint) -> bool {
    let cross = (point.y - start.y) * (end.x - start.x) - (point.x - start.x) * (end.y - start.y);
    if cross.abs() > 1e-6 {
        return false;
    }

    between_inclusive(point.x, start.x, end.x) && between_inclusive(point.y, start.y, end.y)
}

fn between_inclusive(value: f64, left: f64, right: f64) -> bool {
    let min = left.min(right) - 1e-6;
    let max = left.max(right) + 1e-6;
    value >= min && value <= max
}

fn coordinate_key(value: f64) -> i64 {
    (value * 1_000_000.0).round() as i64
}

fn normalize_net_name(name: &str) -> String {
    match name.trim().to_ascii_lowercase().as_str() {
        "gnd" | "agnd" | "dgnd" | "earth" => "0".to_string(),
        _ => name.trim().to_string(),
    }
}

fn preferred_net_label(labels: Option<&BTreeSet<String>>) -> Option<String> {
    let labels = labels?;
    labels
        .iter()
        .find(|label| label.as_str() == "0")
        .cloned()
        .or_else(|| labels.iter().find(|label| !label.is_empty()).cloned())
}

fn kicad_schematic_diagnostic(
    severity: KicadDiagnosticSeverity,
    code: &str,
    message: &str,
    item: Option<String>,
    net: Option<String>,
    pin: Option<String>,
) -> KicadSchematicDiagnostic {
    KicadSchematicDiagnostic {
        severity,
        code: code.to_string(),
        message: message.to_string(),
        item,
        net,
        pin,
    }
}

fn library_symbol_definition_for_lib_id(
    library: &KicadSymbolLibrary,
    library_name: &str,
    lib_id: &str,
) -> Option<KicadSymbolDef> {
    if let Some(symbol) = library.symbol(lib_id) {
        return Some(symbol.clone());
    }

    let (requested_library, requested_name) = lib_id.split_once(':')?;
    if requested_library != library_name {
        return None;
    }

    library
        .symbols
        .iter()
        .find(|symbol| symbol.name == requested_name || symbol.local_name() == requested_name)
        .cloned()
        .map(|mut symbol| {
            symbol.name = lib_id.to_string();
            symbol
        })
}

fn spice_primitive_for_device(device: &str) -> Option<String> {
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

fn symbol_ordered_pins<'a>(
    symbol: &'a KicadSymbolInstance,
    definition: &'a KicadSymbolDef,
) -> Vec<&'a KicadPinDef> {
    let mut by_number = definition
        .pins
        .iter()
        .map(|pin| (pin.number.as_str(), pin))
        .collect::<BTreeMap<_, _>>();
    let by_name = definition
        .pins
        .iter()
        .map(|pin| (pin.name.as_str(), pin))
        .collect::<BTreeMap<_, _>>();
    let mut ordered = Vec::new();

    for pin_number in symbol_sim_pin_order(symbol, definition) {
        if let Some(pin) = by_number.remove(pin_number.as_str()) {
            ordered.push(pin);
        } else if let Some(pin) = by_name.get(pin_number.as_str()) {
            ordered.push(*pin);
        }
    }

    if ordered.is_empty() {
        ordered = definition.pins.iter().collect::<Vec<_>>();
        ordered.sort_by(compare_pin_numbers);
    }

    ordered
}

fn symbol_sim_pin_order(symbol: &KicadSymbolInstance, definition: &KicadSymbolDef) -> Vec<String> {
    let Some(pins) = symbol.sim_pins(Some(definition)) else {
        return Vec::new();
    };
    parse_sim_pin_order(pins)
}

fn parse_sim_pin_order(value: &str) -> Vec<String> {
    value
        .split(|character: char| character.is_ascii_whitespace() || character == ',')
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .filter_map(|token| {
            let symbol_pin = token.split_once('=').map(|(left, _)| left).unwrap_or(token);
            let symbol_pin = symbol_pin.trim();
            (!symbol_pin.is_empty()).then(|| symbol_pin.to_string())
        })
        .collect()
}

fn parse_kicad_bool_value(value: String) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "yes" | "true" | "1" => Some(true),
        "no" | "false" | "0" => Some(false),
        _ => None,
    }
}

fn parse_kicad_enable_value(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "y" | "yes" | "true" | "1" | "on" => Some(true),
        "n" | "no" | "false" | "0" | "off" => Some(false),
        _ => None,
    }
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

fn strip_kicad_sim_model_params(value: &str) -> String {
    split_spice_tokens(value)
        .into_iter()
        .filter(|token| {
            token
                .split_once('=')
                .map(|(name, _)| {
                    !matches!(name.trim().to_ascii_lowercase().as_str(), "model" | "lib")
                })
                .unwrap_or(true)
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn extract_named_sim_param(value: &str, name: &str) -> Option<String> {
    for token in split_spice_tokens(value) {
        let Some((left, right)) = token.split_once('=') else {
            continue;
        };
        if left.trim().eq_ignore_ascii_case(name) {
            return Some(unquote_spice_token(right.trim()).to_string());
        }
    }
    None
}

fn split_spice_tokens(value: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut escaped = false;

    for character in value.chars() {
        if escaped {
            current.push(character);
            escaped = false;
            continue;
        }
        if character == '\\' {
            current.push(character);
            escaped = true;
            continue;
        }
        if character == '"' {
            current.push(character);
            in_quotes = !in_quotes;
            continue;
        }
        if character.is_ascii_whitespace() && !in_quotes {
            if !current.is_empty() {
                tokens.push(current.clone());
                current.clear();
            }
        } else {
            current.push(character);
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
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

fn unquote_spice_token(value: &str) -> &str {
    value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .unwrap_or(value)
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

fn symbol_instance_properties(
    definition: &KicadSymbolDef,
    reference: &str,
    value: &str,
    symbol_at: KicadAt,
) -> Vec<KicadProperty> {
    let mut properties = definition
        .properties
        .iter()
        .map(|property| KicadProperty {
            name: property.name.clone(),
            value: match property.name.as_str() {
                "Reference" => reference.to_string(),
                "Value" => value.to_string(),
                _ => property.value.clone(),
            },
            at: property
                .at
                .map(|property_at| transform_local_at(property_at, symbol_at)),
        })
        .collect::<Vec<_>>();

    if !properties
        .iter()
        .any(|property| property.name == "Reference")
    {
        properties.push(KicadProperty {
            name: "Reference".to_string(),
            value: reference.to_string(),
            at: Some(KicadAt {
                x: symbol_at.x,
                y: symbol_at.y - 2.54,
                rotation: symbol_at.rotation,
            }),
        });
    }
    if !properties.iter().any(|property| property.name == "Value") {
        properties.push(KicadProperty {
            name: "Value".to_string(),
            value: value.to_string(),
            at: Some(KicadAt {
                x: symbol_at.x,
                y: symbol_at.y + 2.54,
                rotation: symbol_at.rotation,
            }),
        });
    }

    properties
}

fn validate_point(point: KicadPoint, context: &str) -> OslResult<()> {
    if point.x.is_finite() && point.y.is_finite() {
        Ok(())
    } else {
        Err(OslError::InvalidInput(format!(
            "{context} coordinates must be finite"
        )))
    }
}

fn validate_at(at: KicadAt, context: &str) -> OslResult<()> {
    validate_point(KicadPoint { x: at.x, y: at.y }, context)?;
    if at.rotation.is_finite() {
        Ok(())
    } else {
        Err(OslError::InvalidInput(format!(
            "{context} rotation must be finite"
        )))
    }
}

fn points_payload(points: &[KicadPoint]) -> String {
    points
        .iter()
        .map(|point| format!("{},{}", format_number(point.x), format_number(point.y)))
        .collect::<Vec<_>>()
        .join(";")
}

fn fnv1a64(input: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in input.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn uuid_from_hashes(left: u64, right: u64) -> String {
    let mut bytes = [0_u8; 16];
    bytes[..8].copy_from_slice(&left.to_be_bytes());
    bytes[8..].copy_from_slice(&right.to_be_bytes());
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;

    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0],
        bytes[1],
        bytes[2],
        bytes[3],
        bytes[4],
        bytes[5],
        bytes[6],
        bytes[7],
        bytes[8],
        bytes[9],
        bytes[10],
        bytes[11],
        bytes[12],
        bytes[13],
        bytes[14],
        bytes[15]
    )
}

#[cfg(test)]
mod tests {
    use super::{
        KicadAt, KicadDiagnosticSeverity, KicadLabelKind, KicadPoint, KicadSchematicEdit,
        parse_kicad_project, parse_kicad_schematic, parse_kicad_symbol_library,
        parse_kicad_symbol_library_table, parse_sexpr, read_kicad_project, read_kicad_schematic,
        read_kicad_symbol_library, read_kicad_symbol_library_index,
        read_kicad_symbol_library_table,
    };
    use std::fs;
    use std::path::Path;

    #[test]
    fn parses_kicad_schematic_fixture() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let schematic =
            read_kicad_schematic(&workspace_root.join("examples/kicad_schematic/rc.kicad_sch"))
                .unwrap();

        assert_eq!(schematic.version.as_deref(), Some("20230121"));
        assert_eq!(schematic.paper.as_deref(), Some("A4"));
        assert_eq!(schematic.symbols.len(), 3);
        assert_eq!(schematic.library_symbols.len(), 3);
        assert_eq!(
            schematic
                .library_symbols
                .iter()
                .map(|symbol| symbol.graphics.len())
                .sum::<usize>(),
            6
        );
        assert_eq!(schematic.wires.len(), 3);
        assert_eq!(
            schematic.wires[0].uuid.as_deref(),
            Some("22222222-2222-2222-2222-222222222222")
        );
        assert_eq!(schematic.labels.len(), 3);
        assert_eq!(
            schematic.labels[1].uuid.as_deref(),
            Some("66666666-6666-6666-6666-666666666666")
        );
        assert_eq!(schematic.spice_directives()[0].text, ".tran 1u 1m");
        assert_eq!(
            schematic.spice_directives()[0].uuid.as_deref(),
            Some("77777777-7777-7777-7777-777777777777")
        );
        assert_eq!(schematic.symbols[0].reference(), Some("V1"));
        assert_eq!(schematic.symbols[0].pins[0].number.as_deref(), Some("1"));
        assert_eq!(
            schematic.symbols[0].pins[0].uuid.as_deref(),
            Some("99999999-9999-9999-9999-999999999991")
        );
        assert_eq!(schematic.symbols[1].value(), Some("1k"));
        assert!(
            schematic
                .labels
                .iter()
                .any(|label| label.text == "out" && label.kind == KicadLabelKind::Local)
        );
        assert!(schematic.to_summary_json().contains("\"symbol_count\": 3"));
        assert!(
            schematic
                .to_summary_json()
                .contains("\"library_graphic_count\": 6")
        );
    }

    #[test]
    fn builds_connectivity_and_exports_spice() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let schematic =
            read_kicad_schematic(&workspace_root.join("examples/kicad_schematic/rc.kicad_sch"))
                .unwrap();

        let graph = schematic.connectivity_graph();
        assert_eq!(
            graph
                .nets
                .iter()
                .map(|net| net.name.as_str())
                .collect::<Vec<_>>(),
            ["0", "in", "out"]
        );

        let netlist = schematic.to_spice_netlist().unwrap();
        assert!(netlist.contains("V1 in 0 PULSE(0 1 0 1u 1u 10u 20u)"));
        assert!(netlist.contains("R1 in out 1k"));
        assert!(netlist.contains("C1 out 0 100n"));
        assert!(netlist.contains(".tran 1u 1m"));
        assert!(netlist.ends_with(".end\n"));
    }

    #[test]
    fn checks_kicad_schematic_fixture_without_errors() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let schematic =
            read_kicad_schematic(&workspace_root.join("examples/kicad_schematic/rc.kicad_sch"))
                .unwrap();

        let report = schematic.check_report();

        assert_eq!(report.error_count(), 0);
        assert_eq!(report.symbol_count, 3);
        assert!(report.net_count >= 3);
        assert!(report.to_json().contains("\"error_count\": 0"));
    }

    #[test]
    fn checks_kicad_schematic_structural_diagnostics() {
        let schematic = parse_kicad_schematic(
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols
    (symbol "NekoSpice:R"
      (property "Reference" "R" (at 0 0 0))
      (property "Value" "1k" (at 0 -2.54 0))
      (symbol "R_0_1"
        (pin passive line (at -2.54 0 0) (length 2.54) (name "~") (number "1"))
        (pin passive line (at 2.54 0 180) (length 2.54) (name "~") (number "2"))
      )
    )
  )
  (wire (pts (xy 10 10) (xy 20 10)))
  (label "floating" (at 40 40 0))
  (symbol
    (lib_id "NekoSpice:R")
    (at 12.54 10 0)
    (property "Reference" "R1" (at 12.54 8 0))
    (property "Value" "" (at 12.54 12 0))
  )
  (symbol
    (lib_id "Missing:X")
    (at 30 30 0)
    (property "Reference" "R1" (at 30 28 0))
    (property "Value" "model" (at 30 32 0))
  )
)"#,
            "bad.kicad_sch",
        )
        .unwrap();

        let report = schematic.check_report();
        let codes = report
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect::<Vec<_>>();

        assert!(report.error_count() >= 3);
        assert!(codes.contains(&"duplicate-reference"));
        assert!(codes.contains(&"missing-symbol-definition"));
        assert!(codes.contains(&"missing-ground"));
        assert!(codes.contains(&"missing-value"));
        assert!(codes.contains(&"missing-spice-directive"));
        assert!(report.to_json().contains("\"diagnostic_count\""));
    }

    #[test]
    fn parses_hierarchical_sheet_items_and_reports_unsupported_expansion() {
        let schematic = parse_kicad_schematic(
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (wire (pts (xy 5 5) (xy 10 5)))
  (label "0" (at 5 5 0))
  (text ".op" (at 5 2 0))
  (sheet
    (at 20 10)
    (size 15 10)
    (exclude_from_sim no)
    (uuid "aaaaaaaa-0000-0000-0000-000000000001")
    (property "Sheetname" "gain_stage" (at 20 9 0))
    (property "Sheetfile" "gain_stage.kicad_sch" (at 20 21 0))
    (pin "in" input (at 20 15 180) (uuid "aaaaaaaa-0000-0000-0000-000000000002"))
    (pin "out" output (at 35 15 0) (uuid "aaaaaaaa-0000-0000-0000-000000000003"))
  )
)"#,
            "hierarchical.kicad_sch",
        )
        .unwrap();

        assert_eq!(schematic.sheets.len(), 1);
        assert_eq!(schematic.sheets[0].sheet_name(), Some("gain_stage"));
        assert_eq!(
            schematic.sheets[0].sheet_file(),
            Some("gain_stage.kicad_sch")
        );
        assert_eq!(schematic.sheets[0].pins.len(), 2);
        assert_eq!(schematic.sheets[0].pins[0].pin_type, "input");
        assert_eq!(schematic.sheets[0].bounding_box().unwrap().width(), 15.0);
        assert!(schematic.to_summary_json().contains("\"sheet_count\": 1"));
        assert!(
            schematic
                .to_summary_json()
                .contains("\"sheet_pin_count\": 2")
        );

        let scene = schematic.canvas_scene();
        assert_eq!(scene.sheets.len(), 1);
        assert_eq!(scene.sheets[0].pins.len(), 2);
        assert!(scene.to_summary_json().contains("\"sheet_count\": 1"));
        assert!(scene.to_summary_json().contains("\"sheet_pin_count\": 2"));

        let report = schematic.check_report();
        assert_eq!(report.sheet_count, 1);
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.severity == KicadDiagnosticSeverity::Error
                && diagnostic.code == "hierarchical-sheet-unsupported"
        }));

        let netlist = schematic.to_spice_netlist().unwrap();
        assert!(
            netlist
                .contains("* Unsupported KiCad hierarchical sheet gain_stage gain_stage.kicad_sch")
        );
        let roundtrip = schematic.to_kicad_schematic_sexpr();
        assert!(roundtrip.contains("(sheet"));
        assert!(roundtrip.contains("(property \"Sheetname\" \"gain_stage\""));
        assert!(roundtrip.contains("(pin \"in\" input"));
    }

    #[test]
    fn exports_kicad_sim_fields_to_spice_netlist() {
        let schematic = parse_kicad_schematic(
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols
    (symbol "NekoSpice:Dual"
      (property "Reference" "U" (at 0 0 0))
      (property "Value" "unused" (at 0 -2.54 0))
      (property "Sim.Device" "SUBCKT" (at 0 0 0))
      (property "Sim.Library" "models/opamp.lib" (at 0 0 0))
      (symbol "Dual_0_1"
        (pin passive line (at -2.54 0 0) (length 2.54) (name "IN") (number "1"))
        (pin passive line (at 0 -2.54 90) (length 2.54) (name "OUT") (number "2"))
        (pin passive line (at 2.54 0 180) (length 2.54) (name "VCC") (number "3"))
      )
    )
    (symbol "NekoSpice:R"
      (property "Reference" "R" (at 0 0 0))
      (property "Value" "1k" (at 0 -2.54 0))
      (symbol "R_0_1"
        (pin passive line (at -2.54 0 0) (length 2.54) (name "~") (number "1"))
        (pin passive line (at 2.54 0 180) (length 2.54) (name "~") (number "2"))
      )
    )
  )
  (wire (pts (xy 10 10) (xy 17.46 10)))
  (wire (pts (xy 20 0) (xy 20 7.46)))
  (wire (pts (xy 22.54 10) (xy 30 10)))
  (label "in" (at 10 10 0))
  (label "out" (at 20 0 0))
  (label "vcc" (at 30 10 0))
  (text ".op" (at 5 5 0))
  (symbol
    (lib_id "NekoSpice:Dual")
    (at 20 10 0)
    (property "Reference" "U1" (at 20 8 0))
    (property "Value" "opamp_model" (at 20 12 0))
    (property "Sim.Pins" "2=OUT 1=IN 3=VCC" (at 20 10 0))
    (property "Sim.Params" "model=\"opamp_model\" gain=100k" (at 20 10 0))
  )
  (symbol
    (lib_id "NekoSpice:R")
    (at 50 50 0)
    (exclude_from_sim yes)
    (property "Reference" "Rskip" (at 50 48 0))
    (property "Value" "1k" (at 50 52 0))
  )
)"#,
            "sim_fields.kicad_sch",
        )
        .unwrap();

        let netlist = schematic.to_spice_netlist().unwrap();

        assert!(netlist.contains(".include \"models/opamp.lib\""));
        assert!(netlist.contains("XU1 out in vcc opamp_model gain=100k"));
        assert!(!netlist.contains("Rskip"));
        assert!(netlist.contains(".op"));
        let reparsed = parse_kicad_schematic(
            &schematic.to_kicad_schematic_sexpr(),
            "sim_fields_roundtrip.kicad_sch",
        )
        .unwrap();
        assert_eq!(
            reparsed
                .symbols
                .iter()
                .find(|symbol| symbol.reference() == Some("Rskip"))
                .unwrap()
                .exclude_from_sim,
            Some(true)
        );
    }

    #[test]
    fn exports_legacy_kicad_spice_fields_to_spice_netlist() {
        let schematic = parse_kicad_schematic(
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols
    (symbol "NekoSpice:LegacyD"
      (property "Reference" "D" (at 0 0 0))
      (property "Value" "unused" (at 0 -2.54 0))
      (property "Spice_Primitive" "D" (at 0 0 0))
      (property "Spice_Model" "Dfast" (at 0 0 0))
      (symbol "LegacyD_0_1"
        (pin passive line (at 0 -2.54 90) (length 2.54) (name "A") (number "1"))
        (pin passive line (at 0 2.54 270) (length 2.54) (name "K") (number "2"))
      )
    )
  )
  (wire (pts (xy 40 37.46) (xy 35 37.46)))
  (wire (pts (xy 40 42.54) (xy 45 42.54)))
  (label "anode" (at 35 37.46 0))
  (label "0" (at 45 42.54 0))
  (text ".op" (at 5 5 0))
  (symbol
    (lib_id "NekoSpice:LegacyD")
    (at 40 40 0)
    (property "Reference" "XD1" (at 40 38 0))
    (property "Value" "ignored" (at 40 42 0))
    (property "Spice_Node_Sequence" "2 1" (at 40 40 0))
  )
)"#,
            "legacy_spice_fields.kicad_sch",
        )
        .unwrap();

        let netlist = schematic.to_spice_netlist().unwrap();

        assert!(netlist.contains("DXD1 0 anode Dfast"));
    }

    #[test]
    fn reports_invalid_sim_pin_mapping() {
        let schematic = parse_kicad_schematic(
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols
    (symbol "NekoSpice:R"
      (property "Reference" "R" (at 0 0 0))
      (property "Value" "1k" (at 0 -2.54 0))
      (property "Sim.Device" "R" (at 0 0 0))
      (symbol "R_0_1"
        (pin passive line (at -2.54 0 0) (length 2.54) (name "~") (number "1"))
        (pin passive line (at 2.54 0 180) (length 2.54) (name "~") (number "2"))
      )
    )
  )
  (wire (pts (xy 10 10) (xy 20 10)))
  (label "0" (at 10 10 0))
  (text ".op" (at 5 5 0))
  (symbol
    (lib_id "NekoSpice:R")
    (at 12.54 10 0)
    (property "Reference" "R1" (at 12.54 8 0))
    (property "Value" "1k" (at 12.54 12 0))
    (property "Sim.Pins" "1 99" (at 12.54 10 0))
  )
)"#,
            "bad_sim_pins.kicad_sch",
        )
        .unwrap();

        let report = schematic.check_report();

        assert!(report.error_count() >= 1);
        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "invalid-sim-pin")
        );
    }

    #[test]
    fn resolves_missing_symbols_from_project_library_table() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let project_dir = std::env::temp_dir().join(format!(
            "nekospice_kicad_library_resolution_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&project_dir);
        fs::create_dir_all(&project_dir).unwrap();
        fs::copy(
            workspace_root.join("examples/kicad_schematic/neko_spice.kicad_sym"),
            project_dir.join("neko_spice.kicad_sym"),
        )
        .unwrap();
        fs::write(
            project_dir.join("sym-lib-table"),
            r#"(sym_lib_table
  (version 7)
  (lib (name "NekoSpice")(type "KiCad")(uri "${KIPRJMOD}/neko_spice.kicad_sym")(options "")(descr ""))
)"#,
        )
        .unwrap();
        let mut schematic = parse_kicad_schematic(
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols)
  (wire (pts (xy 10 10) (xy 7 10)))
  (wire (pts (xy 15.08 10) (xy 18 10)))
  (label "in" (at 7 10 0))
  (label "0" (at 18 10 0))
  (text ".op" (at 5 5 0))
  (symbol
    (lib_id "NekoSpice:R")
    (at 12.54 10 0)
    (property "Reference" "R1" (at 12.54 8 0))
    (property "Value" "1k" (at 12.54 12 0))
  )
)"#,
            "library_resolution.kicad_sch",
        )
        .unwrap();

        assert!(
            schematic
                .check_report()
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "missing-symbol-definition")
        );
        let diagnostics = schematic
            .resolve_project_symbol_libraries(&project_dir)
            .unwrap();
        let netlist = schematic.to_spice_netlist().unwrap();

        assert_eq!(diagnostics.len(), 0);
        assert_eq!(schematic.library_symbols.len(), 1);
        assert!(netlist.contains("R1 in 0 1k"));
        assert!(
            !schematic
                .check_report()
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "missing-symbol-definition")
        );

        let _ = fs::remove_dir_all(project_dir);
    }

    #[test]
    fn builds_canvas_scene_from_kicad_schematic_fixture() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let schematic =
            read_kicad_schematic(&workspace_root.join("examples/kicad_schematic/rc.kicad_sch"))
                .unwrap();

        let scene = schematic.canvas_scene();
        assert_eq!(scene.symbols.len(), 3);
        assert_eq!(
            scene
                .symbols
                .iter()
                .map(|symbol| symbol.graphics.len())
                .sum::<usize>(),
            6
        );
        assert_eq!(
            scene
                .symbols
                .iter()
                .map(|symbol| symbol.pins.len())
                .sum::<usize>(),
            6
        );
        assert_eq!(scene.wires.len(), 3);
        assert_eq!(scene.labels.len(), 3);
        assert!(scene.bounds.unwrap().width() > 20.0);

        let resistor = scene
            .symbols
            .iter()
            .find(|symbol| symbol.reference == "R1")
            .unwrap();
        assert_eq!(resistor.lib_id, "NekoSpice:R");
        assert_eq!(resistor.graphics.len(), 1);
        assert_close(resistor.pins[0].start.x, 67.31);
        assert_close(resistor.pins[0].end.x, 69.85);
        assert!(scene.to_summary_json().contains("\"graphic_count\": 6"));
        assert!(scene.to_summary_json().contains("\"pin_count\": 6"));
    }

    #[test]
    fn roundtrips_kicad_schematic_fixture_through_writer() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let schematic =
            read_kicad_schematic(&workspace_root.join("examples/kicad_schematic/rc.kicad_sch"))
                .unwrap();

        let exported = schematic.to_kicad_schematic_sexpr();
        assert!(exported.contains("(kicad_sch"));
        assert!(exported.contains("(lib_symbols"));
        assert!(exported.contains("(lib_id \"NekoSpice:R\")"));
        let reparsed = parse_kicad_schematic(&exported, "roundtrip.kicad_sch").unwrap();

        assert_eq!(reparsed.symbols.len(), 3);
        assert_eq!(reparsed.paper.as_deref(), Some("A4"));
        assert_eq!(reparsed.library_symbols.len(), 3);
        assert_eq!(reparsed.wires.len(), 3);
        assert_eq!(
            reparsed.wires[0].uuid.as_deref(),
            Some("22222222-2222-2222-2222-222222222222")
        );
        assert_eq!(reparsed.labels.len(), 3);
        assert_eq!(
            reparsed.labels[1].uuid.as_deref(),
            Some("66666666-6666-6666-6666-666666666666")
        );
        assert_eq!(reparsed.spice_directives()[0].text, ".tran 1u 1m");
        assert_eq!(
            reparsed.spice_directives()[0].uuid.as_deref(),
            Some("77777777-7777-7777-7777-777777777777")
        );
        assert_eq!(reparsed.symbols[0].pins[0].number.as_deref(), Some("1"));
        assert_eq!(
            reparsed.symbols[0].pins[0].uuid.as_deref(),
            Some("99999999-9999-9999-9999-999999999991")
        );
        assert_eq!(
            reparsed
                .library_symbols
                .iter()
                .map(|symbol| symbol.graphics.len())
                .sum::<usize>(),
            6
        );
        assert!(reparsed.canvas_scene().bounds.is_some());
        let netlist = reparsed.to_spice_netlist().unwrap();
        assert!(netlist.contains("R1 in out 1k"));
        assert!(netlist.contains("C1 out 0 100n"));
    }

    #[test]
    fn edits_kicad_schematic_in_rust_ir_and_roundtrips() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let mut schematic =
            read_kicad_schematic(&workspace_root.join("examples/kicad_schematic/rc.kicad_sch"))
                .unwrap();

        schematic
            .apply_edit(KicadSchematicEdit::MoveSymbol {
                reference: "R1".to_string(),
                to: KicadPoint { x: 73.66, y: 50.8 },
                rotation: Some(0.0),
            })
            .unwrap();
        schematic
            .apply_edit(KicadSchematicEdit::SetSymbolProperty {
                reference: "R1".to_string(),
                name: "Value".to_string(),
                value: "2k".to_string(),
                at: None,
            })
            .unwrap();
        schematic
            .apply_edit(KicadSchematicEdit::AddWire {
                points: vec![
                    KicadPoint { x: 73.66, y: 45.72 },
                    KicadPoint { x: 88.9, y: 45.72 },
                ],
                uuid: Some("eeeeeeee-eeee-eeee-eeee-eeeeeeeeeeee".to_string()),
            })
            .unwrap();
        schematic
            .apply_edit(KicadSchematicEdit::AddLabel {
                text: "sense".to_string(),
                kind: KicadLabelKind::Global,
                at: KicadAt {
                    x: 88.9,
                    y: 45.72,
                    rotation: 0.0,
                },
                uuid: Some("ffffffff-ffff-ffff-ffff-ffffffffffff".to_string()),
            })
            .unwrap();
        schematic
            .apply_edit(KicadSchematicEdit::AddText {
                text: ".save v(sense)".to_string(),
                at: KicadAt {
                    x: 45.72,
                    y: 35.56,
                    rotation: 0.0,
                },
                uuid: Some("abababab-abab-abab-abab-abababababab".to_string()),
            })
            .unwrap();

        let resistor = schematic
            .symbols
            .iter()
            .find(|symbol| symbol.reference() == Some("R1"))
            .unwrap();
        assert_close(resistor.at.unwrap().x, 73.66);
        assert_close(
            resistor
                .properties
                .iter()
                .find(|property| property.name == "Reference")
                .unwrap()
                .at
                .unwrap()
                .x,
            73.66,
        );
        assert_eq!(resistor.value(), Some("2k"));
        assert_eq!(schematic.wires.len(), 4);
        assert!(schematic.labels.iter().any(|label| {
            label.text == "sense"
                && label.kind == KicadLabelKind::Global
                && label.uuid.as_deref() == Some("ffffffff-ffff-ffff-ffff-ffffffffffff")
        }));
        assert!(
            schematic
                .spice_directives()
                .iter()
                .any(|directive| directive.text == ".save v(sense)")
        );

        let exported = schematic.to_kicad_schematic_sexpr();
        assert!(exported.contains("(global_label \"sense\""));
        assert!(exported.contains("(text \".save v(sense)\""));
        let reparsed = parse_kicad_schematic(&exported, "edited.kicad_sch").unwrap();
        assert_eq!(reparsed.wires.len(), 4);
        assert_eq!(
            reparsed
                .symbols
                .iter()
                .find(|symbol| symbol.reference() == Some("R1"))
                .unwrap()
                .value(),
            Some("2k")
        );
        assert!(
            reparsed
                .spice_directives()
                .iter()
                .any(|directive| directive.text == ".save v(sense)")
        );
    }

    #[test]
    fn places_symbol_from_kicad_library_into_schematic_ir() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let mut schematic =
            read_kicad_schematic(&workspace_root.join("examples/kicad_schematic/rc.kicad_sch"))
                .unwrap();
        let library = read_kicad_symbol_library(
            &workspace_root.join("examples/kicad_schematic/neko_spice.kicad_sym"),
        )
        .unwrap();
        let capacitor = library.symbol("NekoSpice:C").unwrap().clone();

        schematic
            .apply_edit(KicadSchematicEdit::PlaceSymbol {
                definition: capacitor,
                reference: "C2".to_string(),
                value: "47n".to_string(),
                at: KicadAt {
                    x: 101.6,
                    y: 53.34,
                    rotation: 0.0,
                },
                unit: Some(1),
                uuid: Some("eeeeeeee-eeee-eeee-eeee-eeeeeeeeeeee".to_string()),
            })
            .unwrap();

        let placed = schematic
            .symbols
            .iter()
            .find(|symbol| symbol.reference() == Some("C2"))
            .unwrap();
        assert_eq!(placed.lib_id, "NekoSpice:C");
        assert_eq!(placed.value(), Some("47n"));
        assert_eq!(placed.pins.len(), 2);
        assert!(placed.pins.iter().all(|pin| pin.uuid.is_some()));
        assert!(
            schematic
                .library_symbols
                .iter()
                .any(|symbol| symbol.name == "NekoSpice:C")
        );

        let exported = schematic.to_kicad_schematic_sexpr();
        assert!(exported.contains("(property \"Reference\" \"C2\""));
        assert!(exported.contains("(property \"Value\" \"47n\""));
        let reparsed = parse_kicad_schematic(&exported, "placed.kicad_sch").unwrap();
        assert!(
            reparsed
                .canvas_scene()
                .symbols
                .iter()
                .any(|symbol| symbol.reference == "C2" && symbol.pins.len() == 2)
        );
    }

    #[test]
    fn rejects_edit_that_reuses_existing_kicad_uuid() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let mut schematic =
            read_kicad_schematic(&workspace_root.join("examples/kicad_schematic/rc.kicad_sch"))
                .unwrap();

        let error = schematic
            .apply_edit(KicadSchematicEdit::AddWire {
                points: vec![
                    KicadPoint { x: 10.0, y: 10.0 },
                    KicadPoint { x: 20.0, y: 10.0 },
                ],
                uuid: Some("22222222-2222-2222-2222-222222222222".to_string()),
            })
            .unwrap_err();

        assert!(error.to_string().contains("already used"));
    }

    #[test]
    fn parses_kicad_symbol_library_fixture() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let library = read_kicad_symbol_library(
            &workspace_root.join("examples/kicad_schematic/neko_spice.kicad_sym"),
        )
        .unwrap();

        let resistor = library.symbol("NekoSpice:R").unwrap();
        assert_eq!(resistor.property("Reference"), Some("R"));
        assert_eq!(resistor.graphics.len(), 1);
        assert_eq!(resistor.pins.len(), 2);
        assert_eq!(resistor.pins[0].number, "1");
        assert_eq!(resistor.pins[0].electrical_type, "passive");
        let bounds = resistor.bounding_box().unwrap();
        assert_eq!(bounds.min.x, -2.54);
        assert_eq!(bounds.max.x, 2.54);
        assert!(bounds.width() > 5.0);
        assert!(library.to_summary_json().contains("\"symbol_count\": 3"));
        assert!(library.to_summary_json().contains("\"graphic_count\": 6"));
    }

    #[test]
    fn roundtrips_kicad_symbol_library_fixture_through_writer() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let library = read_kicad_symbol_library(
            &workspace_root.join("examples/kicad_schematic/neko_spice.kicad_sym"),
        )
        .unwrap();

        let exported = library.to_kicad_symbol_library_sexpr();
        assert!(exported.contains("(kicad_symbol_lib"));
        assert!(exported.contains("(symbol \"NekoSpice:R\""));
        let reparsed = parse_kicad_symbol_library(&exported, "roundtrip.kicad_sym").unwrap();

        assert_eq!(reparsed.symbols.len(), library.symbols.len());
        assert_eq!(
            reparsed
                .symbols
                .iter()
                .map(|symbol| symbol.graphics.len())
                .sum::<usize>(),
            6
        );
        let resistor = reparsed.symbol("NekoSpice:R").unwrap();
        assert_eq!(resistor.pins.len(), 2);
        assert_eq!(resistor.property("Reference"), Some("R"));
        assert_eq!(resistor.graphics.len(), 1);
        let bounds = resistor.bounding_box().unwrap();
        assert_close(bounds.min.x, -2.54);
        assert_close(bounds.max.x, 2.54);
    }

    #[test]
    fn parses_kicad_symbol_library_table_fixture() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let table = read_kicad_symbol_library_table(
            &workspace_root.join("examples/kicad_schematic/sym-lib-table"),
        )
        .unwrap();

        assert_eq!(table.version.as_deref(), Some("7"));
        assert_eq!(table.libraries.len(), 1);
        assert_eq!(table.libraries[0].name, "NekoSpice");
        assert_eq!(table.libraries[0].library_type, "KiCad");
        assert_eq!(
            table.libraries[0].description.as_deref(),
            Some("NekoSpice analog simulation symbols")
        );
        assert_eq!(table.enabled_kicad_libraries().count(), 1);
        assert!(table.to_summary_json().contains("\"library_count\": 1"));
    }

    #[test]
    fn parses_kicad_project_fixture_and_sheet_summary() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let project = read_kicad_project(
            &workspace_root
                .join("examples/kicad_project_schematic/kicad_project_schematic.kicad_pro"),
        )
        .unwrap();

        assert_eq!(
            project.meta_filename.as_deref(),
            Some("kicad_project_schematic.kicad_pro")
        );
        assert_eq!(project.meta_version, Some(1));
        assert_eq!(
            project.project_name.as_deref(),
            Some("kicad_project_schematic")
        );
        assert!(
            project
                .schematic_stem_candidates()
                .contains(&"kicad_project_schematic".to_string())
        );
        assert!(project.to_summary_json().contains("\"project_name\""));

        let project = parse_kicad_project(
            r#"{
  "meta": { "filename": "root_project.kicad_pro", "version": 2 },
  "schematic": { "page_layout_descr_file": "layout.kicad_wks" },
  "sheets": [
    [ "root-sheet", "Root" ],
    [ "child-sheet", "child" ]
  ],
  "text_variables": { "REV": "A" }
}"#,
            "root_project.kicad_pro",
        )
        .unwrap();

        assert_eq!(project.meta_version, Some(2));
        assert_eq!(
            project.schematic_page_layout_descr_file.as_deref(),
            Some("layout.kicad_wks")
        );
        assert_eq!(project.sheets.len(), 2);
        assert_eq!(project.sheets[0].name, "Root");
        assert_eq!(project.sheets[1].uuid, "child-sheet");
        assert_eq!(project.text_variable_count, 1);
        assert_eq!(
            project.schematic_stem_candidates(),
            vec!["root_project".to_string()]
        );
    }

    #[test]
    fn builds_kicad_symbol_library_index_fixture() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let index = read_kicad_symbol_library_index(
            &workspace_root.join("examples/kicad_schematic/sym-lib-table"),
        )
        .unwrap();

        assert_eq!(index.libraries.len(), 1);
        assert_eq!(index.symbols.len(), 3);
        assert_eq!(index.diagnostics.len(), 0);
        let resistor = index.symbol("NekoSpice:R").unwrap();
        assert_eq!(resistor.library, "NekoSpice");
        assert_eq!(resistor.name, "R");
        assert_eq!(resistor.pin_count, 2);
        assert_eq!(resistor.graphic_count, 1);
        assert!(resistor.bounding_box.is_some());
        assert!(index.to_summary_json().contains("\"symbol_count\": 3"));
    }

    #[test]
    fn indexes_kicad_library_table_diagnostics() {
        let table = parse_kicad_symbol_library_table(
            r#"(sym_lib_table
  (version 7)
  (lib (name "Disabled")(type "KiCad")(uri "disabled.kicad_sym")(options "")(descr "")(disabled))
  (lib (name "Future")(type "FutureCAD")(uri "future.kicad_sym")(options "")(descr ""))
)"#,
            "inline",
        )
        .unwrap();

        let index = super::KicadSymbolLibraryIndex::from_table(table, Path::new("."));
        assert_eq!(index.libraries.len(), 0);
        assert_eq!(index.symbols.len(), 0);
        assert_eq!(index.diagnostics.len(), 2);
        assert!(
            index
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message == "library row is disabled")
        );
        assert!(index.diagnostics.iter().any(|diagnostic| {
            diagnostic
                .message
                .contains("unsupported symbol library type")
        }));
    }

    #[test]
    fn parses_quoted_strings_and_comments() {
        let parsed =
            parse_sexpr("(root ; comment\n  \"quoted value\" (child \"a\\\\b\"))").unwrap();
        let items = match parsed {
            super::Sexp::List(items) => items,
            super::Sexp::Atom(_) => panic!("root should be a list"),
        };

        assert_eq!(items.len(), 3);
    }

    #[test]
    fn rejects_wrong_kicad_root() {
        let error = parse_kicad_schematic("(kicad_symbol_lib)", "bad.kicad_sch").unwrap_err();
        assert!(error.to_string().contains("expected KiCad root"));

        let error = parse_kicad_symbol_library("(kicad_sch)", "bad.kicad_sym").unwrap_err();
        assert!(error.to_string().contains("expected KiCad root"));

        let error = parse_kicad_symbol_library_table("(kicad_sch)", "sym-lib-table").unwrap_err();
        assert!(error.to_string().contains("expected KiCad root"));
    }

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1e-9,
            "expected {actual} to be close to {expected}"
        );
    }
}
