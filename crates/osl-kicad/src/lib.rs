use osl_core::{OslError, OslResult, json_escape, read_text};
use std::path::Path;

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
    pub fn spice_directives(&self) -> Vec<&KicadTextItem> {
        self.text_items
            .iter()
            .filter(|item| item.text.trim_start().starts_with('.'))
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
                "  \"spice_directive_count\": {}\n",
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
            self.spice_directives().len()
        )
    }
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
                "  \"symbol_count\": {}\n",
                "}}"
            ),
            json_escape(&self.source),
            json_option(self.version.as_deref()),
            json_option(self.generator.as_deref()),
            self.symbols.len()
        )
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
    pub pins: Vec<KicadPinDef>,
}

impl KicadSymbolDef {
    pub fn property(&self, name: &str) -> Option<&str> {
        self.properties
            .iter()
            .find(|property| property.name == name)
            .map(|property| property.value.as_str())
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
        pins: collect_pin_defs(node),
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

#[cfg(test)]
mod tests {
    use super::{
        KicadLabelKind, parse_kicad_schematic, parse_kicad_symbol_library, parse_sexpr,
        read_kicad_schematic, read_kicad_symbol_library,
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
        assert_eq!(schematic.wires.len(), 3);
        assert_eq!(schematic.labels.len(), 2);
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
        assert_eq!(resistor.pins.len(), 2);
        assert_eq!(resistor.pins[0].number, "1");
        assert_eq!(resistor.pins[0].electrical_type, "passive");
        assert!(library.to_summary_json().contains("\"symbol_count\": 3"));
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
    }
}
