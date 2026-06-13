//! KiCad property parsing and serialization for symbols.

use crate::coordinates::{KicadAt, parse_at};
use crate::sexpr::{Sexp, child, child_value, format_number, list_items, list_value, sexpr_string};
use crate::style::{KicadTextEffects, parse_text_effects, write_optional_bool_sexpr};
use crate::util::parse_kicad_bool_value;

#[derive(Debug, Clone, PartialEq)]
pub struct KicadProperty {
    pub name: String,
    pub value: String,
    pub id: Option<u32>,
    pub at: Option<KicadAt>,
    pub hide: Option<bool>,
    pub show_name: Option<bool>,
    pub do_not_autoplace: Option<bool>,
    pub effects: Option<KicadTextEffects>,
}

impl KicadProperty {
    /// write property sexpr。
    pub(crate) fn write_property_sexpr(&self, output: &mut String, indent: usize) {
        let pad = " ".repeat(indent);
        output.push_str(&format!(
            "{}(property {} {}",
            pad,
            sexpr_string(&self.name),
            sexpr_string(&self.value)
        ));
        if let Some(id) = self.id {
            output.push_str(&format!(" (id {})", id));
        }
        if let Some(at) = self.at {
            output.push_str(&format!(
                " (at {} {} {})",
                format_number(at.x),
                format_number(at.y),
                format_number(at.rotation)
            ));
        }
        output.push('\n');
        write_optional_bool_sexpr(output, indent + 2, "hide", self.hide);
        write_optional_bool_sexpr(output, indent + 2, "show_name", self.show_name);
        write_optional_bool_sexpr(
            output,
            indent + 2,
            "do_not_autoplace",
            self.do_not_autoplace,
        );
        match &self.effects {
            Some(effects) => effects.write_effects_sexpr(output, indent + 2),
            None => output.push_str(&format!("{}  (effects (font (size 1.27 1.27)))\n", pad)),
        }
        output.push_str(&format!("{})\n", pad));
    }
}

/// parse property。
pub(crate) fn parse_property(node: &Sexp) -> Option<KicadProperty> {
    let items = list_items(node);
    Some(KicadProperty {
        name: list_value(node, 1)?,
        value: list_value(node, 2)?,
        id: child_value(items, "id").and_then(|value| value.parse().ok()),
        at: child(items, "at").and_then(parse_at),
        hide: child_value(items, "hide").and_then(parse_kicad_bool_value),
        show_name: child_value(items, "show_name").and_then(parse_kicad_bool_value),
        do_not_autoplace: child_value(items, "do_not_autoplace").and_then(parse_kicad_bool_value),
        effects: child(items, "effects").map(parse_text_effects),
    })
}
