use osl_core::{OslError, OslResult, json_escape, read_text, write_text};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
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
    pub text_items: Vec<KicadTextItem>,
    pub junctions: Vec<KicadJunction>,
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

    pub fn to_spice_netlist(&self) -> OslResult<String> {
        let graph = self.connectivity_graph();
        let mut lines = vec![format!("* Imported from KiCad schematic: {}", self.source)];

        for symbol in &self.symbols {
            match self.symbol_to_spice_line(symbol, &graph) {
                Some(line) => lines.push(line),
                None => lines.push(format!(
                    "* Unsupported KiCad symbol {} {}",
                    symbol.reference().unwrap_or("<no-reference>"),
                    symbol.lib_id
                )),
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
        let reference = symbol.reference()?.trim();
        if reference.is_empty() || reference.starts_with('#') {
            return None;
        }

        let value = symbol.value().unwrap_or_default().trim();
        let nodes = self.symbol_pin_nets(symbol, graph)?;
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
        let mut pins = definition.pins.iter().collect::<Vec<_>>();
        pins.sort_by(compare_pin_numbers);

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
        let used = self.used_uuids();
        if let Some(uuid) = uuid.filter(|uuid| !uuid.trim().is_empty()) {
            if used.contains(&uuid) {
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
            if !used.contains(&candidate) {
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
            self.text_items.len(),
            self.spice_directives().len(),
            self.library_symbols
                .iter()
                .map(|symbol| symbol.graphics.len())
                .sum::<usize>()
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadCanvasScene {
    pub source: String,
    pub symbols: Vec<KicadCanvasSymbol>,
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
                "  \"graphic_count\": {},\n",
                "  \"pin_count\": {},\n",
                "  \"wire_count\": {},\n",
                "  \"label_count\": {},\n",
                "  \"junction_count\": {},\n",
                "  \"bounds\": {}\n",
                "}}"
            ),
            json_escape(&self.source),
            self.symbols.len(),
            graphic_count,
            pin_count,
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
        KicadAt, KicadLabelKind, KicadPoint, KicadSchematicEdit, parse_kicad_schematic,
        parse_kicad_symbol_library, parse_kicad_symbol_library_table, parse_sexpr,
        read_kicad_schematic, read_kicad_symbol_library, read_kicad_symbol_library_index,
        read_kicad_symbol_library_table,
    };
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
