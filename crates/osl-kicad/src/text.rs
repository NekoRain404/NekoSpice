use crate::coordinates::{KicadAt, KicadSize, parse_at, parse_size};
use crate::geometry::{KicadBoundingBox, kicad_rotated_rect_bounds};
use crate::sexpr::{Sexp, child, child_value, format_number, list_items, list_value, sexpr_string};
use crate::style::{
    KicadFill, KicadMargins, KicadStroke, KicadTextEffects, parse_fill, parse_margins,
    parse_stroke, parse_text_effects, write_inline_fill, write_inline_stroke,
    write_inline_text_effects, write_text_effects_line,
};
use crate::util::{parse_kicad_bool_value, parse_optional_bool_child};

#[derive(Debug, Clone, PartialEq)]
pub struct KicadTextItem {
    pub text: String,
    pub at: Option<KicadAt>,
    pub uuid: Option<String>,
    pub effects: Option<KicadTextEffects>,
}

impl KicadTextItem {
    pub(crate) fn write_text_sexpr(&self, output: &mut String, indent: usize) {
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
        write_inline_text_effects(output, self.effects.as_ref());
        if let Some(uuid) = &self.uuid {
            output.push_str(&format!(" (uuid {})", sexpr_string(uuid)));
        }
        output.push_str(")\n");
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadTextBox {
    pub text: String,
    pub at: Option<KicadAt>,
    pub size: Option<KicadSize>,
    pub margins: Option<KicadMargins>,
    pub stroke: Option<KicadStroke>,
    pub fill: Option<KicadFill>,
    pub exclude_from_sim: Option<bool>,
    pub uuid: Option<String>,
    pub locked: Option<bool>,
    pub effects: Option<KicadTextEffects>,
}

impl KicadTextBox {
    pub fn bounding_box(&self) -> Option<KicadBoundingBox> {
        kicad_rotated_rect_bounds(self.at?, self.size?)
    }

    pub(crate) fn write_text_box_sexpr(&self, output: &mut String, indent: usize) {
        let pad = " ".repeat(indent);
        output.push_str(&format!("{}(text_box {}\n", pad, sexpr_string(&self.text)));
        if let Some(exclude_from_sim) = self.exclude_from_sim {
            output.push_str(&format!(
                "{}  (exclude_from_sim {})\n",
                pad,
                if exclude_from_sim { "yes" } else { "no" }
            ));
        }
        if let Some(at) = self.at {
            output.push_str(&format!(
                "{}  (at {} {} {})\n",
                pad,
                format_number(at.x),
                format_number(at.y),
                format_number(at.rotation)
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
        if let Some(margins) = self.margins {
            output.push_str(&format!(
                "{}  (margins {} {} {} {})\n",
                pad,
                format_number(margins.left),
                format_number(margins.top),
                format_number(margins.right),
                format_number(margins.bottom)
            ));
        }
        output.push_str(&format!("{} ", pad));
        write_inline_stroke(output, self.stroke.as_ref(), 0.0);
        output.push('\n');
        output.push_str(&format!("{} ", pad));
        write_inline_fill(output, self.fill.as_ref());
        output.push('\n');
        write_text_effects_line(output, indent + 2, self.effects.as_ref());
        if let Some(uuid) = &self.uuid {
            output.push_str(&format!("{}  (uuid {})\n", pad, sexpr_string(uuid)));
        }
        if self.locked == Some(true) {
            output.push_str(&format!("{}  (locked yes)\n", pad));
        }
        output.push_str(&format!("{})\n", pad));
    }
}

pub(crate) fn parse_text_item(node: &Sexp) -> Option<KicadTextItem> {
    let items = list_items(node);
    Some(KicadTextItem {
        text: list_value(node, 1)?,
        at: child(items, "at").and_then(parse_at),
        uuid: child_value(items, "uuid"),
        effects: child(items, "effects").map(parse_text_effects),
    })
}

pub(crate) fn parse_text_box(node: &Sexp) -> Option<KicadTextBox> {
    let items = list_items(node);
    Some(KicadTextBox {
        text: list_value(node, 1)?,
        at: child(items, "at").and_then(parse_at),
        size: child(items, "size").and_then(parse_size),
        margins: child(items, "margins").and_then(parse_margins),
        stroke: child(items, "stroke").map(parse_stroke),
        fill: child(items, "fill").map(parse_fill),
        exclude_from_sim: child_value(items, "exclude_from_sim").and_then(parse_kicad_bool_value),
        uuid: child_value(items, "uuid"),
        locked: parse_optional_bool_child(items, "locked"),
        effects: child(items, "effects").map(parse_text_effects),
    })
}
