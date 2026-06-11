// Symbol parsing helpers and S-expression deserialization.
// Covers: KicadSymbolPower, KicadSymbolBodyStyles,
// parse_symbol_mirror, collect_symbol_unit_names, etc.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KicadSymbolPower {
    Bare,
    Global,
    Local,
}

impl KicadSymbolPower {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Bare => "bare",
            Self::Global => "global",
            Self::Local => "local",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KicadSymbolBodyStyles {
    Demorgan,
    Names(Vec<String>),
}

impl KicadSymbolBodyStyles {
    fn body_style_numbers(&self) -> Vec<u32> {
        match self {
            Self::Demorgan => vec![1, 2],
            Self::Names(names) => (1..=names.len() as u32).collect(),
        }
    }

    fn write_body_styles_sexpr(&self, output: &mut String, indent: usize) {
        let pad = " ".repeat(indent);
        output.push_str(&format!("{}(body_styles", pad));
        match self {
            Self::Demorgan => output.push_str(" demorgan"),
            Self::Names(names) => {
                for name in names {
                    output.push(' ');
                    output.push_str(&sexpr_atom_or_string(name));
                }
            }
        }
        output.push_str(")\n");
    }
}

pub(crate) fn parse_symbol_instance(node: &Sexp) -> Option<KicadSymbolInstance> {
    let items = list_items(node);
    Some(KicadSymbolInstance {
        lib_id: child_value(items, "lib_id")?,
        at: child(items, "at").and_then(parse_at),
        mirror: child(items, "mirror").and_then(parse_symbol_mirror),
        unit: child_value(items, "unit").and_then(|value| value.parse().ok()),
        body_style: child_value(items, "body_style")
            .or_else(|| child_value(items, "convert"))
            .and_then(|value| value.parse().ok()),
        uuid: child_value(items, "uuid"),
        exclude_from_sim: child_value(items, "exclude_from_sim").and_then(parse_kicad_bool_value),
        in_bom: child_value(items, "in_bom").and_then(parse_kicad_bool_value),
        on_board: child_value(items, "on_board").and_then(parse_kicad_bool_value),
        dnp: child_value(items, "dnp").and_then(parse_kicad_bool_value),
        fields_autoplaced: parse_optional_bool_child(items, "fields_autoplaced"),
        properties: direct_children(items, "property")
            .filter_map(parse_property)
            .collect(),
        pins: direct_children(items, "pin")
            .filter_map(parse_symbol_pin_ref)
            .collect(),
        instances: child(items, "instances")
            .map(parse_project_instances)
            .unwrap_or_default(),
    })
}

fn parse_symbol_mirror(node: &Sexp) -> Option<String> {
    let mirror = list_items(node)
        .iter()
        .skip(1)
        .filter_map(atom_text)
        .collect::<Vec<_>>()
        .join(" ");
    normalize_symbol_mirror(&mirror).ok().flatten()
}

pub(crate) fn parse_symbol_def(node: &Sexp) -> Option<KicadSymbolDef> {
    let items = list_items(node);
    Some(KicadSymbolDef {
        name: list_value(node, 1)?,
        extends: child_value(items, "extends"),
        power: child(items, "power").map(parse_symbol_power),
        body_styles: child(items, "body_styles").and_then(parse_symbol_body_styles),
        exclude_from_sim: child_value(items, "exclude_from_sim").and_then(parse_kicad_bool_value),
        in_bom: child_value(items, "in_bom").and_then(parse_kicad_bool_value),
        on_board: child_value(items, "on_board").and_then(parse_kicad_bool_value),
        in_pos_files: child_value(items, "in_pos_files").and_then(parse_kicad_bool_value),
        duplicate_pin_numbers_are_jumpers: child_value(items, "duplicate_pin_numbers_are_jumpers")
            .and_then(parse_kicad_bool_value),
        jumper_pin_groups: child(items, "jumper_pin_groups")
            .map(parse_jumper_pin_groups)
            .unwrap_or_default(),
        embedded_fonts: child_value(items, "embedded_fonts").and_then(parse_kicad_bool_value),
        pin_names: child(items, "pin_names").map(parse_pin_display),
        pin_numbers: child(items, "pin_numbers").map(parse_pin_display),
        unit_names: collect_symbol_unit_names(node),
        properties: direct_children(items, "property")
            .filter_map(parse_property)
            .collect(),
        graphics: collect_graphics(node),
        pins: collect_pin_defs(node),
    })
}

fn collect_symbol_unit_names(node: &Sexp) -> BTreeMap<u32, String> {
    let mut unit_names = BTreeMap::new();
    collect_symbol_unit_names_into(node, &mut unit_names);
    unit_names
}

fn collect_symbol_unit_names_into(node: &Sexp, unit_names: &mut BTreeMap<u32, String>) {
    if let Some(scope) = child_symbol_item_scope(node)
        && scope.unit != 0
        && let Some(unit_name) = child_value(list_items(node), "unit_name")
    {
        unit_names.insert(scope.unit, unit_name);
    }
    for child in list_items(node) {
        if matches!(child, Sexp::List(_)) {
            collect_symbol_unit_names_into(child, unit_names);
        }
    }
}

fn parse_symbol_power(node: &Sexp) -> KicadSymbolPower {
    match list_value(node, 1)
        .as_deref()
        .map(str::trim)
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("global") => KicadSymbolPower::Global,
        Some("local") => KicadSymbolPower::Local,
        _ => KicadSymbolPower::Bare,
    }
}

fn parse_symbol_body_styles(node: &Sexp) -> Option<KicadSymbolBodyStyles> {
    let names = list_items(node)
        .iter()
        .skip(1)
        .filter_map(atom_text)
        .map(str::to_string)
        .collect::<Vec<_>>();
    if names.iter().any(|name| name == "demorgan") {
        Some(KicadSymbolBodyStyles::Demorgan)
    } else if names.is_empty() {
        None
    } else {
        Some(KicadSymbolBodyStyles::Names(names))
    }
}

fn parse_jumper_pin_groups(node: &Sexp) -> Vec<Vec<String>> {
    list_items(node)
        .iter()
        .skip(1)
        .filter_map(|group| {
            let pins = list_items(group)
                .iter()
                .filter_map(atom_text)
                .map(str::to_string)
                .collect::<Vec<_>>();
            (!pins.is_empty()).then_some(pins)
        })
        .collect()
}

fn collect_pin_defs(node: &Sexp) -> Vec<KicadPinDef> {
    let mut pins = Vec::new();
    collect_pin_defs_into(node, KicadSymbolItemScope::default(), &mut pins);
    pins
}

fn collect_pin_defs_into(node: &Sexp, scope: KicadSymbolItemScope, pins: &mut Vec<KicadPinDef>) {
    if head(node) == Some("pin")
        && let Some(mut pin) = parse_pin_def(node)
    {
        pin.unit = scope.unit;
        pin.body_style = scope.body_style;
        pins.push(pin);
    }
    for child in list_items(node) {
        if matches!(child, Sexp::List(_)) {
            let child_scope = child_symbol_item_scope(child).unwrap_or(scope);
            collect_pin_defs_into(child, child_scope, pins);
        }
    }
}

fn collect_graphics(node: &Sexp) -> Vec<KicadSymbolGraphic> {
    let mut graphics = Vec::new();
    collect_graphics_into(node, KicadSymbolItemScope::default(), &mut graphics);
    graphics
}

fn collect_graphics_into(
    node: &Sexp,
    scope: KicadSymbolItemScope,
    graphics: &mut Vec<KicadSymbolGraphic>,
) {
    if let Some(graphic) = parse_symbol_graphic(node) {
        graphics.push(KicadSymbolGraphic {
            unit: scope.unit,
            body_style: scope.body_style,
            ..graphic
        });
    }
    for child in list_items(node) {
        if matches!(child, Sexp::List(_)) {
            let child_scope = child_symbol_item_scope(child).unwrap_or(scope);
            collect_graphics_into(child, child_scope, graphics);
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
struct KicadSymbolItemScope {
    unit: u32,
    body_style: u32,
}

fn child_symbol_item_scope(node: &Sexp) -> Option<KicadSymbolItemScope> {
    if head(node) != Some("symbol") {
        return None;
    }
    parse_symbol_item_scope(list_value(node, 1)?.as_str())
}

fn parse_symbol_item_scope(name: &str) -> Option<KicadSymbolItemScope> {
    let (_, body_style) = name.rsplit_once('_')?;
    let (base, unit) = name[..name.len() - body_style.len() - 1].rsplit_once('_')?;
    if base.is_empty() {
        return None;
    }
    Some(KicadSymbolItemScope {
        unit: unit.parse().ok()?,
        body_style: body_style.parse().ok()?,
    })
}

pub(crate) fn library_symbol_definition_for_lib_id(
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
            qualify_library_symbol_name(&mut symbol, library_name);
            symbol
        })
}

pub(crate) fn qualify_library_symbol_name(symbol: &mut KicadSymbolDef, library_name: &str) {
    if !symbol.name.contains(':') {
        symbol.name = format!("{library_name}:{}", symbol.name);
    }
}

pub(crate) fn symbol_ordered_pins<'a>(
    symbol: &'a KicadSymbolInstance,
    definition: &'a KicadResolvedSymbolDef,
) -> Vec<&'a KicadPinDef> {
    let scoped_pins = definition
        .scoped_pins(symbol.unit, symbol.body_style)
        .collect::<Vec<_>>();
    let mut by_number = scoped_pins
        .iter()
        .copied()
        .map(|pin| (pin.number(), pin))
        .collect::<BTreeMap<_, _>>();
    let by_name = scoped_pins
        .iter()
        .copied()
        .map(|pin| (pin.name(), pin))
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
        ordered = scoped_pins;
        ordered.sort_by(compare_pin_numbers);
    }

    ordered
}

fn scoped_symbol_pins<'a>(
    definition: &'a KicadSymbolDef,
    unit: Option<u32>,
    body_style: Option<u32>,
) -> impl Iterator<Item = &'a KicadPinDef> + 'a {
    let unit = unit.unwrap_or(1);
    let body_style = body_style.unwrap_or(1);
    definition
        .pins
        .iter()
        .filter(move |pin| symbol_item_scope_matches(pin.unit, pin.body_style, unit, body_style))
}

fn scoped_definition_graphics<'a>(
    definition: &'a KicadSymbolDef,
    unit: Option<u32>,
    body_style: Option<u32>,
) -> impl Iterator<Item = &'a KicadSymbolGraphic> + 'a {
    let unit = unit.unwrap_or(1);
    let body_style = body_style.unwrap_or(1);
    definition.graphics.iter().filter(move |graphic| {
        symbol_item_scope_matches(graphic.unit, graphic.body_style, unit, body_style)
    })
}

fn scoped_symbol_items<'a, T>(
    items: &'a [T],
    unit: Option<u32>,
    body_style: Option<u32>,
    scope: impl Fn(&T) -> (u32, u32) + 'a,
) -> impl Iterator<Item = &'a T> + 'a {
    let unit = unit.unwrap_or(1);
    let body_style = body_style.unwrap_or(1);
    items.iter().filter(move |item| {
        let (item_unit, item_body_style) = scope(item);
        symbol_item_scope_matches(item_unit, item_body_style, unit, body_style)
    })
}

pub(crate) fn symbol_item_scope_matches(
    item_unit: u32,
    item_body_style: u32,
    selected_unit: u32,
    selected_body_style: u32,
) -> bool {
    (item_unit == 0 || item_unit == selected_unit)
        && (item_body_style == 0 || item_body_style == selected_body_style)
}

pub(crate) fn symbol_sim_pin_order(
    symbol: &KicadSymbolInstance,
    definition: &KicadResolvedSymbolDef,
) -> Vec<String> {
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

fn parse_kicad_enable_value(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "y" | "yes" | "true" | "1" | "on" => Some(true),
        "n" | "no" | "false" | "0" | "off" => Some(false),
        _ => None,
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

fn unquote_spice_token(value: &str) -> &str {
    value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .unwrap_or(value)
}

pub(crate) fn symbol_instance_properties(
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
            id: property.id,
            at: property
                .at
                .map(|property_at| transform_local_at(property_at, symbol_at, None)),
            hide: property.hide,
            show_name: property.show_name,
            do_not_autoplace: property.do_not_autoplace,
            effects: property.effects.clone(),
        })
        .collect::<Vec<_>>();

    if !properties
        .iter()
        .any(|property| property.name == "Reference")
    {
        properties.push(KicadProperty {
            name: "Reference".to_string(),
            value: reference.to_string(),
            id: None,
            at: Some(KicadAt {
                x: symbol_at.x,
                y: symbol_at.y - 2.54,
                rotation: symbol_at.rotation,
            }),
            hide: None,
            show_name: None,
            do_not_autoplace: None,
            effects: None,
        });
    }
    if !properties.iter().any(|property| property.name == "Value") {
        properties.push(KicadProperty {
            name: "Value".to_string(),
            value: value.to_string(),
            id: None,
            at: Some(KicadAt {
                x: symbol_at.x,
                y: symbol_at.y + 2.54,
                rotation: symbol_at.rotation,
            }),
            hide: None,
            show_name: None,
            do_not_autoplace: None,
            effects: None,
        });
    }

    properties
}
