use crate::coordinates::{KicadAt, parse_at};
use crate::sexpr::{
    Sexp, atom_text, child, child_value, direct_children, format_number, list_items, list_value,
    sexpr_atom_or_string, sexpr_string,
};
use crate::style::{KicadTextEffects, parse_text_effects, write_optional_bool_sexpr};
use crate::util::parse_optional_bool_child;
use std::cmp::Ordering;

#[derive(Debug, Clone, PartialEq)]
pub struct KicadSymbolPinRef {
    pub number: Option<String>,
    pub uuid: Option<String>,
    pub alternate: Option<String>,
}

impl KicadSymbolPinRef {
    /// write pin ref sexpr。
    pub(crate) fn write_pin_ref_sexpr(&self, output: &mut String, indent: usize) {
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
        if let Some(alternate) = &self.alternate {
            output.push_str(&format!(" (alternate {})", sexpr_string(alternate)));
        }
        output.push_str(")\n");
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadPinDisplay {
    pub offset: Option<f64>,
    pub hide: Option<bool>,
}

impl KicadPinDisplay {
    /// write pin names sexpr。
    pub(crate) fn write_pin_names_sexpr(&self, output: &mut String, indent: usize) {
        self.write_pin_display_sexpr(output, indent, "pin_names", true);
    }

    /// write pin numbers sexpr。
    pub(crate) fn write_pin_numbers_sexpr(&self, output: &mut String, indent: usize) {
        self.write_pin_display_sexpr(output, indent, "pin_numbers", false);
    }

    fn write_pin_display_sexpr(
        &self,
        output: &mut String,
        indent: usize,
        name: &str,
        include_offset: bool,
    ) {
        let pad = " ".repeat(indent);
        output.push_str(&format!("{}({}\n", pad, name));
        if include_offset && let Some(offset) = self.offset {
            output.push_str(&format!("{}  (offset {})\n", pad, format_number(offset)));
        }
        write_optional_bool_sexpr(output, indent + 2, "hide", self.hide);
        output.push_str(&format!("{})\n", pad));
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadPinDef {
    pub number: KicadPinText,
    pub name: KicadPinText,
    pub electrical_type: String,
    pub shape: String,
    pub unit: u32,
    pub body_style: u32,
    pub at: Option<KicadAt>,
    pub length: Option<f64>,
    pub alternates: Vec<KicadPinAlternate>,
}

impl KicadPinDef {
    /// number。
    pub fn number(&self) -> &str {
        &self.number.text
    }

    /// name。
    pub fn name(&self) -> &str {
        &self.name.text
    }

    /// number effects。
    pub fn number_effects(&self) -> Option<&KicadTextEffects> {
        self.number.effects.as_ref()
    }

    /// name effects。
    pub fn name_effects(&self) -> Option<&KicadTextEffects> {
        self.name.effects.as_ref()
    }

    /// write pin sexpr。
    pub(crate) fn write_pin_sexpr(&self, output: &mut String, indent: usize) {
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
        self.name.write_inline_pin_text_sexpr(output, "name");
        self.number.write_inline_pin_text_sexpr(output, "number");
        for alternate in &self.alternates {
            alternate.write_inline_alternate_sexpr(output);
        }
        output.push_str(")\n");
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KicadPinAlternate {
    pub name: String,
    pub electrical_type: String,
    pub shape: String,
}

impl KicadPinAlternate {
    fn write_inline_alternate_sexpr(&self, output: &mut String) {
        output.push_str(&format!(
            " (alternate {} {} {})",
            sexpr_string(&self.name),
            sexpr_atom_or_string(&self.electrical_type),
            sexpr_atom_or_string(&self.shape)
        ));
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadPinText {
    pub text: String,
    pub effects: Option<KicadTextEffects>,
}

impl KicadPinText {
    fn new(text: String, effects: Option<KicadTextEffects>) -> Self {
        Self { text, effects }
    }

    fn write_inline_pin_text_sexpr(&self, output: &mut String, name: &str) {
        output.push_str(&format!(" ({} {}", name, sexpr_string(&self.text)));
        match &self.effects {
            Some(effects) => effects.write_inline_effects_sexpr(output),
            None => output.push_str(" (effects (font (size 1.27 1.27)))"),
        }
        output.push(')');
    }
}

/// parse symbol pin ref。
pub(crate) fn parse_symbol_pin_ref(node: &Sexp) -> Option<KicadSymbolPinRef> {
    let items = list_items(node);
    Some(KicadSymbolPinRef {
        number: list_value(node, 1),
        uuid: child_value(items, "uuid"),
        alternate: child_value(items, "alternate"),
    })
}

/// parse pin def。
pub(crate) fn parse_pin_def(node: &Sexp) -> Option<KicadPinDef> {
    let items = list_items(node);
    Some(KicadPinDef {
        number: child(items, "number").and_then(parse_pin_text)?,
        name: child(items, "name")
            .and_then(parse_pin_text)
            .unwrap_or_else(|| KicadPinText::new("~".to_string(), None)),
        electrical_type: list_value(node, 1).unwrap_or_else(|| "unspecified".to_string()),
        shape: list_value(node, 2).unwrap_or_else(|| "line".to_string()),
        unit: 0,
        body_style: 0,
        at: child(items, "at").and_then(parse_at),
        length: child_value(items, "length").and_then(|value| value.parse().ok()),
        alternates: direct_children(items, "alternate")
            .filter_map(parse_pin_alternate)
            .collect(),
    })
}

fn parse_pin_alternate(node: &Sexp) -> Option<KicadPinAlternate> {
    Some(KicadPinAlternate {
        name: list_value(node, 1)?,
        electrical_type: list_value(node, 2).unwrap_or_else(|| "unspecified".to_string()),
        shape: list_value(node, 3).unwrap_or_else(|| "line".to_string()),
    })
}

/// parse pin display。
pub(crate) fn parse_pin_display(node: &Sexp) -> KicadPinDisplay {
    let items = list_items(node);
    KicadPinDisplay {
        offset: child_value(items, "offset").and_then(|value| value.parse().ok()),
        hide: parse_optional_bool_child(items, "hide").or_else(|| {
            items
                .iter()
                .skip(1)
                .any(|item| atom_text(item) == Some("hide"))
                .then_some(true)
        }),
    }
}

fn parse_pin_text(node: &Sexp) -> Option<KicadPinText> {
    let items = list_items(node);
    Some(KicadPinText::new(
        list_value(node, 1)?,
        child(items, "effects").map(parse_text_effects),
    ))
}

/// kicad pin alternate value。
pub(crate) fn kicad_pin_alternate_value(alternate: &KicadPinAlternate) -> serde_json::Value {
    serde_json::json!({
        "name": alternate.name,
        "electrical_type": alternate.electrical_type,
        "shape": alternate.shape,
    })
}

/// kicad pin display value。
pub(crate) fn kicad_pin_display_value(display: &KicadPinDisplay) -> serde_json::Value {
    serde_json::json!({
        "offset": display.offset,
        "hide": display.hide,
    })
}

/// compare pin numbers。
pub(crate) fn compare_pin_numbers(left: &&KicadPinDef, right: &&KicadPinDef) -> Ordering {
    match (left.number().parse::<u32>(), right.number().parse::<u32>()) {
        (Ok(left), Ok(right)) => left.cmp(&right),
        _ => left.number().cmp(right.number()),
    }
}
