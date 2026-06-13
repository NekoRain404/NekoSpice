// LTspice schematic types and data structures.
include!("ltspice_types_impl.rs");

impl LtspiceSymbolLibrary {
    fn new(base_dir: &Path) -> Self {
        let search_dirs = ltspice_symbol_search_dirs(base_dir);
        Self {
            search_dirs,
            cache: BTreeMap::new(),
            diagnostics: Vec::new(),
        }
    }

    fn spec_for(&mut self, symbol: &LtspiceSymbol) -> Option<LtspiceSymbolSpec> {
        let key = ltspice_symbol_basename(&symbol.name);
        if let Some(spec) = self.cache.get(&key) {
            return spec.clone();
        }

        let spec = self
            .read_symbol_spec(symbol)
            .or_else(|| ltspice_builtin_symbol(&symbol.name));
        self.cache.insert(key, spec.clone());
        spec
    }

    fn read_symbol_spec(&mut self, symbol: &LtspiceSymbol) -> Option<LtspiceSymbolSpec> {
        let path = self.symbol_path(&symbol.name)?;
        let content = match read_text(&path) {
            Ok(content) => content,
            Err(error) => {
                self.diagnostics.push(import_diagnostic(
                    symbol.line,
                    ImportSeverity::Warning,
                    "ltspice_symbol_read_failed",
                    &format!("could not read LTspice symbol {}: {error}", path.display()),
                    "Keep custom .asy files next to the imported .asc or use supported primitive symbols.",
                ));
                return None;
            }
        };
        match parse_ltspice_asy_spec(&content) {
            Some(spec) => Some(spec),
            None => {
                self.diagnostics.push(import_diagnostic(
                    symbol.line,
                    ImportSeverity::Warning,
                    "ltspice_symbol_no_pins",
                    &format!("LTspice symbol {} contains no ordered pins", path.display()),
                    "Add PIN entries with PINATTR SpiceOrder or use a symbol with explicit pin metadata.",
                ));
                None
            }
        }
    }

    fn symbol_path(&self, symbol_name: &str) -> Option<PathBuf> {
        let normalized = symbol_name.replace('\\', "/");
        let raw_path = Path::new(&normalized);
        self.search_dirs
            .iter()
            .flat_map(|search_dir| symbol_path_candidates(search_dir, raw_path, &normalized))
            .find(|candidate| candidate.is_file())
    }
}

fn ltspice_symbol_search_dirs(base_dir: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    push_unique_path(&mut dirs, base_dir.to_path_buf());
    push_unique_path(&mut dirs, base_dir.join("sym"));

    if let Some(paths) = env::var_os("NEKOSPICE_LTSPICE_SYM_PATH") {
        for path in env::split_paths(&paths) {
            push_unique_path(&mut dirs, path);
        }
    }
    if let Some(home) = env::var_os("HOME").map(PathBuf::from) {
        push_unique_path(&mut dirs, home.join(".local/share/LTspice/lib/sym"));
        push_unique_path(
            &mut dirs,
            home.join(".local/share/wineprefixes/ltspice/drive_c/users")
                .join(env::var("USER").unwrap_or_else(|_| "user".to_string()))
                .join("AppData/Local/LTspice/lib/sym"),
        );
        push_unique_path(&mut dirs, home.join("Documents/LTspiceXVII/lib/sym"));
    }
    push_unique_path(
        &mut dirs,
        PathBuf::from("/Applications/LTspice.app/Contents/lib/sym"),
    );
    push_unique_path(
        &mut dirs,
        PathBuf::from("C:/Users/Public/Documents/LTspiceXVII/lib/sym"),
    );
    dirs
}

fn symbol_path_candidates(search_dir: &Path, raw_path: &Path, normalized: &str) -> Vec<PathBuf> {
    if raw_path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("asy"))
    {
        vec![search_dir.join(raw_path)]
    } else {
        vec![
            search_dir.join(format!("{normalized}.asy")),
            search_dir.join(raw_path).with_extension("asy"),
        ]
    }
}

fn push_unique_path(paths: &mut Vec<PathBuf>, path: PathBuf) {
    if !paths.iter().any(|existing| existing == &path) {
        paths.push(path);
    }
}

impl AscNetGraph {
    fn build(wires: &[LtspiceWire], flags: &[LtspiceFlag], pin_points: &[AscPoint]) -> AscNetGraph {
        let mut points = BTreeSet::new();
        for wire in wires {
            points.insert(wire.start);
            points.insert(wire.end);
        }
        for flag in flags {
            points.insert(flag.point);
        }
        for point in pin_points {
            points.insert(*point);
        }
        for left in wires {
            for right in wires {
                if let Some(point) = wire_intersection(left, right) {
                    points.insert(point);
                }
            }
        }

        let ordered_points = points.into_iter().collect::<Vec<_>>();
        let point_indexes = ordered_points
            .iter()
            .enumerate()
            .map(|(index, point)| (*point, index))
            .collect::<BTreeMap<_, _>>();
        let mut graph = DisjointSet::new(ordered_points.len());

        for wire in wires {
            let mut segment_points = ordered_points
                .iter()
                .filter(|point| wire_contains_point(wire, **point))
                .filter_map(|point| point_indexes.get(point).copied())
                .collect::<Vec<_>>();
            segment_points.sort_by_key(|index| ordered_points[*index]);
            if let Some(first) = segment_points.first().copied() {
                for point in segment_points.iter().copied().skip(1) {
                    graph.union(first, point);
                }
            }
        }

        let mut flags_by_name = BTreeMap::<String, Vec<usize>>::new();
        for flag in flags {
            if let Some(index) = point_indexes.get(&flag.point).copied() {
                flags_by_name
                    .entry(flag.name.clone())
                    .or_default()
                    .push(index);
            }
        }
        for flag_points in flags_by_name.values() {
            if let Some(first) = flag_points.first().copied() {
                for point in flag_points.iter().copied().skip(1) {
                    graph.union(first, point);
                }
            }
        }

        let mut labels_by_root = BTreeMap::<usize, BTreeSet<String>>::new();
        for flag in flags {
            if let Some(index) = point_indexes.get(&flag.point).copied() {
                let root = graph.find(index);
                labels_by_root
                    .entry(root)
                    .or_default()
                    .insert(flag.name.clone());
            }
        }

        let mut names_by_root = BTreeMap::<usize, String>::new();
        let mut generated_index = 1;
        for (index, _) in ordered_points.iter().enumerate() {
            let root = graph.find(index);
            names_by_root.entry(root).or_insert_with(|| {
                labels_by_root
                    .get(&root)
                    .and_then(preferred_label)
                    .unwrap_or_else(|| {
                        let name = format!("n{generated_index:03}");
                        generated_index += 1;
                        name
                    })
            });
        }

        let mut has_ground = false;
        let mut names = BTreeMap::new();
        for (index, point) in ordered_points.iter().enumerate() {
            let root = graph.find(index);
            let name = names_by_root.get(&root).cloned().unwrap_or_else(|| {
                let name = format!("n{generated_index:03}");
                generated_index += 1;
                name
            });
            if is_ground_node(&name.to_ascii_lowercase()) {
                has_ground = true;
            }
            names.insert(*point, name);
        }

        Self { names, has_ground }
    }

    fn node_name(&self, point: AscPoint) -> Option<&str> {
        self.names.get(&point).map(String::as_str)
    }
}

/// import ltspice asc。
pub(crate) fn import_ltspice_asc(
    input: &str,
    source: &str,
    base_dir: &Path,
) -> LtspiceSchematicImport {
    let mut schematic = parse_ltspice_asc(input);
    let mut library = LtspiceSymbolLibrary::new(base_dir);
    let pin_points = schematic
        .symbols
        .iter()
        .flat_map(|symbol| ltspice_symbol_pin_points(symbol, &mut library))
        .collect::<Vec<_>>();
    schematic.diagnostics.append(&mut library.diagnostics);
    let graph = AscNetGraph::build(&schematic.wires, &schematic.flags, &pin_points);

    let mut lines = vec![
        format!("* Imported from LTspice schematic: {source}"),
        "* Generated by NekoSpice asc importer.".to_string(),
    ];
    for symbol in &schematic.symbols {
        if let Some(line) =
            ltspice_symbol_to_netlist(symbol, &graph, &mut library, &mut schematic.diagnostics)
        {
            lines.push(line);
        }
    }
    schematic.diagnostics.append(&mut library.diagnostics);
    for directive in &schematic.directives {
        lines.push(directive.text.clone());
    }
    if !lines
        .iter()
        .any(|line| line.trim().eq_ignore_ascii_case(".end"))
    {
        lines.push(".end".to_string());
    }

    if !schematic.symbols.is_empty() && !graph.has_ground {
        schematic.diagnostics.push(import_diagnostic(
            1,
            ImportSeverity::Warning,
            "ltspice_missing_ground",
            "LTspice schematic has no node labelled 0 or ground",
            "Add a ground symbol or FLAG 0 before running the imported netlist.",
        ));
    }

    LtspiceSchematicImport {
        netlist: format!("{}\n", lines.join("\n")),
        diagnostics: schematic.diagnostics,
    }
}

fn parse_ltspice_asc(input: &str) -> LtspiceSchematic {
    let mut schematic = LtspiceSchematic::default();
    let mut current_symbol = None::<LtspiceSymbol>;

    for (line_number, raw_line) in input.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let tokens = line.split_whitespace().collect::<Vec<_>>();
        let Some(keyword) = tokens.first().copied() else {
            continue;
        };

        match keyword {
            "WIRE" => match parse_wire_tokens(&tokens) {
                Some(wire) => schematic.wires.push(wire),
                None => schematic.diagnostics.push(import_diagnostic(
                    line_number + 1,
                    ImportSeverity::Error,
                    "ltspice_bad_wire",
                    "WIRE statement must contain four integer coordinates",
                    "Check the LTspice schematic export around this line.",
                )),
            },
            "FLAG" => match parse_flag_tokens(&tokens) {
                Some(flag) => schematic.flags.push(flag),
                None => schematic.diagnostics.push(import_diagnostic(
                    line_number + 1,
                    ImportSeverity::Error,
                    "ltspice_bad_flag",
                    "FLAG statement must contain x, y, and node name",
                    "Check the LTspice schematic export around this line.",
                )),
            },
            "SYMBOL" => {
                if let Some(symbol) = current_symbol.take() {
                    schematic.symbols.push(symbol);
                }
                match parse_symbol_tokens(&tokens, line_number + 1) {
                    Some(symbol) => current_symbol = Some(symbol),
                    None => schematic.diagnostics.push(import_diagnostic(
                        line_number + 1,
                        ImportSeverity::Error,
                        "ltspice_bad_symbol",
                        "SYMBOL statement must contain name, x, y, and rotation",
                        "Check the LTspice schematic export around this line.",
                    )),
                }
            }
            "SYMATTR" => {
                if let Some(symbol) = current_symbol.as_mut() {
                    if let Some((key, value)) = split_key_value(line.trim_start_matches("SYMATTR"))
                    {
                        symbol.attrs.insert(key.to_string(), value.to_string());
                    }
                } else {
                    schematic.diagnostics.push(import_diagnostic(
                        line_number + 1,
                        ImportSeverity::Warning,
                        "ltspice_orphan_attribute",
                        "SYMATTR appears before any SYMBOL",
                        "Move the attribute below the symbol it belongs to.",
                    ));
                }
            }
            "TEXT" => {
                if let Some(directive) = parse_text_directive(&tokens) {
                    schematic.directives.push(directive);
                }
            }
            _ => {}
        }
    }

    if let Some(symbol) = current_symbol.take() {
        schematic.symbols.push(symbol);
    }
    schematic
}

fn parse_wire_tokens(tokens: &[&str]) -> Option<LtspiceWire> {
    Some(LtspiceWire {
        start: AscPoint::new(tokens.get(1)?.parse().ok()?, tokens.get(2)?.parse().ok()?),
        end: AscPoint::new(tokens.get(3)?.parse().ok()?, tokens.get(4)?.parse().ok()?),
    })
}

fn parse_flag_tokens(tokens: &[&str]) -> Option<LtspiceFlag> {
    Some(LtspiceFlag {
        point: AscPoint::new(tokens.get(1)?.parse().ok()?, tokens.get(2)?.parse().ok()?),
        name: tokens.get(3)?.to_string(),
    })
}

fn parse_symbol_tokens(tokens: &[&str], line: usize) -> Option<LtspiceSymbol> {
    Some(LtspiceSymbol {
        line,
        name: tokens.get(1)?.to_string(),
        origin: AscPoint::new(tokens.get(2)?.parse().ok()?, tokens.get(3)?.parse().ok()?),
        rotation: tokens.get(4)?.to_string(),
        attrs: BTreeMap::new(),
    })
}

fn parse_text_directive(tokens: &[&str]) -> Option<LtspiceDirective> {
    let text = tokens.get(5..)?.join(" ");
    let directive = text.strip_prefix('!')?.trim();
    if directive.is_empty() {
        None
    } else {
        Some(LtspiceDirective {
            text: directive.to_string(),
        })
    }
}

fn split_key_value(input: &str) -> Option<(&str, &str)> {
    let input = input.trim();
    let split_at = input
        .char_indices()
        .find(|(_, character)| character.is_whitespace())
        .map(|(index, _)| index)?;
    let key = input[..split_at].trim();
    let value = input[split_at..].trim();
    if key.is_empty() {
        None
    } else {
        Some((key, value))
    }
}

#[derive(Debug)]
struct LtspiceAsyPin {
    point: AscPoint,
    spice_order: Option<usize>,
}

fn parse_ltspice_asy_spec(input: &str) -> Option<LtspiceSymbolSpec> {
    let mut prefix = None::<String>;
    let mut pins = Vec::new();

    for raw_line in input.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let tokens = line.split_whitespace().collect::<Vec<_>>();
        let Some(keyword) = tokens.first().copied() else {
            continue;
        };

        match keyword {
            "SYMATTR" if tokens.len() >= 3 && tokens[1] == "Prefix" => {
                prefix = Some(tokens[2].to_string());
            }
            "PIN" => {
                let Some(x) = tokens.get(1).and_then(|value| value.parse::<i32>().ok()) else {
                    continue;
                };
                let Some(y) = tokens.get(2).and_then(|value| value.parse::<i32>().ok()) else {
                    continue;
                };
                pins.push(LtspiceAsyPin {
                    point: AscPoint::new(x, y),
                    spice_order: None,
                });
            }
            "PINATTR" if tokens.len() >= 3 && tokens[1] == "SpiceOrder" => {
                if let Some(pin) = pins.last_mut() {
                    pin.spice_order = tokens[2].parse::<usize>().ok();
                }
            }
            _ => {}
        }
    }

    if pins.is_empty() {
        return None;
    }

    pins.sort_by_key(|pin| pin.spice_order.unwrap_or(usize::MAX));
    Some(LtspiceSymbolSpec {
        prefix: prefix.unwrap_or_else(|| "X".to_string()),
        pins: pins.into_iter().map(|pin| pin.point).collect(),
        source: LtspiceSymbolSpecSource::AsyFile,
    })
}

fn ltspice_symbol_pin_points(
    symbol: &LtspiceSymbol,
    library: &mut LtspiceSymbolLibrary,
) -> Vec<AscPoint> {
    let Some(spec) = library.spec_for(symbol) else {
        return Vec::new();
    };
    spec.pins
        .iter()
        .filter_map(|pin| transform_ltspice_point(*pin, symbol.origin, &symbol.rotation))
        .collect()
}

fn ltspice_symbol_to_netlist(
    symbol: &LtspiceSymbol,
    graph: &AscNetGraph,
    library: &mut LtspiceSymbolLibrary,
    diagnostics: &mut Vec<ImportDiagnostic>,
) -> Option<String> {
    let Some(spec) = library.spec_for(symbol) else {
        diagnostics.push(import_diagnostic(
            symbol.line,
            ImportSeverity::Error,
            "ltspice_unsupported_symbol",
            &format!(
                "LTspice symbol '{}' is not supported by the importer yet",
                symbol.name
            ),
            "Replace it with a supported primitive or add an .asy pin mapping rule.",
        ));
        return None;
    };
    let pins = ltspice_symbol_pin_points(symbol, library);
    if pins.len() != spec.pins.len() {
        diagnostics.push(import_diagnostic(
            symbol.line,
            ImportSeverity::Error,
            "ltspice_unsupported_rotation",
            &format!(
                "LTspice symbol '{}' uses unsupported rotation '{}'",
                symbol.name, symbol.rotation
            ),
            "Use R0, R90, R180, R270, M0, M90, M180, or M270 before importing.",
        ));
        return None;
    }

    let mut nodes = Vec::new();
    for pin in pins {
        let Some(node) = graph.node_name(pin) else {
            diagnostics.push(import_diagnostic(
                symbol.line,
                ImportSeverity::Error,
                "ltspice_unmapped_pin",
                &format!(
                    "LTspice symbol '{}' has a pin at {},{} that is not in the net graph",
                    symbol.name, pin.x, pin.y
                ),
                "Connect the pin to a wire or label before importing.",
            ));
            return None;
        };
        nodes.push(node.to_string());
    }

    let instance = symbol_instance_name(symbol, &spec, diagnostics);
    let value = symbol_value(symbol, &spec, diagnostics)?;
    Some(format!("{} {} {}", instance, nodes.join(" "), value))
}

fn symbol_instance_name(
    symbol: &LtspiceSymbol,
    spec: &LtspiceSymbolSpec,
    diagnostics: &mut Vec<ImportDiagnostic>,
) -> String {
    let raw_name = symbol
        .attrs
        .get("InstName")
        .filter(|name| !name.trim().is_empty())
        .cloned()
        .unwrap_or_else(|| {
            diagnostics.push(import_diagnostic(
                symbol.line,
                ImportSeverity::Warning,
                "ltspice_missing_instance_name",
                &format!("LTspice symbol '{}' has no InstName", symbol.name),
                "Assign a stable reference designator before importing.",
            ));
            format!("{}{}", spec.prefix, symbol.line)
        });
    normalize_instance_prefix(&raw_name, &spec.prefix)
}

fn symbol_value(
    symbol: &LtspiceSymbol,
    spec: &LtspiceSymbolSpec,
    diagnostics: &mut Vec<ImportDiagnostic>,
) -> Option<String> {
    let mut parts = Vec::new();
    if let Some(value) = symbol
        .attrs
        .get("Value")
        .filter(|value| !value.trim().is_empty())
    {
        parts.push(value.trim().to_string());
    }
    for key in ["Value2", "SpiceLine", "SpiceLine2"] {
        if let Some(value) = symbol
            .attrs
            .get(key)
            .filter(|value| !value.trim().is_empty())
        {
            parts.push(value.trim().to_string());
        }
    }
    if parts.is_empty() && spec.source == LtspiceSymbolSpecSource::AsyFile {
        parts.push(ltspice_symbol_basename(&symbol.name));
    }
    if parts.is_empty() {
        diagnostics.push(import_diagnostic(
            symbol.line,
            ImportSeverity::Error,
            "ltspice_missing_value",
            &format!("LTspice symbol '{}' has no Value attribute", symbol.name),
            &format!(
                "Add a value compatible with {} before importing.",
                spec.prefix
            ),
        ));
        None
    } else {
        Some(parts.join(" "))
    }
}

// LTspice built-in symbol table.
include!("ltspice_builtins_impl.rs");

fn normalize_instance_prefix(instance: &str, prefix: &str) -> String {
    let instance = instance.trim();
    let prefix = prefix.trim();
    if instance.is_empty() || prefix.is_empty() {
        return instance.to_string();
    }
    let first = instance.chars().next().unwrap_or_default();
    let prefix_first = prefix.chars().next().unwrap_or_default();
    if first.eq_ignore_ascii_case(&prefix_first) {
        instance.to_string()
    } else {
        format!("{prefix}{instance}")
    }
}

fn ltspice_symbol_basename(name: &str) -> String {
    name.replace('\\', "/")
        .split('/')
        .next_back()
        .unwrap_or(name)
        .to_ascii_lowercase()
}

fn transform_ltspice_point(point: AscPoint, origin: AscPoint, rotation: &str) -> Option<AscPoint> {
    let (x, y) = match rotation {
        "R0" => (point.x, point.y),
        "R90" => (-point.y, point.x),
        "R180" => (-point.x, -point.y),
        "R270" => (point.y, -point.x),
        "M0" => (-point.x, point.y),
        "M90" => (-point.y, -point.x),
        "M180" => (point.x, -point.y),
        "M270" => (point.y, point.x),
        _ => return None,
    };
    Some(AscPoint::new(origin.x + x, origin.y + y))
}

fn preferred_label(labels: &BTreeSet<String>) -> Option<String> {
    labels
        .iter()
        .find(|label| is_ground_node(&label.to_ascii_lowercase()))
        .cloned()
        .or_else(|| labels.iter().next().cloned())
}

fn wire_contains_point(wire: &LtspiceWire, point: AscPoint) -> bool {
    if wire.start.x == wire.end.x {
        point.x == wire.start.x && between_inclusive(point.y, wire.start.y, wire.end.y)
    } else if wire.start.y == wire.end.y {
        point.y == wire.start.y && between_inclusive(point.x, wire.start.x, wire.end.x)
    } else {
        false
    }
}

fn wire_intersection(left: &LtspiceWire, right: &LtspiceWire) -> Option<AscPoint> {
    if left.start.x == left.end.x && right.start.y == right.end.y {
        let point = AscPoint::new(left.start.x, right.start.y);
        return (wire_contains_point(left, point) && wire_contains_point(right, point))
            .then_some(point);
    }
    if left.start.y == left.end.y && right.start.x == right.end.x {
        let point = AscPoint::new(right.start.x, left.start.y);
        return (wire_contains_point(left, point) && wire_contains_point(right, point))
            .then_some(point);
    }
    None
}

fn between_inclusive(value: i32, left: i32, right: i32) -> bool {
    let min = left.min(right);
    let max = left.max(right);
    value >= min && value <= max
}

fn import_diagnostic(
    line: usize,
    severity: ImportSeverity,
    code: &str,
    message: &str,
    suggestion: &str,
) -> ImportDiagnostic {
    ImportDiagnostic {
        line,
        severity,
        code: code.to_string(),
        message: message.to_string(),
        suggestion: suggestion.to_string(),
    }
}
