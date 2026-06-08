use osl_core::{OslError, OslResult, json_escape, read_text};
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

pub fn read_kicad_symbol_library(path: &Path) -> OslResult<KicadSymbolLibrary> {
    let content = read_text(path)?;
    parse_kicad_symbol_library(&content, &path.display().to_string())
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
    pub library_symbols: Vec<KicadSymbolDef>,
    pub symbols: Vec<KicadSymbolInstance>,
    pub wires: Vec<KicadWire>,
    pub labels: Vec<KicadLabel>,
    pub text_items: Vec<KicadTextItem>,
    pub junctions: Vec<KicadJunction>,
}

impl KicadSchematic {
    pub fn connectivity_graph(&self) -> KicadNetGraph {
        KicadNetGraph::build(self)
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
    pub pins: Vec<String>,
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

#[derive(Debug, Clone, PartialEq)]
pub struct KicadProperty {
    pub name: String,
    pub value: String,
    pub at: Option<KicadAt>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadWire {
    pub points: Vec<KicadPoint>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadLabel {
    pub text: String,
    pub kind: KicadLabelKind,
    pub at: Option<KicadAt>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KicadLabelKind {
    Local,
    Global,
    Hierarchical,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadTextItem {
    pub text: String,
    pub at: Option<KicadAt>,
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
            .filter_map(|pin| child_value(list_items(pin), "uuid").or_else(|| list_value(pin, 1)))
            .collect(),
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
    }
}

fn parse_label(node: &Sexp, kind: KicadLabelKind) -> Option<KicadLabel> {
    let items = list_items(node);
    Some(KicadLabel {
        text: list_value(node, 1)?,
        kind,
        at: child(items, "at").and_then(parse_at),
    })
}

fn parse_text_item(node: &Sexp) -> Option<KicadTextItem> {
    let items = list_items(node);
    Some(KicadTextItem {
        text: list_value(node, 1)?,
        at: child(items, "at").and_then(parse_at),
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
    let local = KicadPoint {
        x: pin_at.x,
        y: pin_at.y,
    };
    let rotated = rotate_point(local, symbol_at.rotation);
    KicadPoint {
        x: symbol_at.x + rotated.x,
        y: symbol_at.y + rotated.y,
    }
}

fn pin_body_end(at: KicadAt, length: f64) -> KicadPoint {
    let radians = at.rotation.to_radians();
    KicadPoint {
        x: at.x + length * radians.cos(),
        y: at.y + length * radians.sin(),
    }
}

fn rotate_point(point: KicadPoint, rotation: f64) -> KicadPoint {
    let normalized = ((rotation.round() as i32 % 360) + 360) % 360;
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

#[cfg(test)]
mod tests {
    use super::{
        KicadLabelKind, parse_kicad_schematic, parse_kicad_symbol_library,
        parse_kicad_symbol_library_table, parse_sexpr, read_kicad_schematic,
        read_kicad_symbol_library, read_kicad_symbol_library_index,
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
        assert_eq!(schematic.labels.len(), 3);
        assert_eq!(schematic.spice_directives()[0].text, ".tran 1u 1m");
        assert_eq!(schematic.symbols[0].reference(), Some("V1"));
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
}
