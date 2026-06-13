//! KiCad visual style — fill, stroke, margins, colors, and text effects.

use crate::sexpr::{
    Sexp, atom_text, child, child_value, format_number, head, list_items, list_value,
    sexpr_atom_or_string, sexpr_string,
};
use crate::util::parse_kicad_bool_value;
use crate::{KicadSize, parse_size};

#[derive(Debug, Clone, PartialEq)]
pub struct KicadStroke {
    pub width: Option<f64>,
    pub stroke_type: Option<String>,
    pub color: Option<KicadColor>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadFill {
    pub fill_type: Option<String>,
    pub color: Option<KicadColor>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadTextEffects {
    pub font_size: Option<KicadSize>,
    pub font_thickness: Option<f64>,
    pub font_bold: Option<bool>,
    pub font_italic: Option<bool>,
    pub font_color: Option<KicadColor>,
    pub justify: Vec<String>,
    pub hide: bool,
    pub href: Option<String>,
}

impl KicadTextEffects {
    /// write effects sexpr。
    pub(crate) fn write_effects_sexpr(&self, output: &mut String, indent: usize) {
        let pad = " ".repeat(indent);
        output.push_str(&format!("{}(effects", pad));
        self.write_font_sexpr(output);
        self.write_effect_tail(output);
        output.push_str(")\n");
    }

    /// write inline effects sexpr。
    pub(crate) fn write_inline_effects_sexpr(&self, output: &mut String) {
        output.push_str(" (effects");
        self.write_font_sexpr(output);
        self.write_effect_tail(output);
        output.push(')');
    }

    fn write_font_sexpr(&self, output: &mut String) {
        output.push_str(" (font");
        if let Some(size) = self.font_size {
            output.push_str(&format!(
                " (size {} {})",
                format_number(size.width),
                format_number(size.height)
            ));
        } else {
            output.push_str(" (size 1.27 1.27)");
        }
        if let Some(thickness) = self.font_thickness {
            output.push_str(&format!(" (thickness {})", format_number(thickness)));
        }
        write_inline_optional_bool_sexpr(output, "bold", self.font_bold);
        write_inline_optional_bool_sexpr(output, "italic", self.font_italic);
        if let Some(color) = self.font_color {
            color.write_inline_color_sexpr(output);
        }
        output.push(')');
    }

    fn write_effect_tail(&self, output: &mut String) {
        if !self.justify.is_empty() {
            output.push_str(" (justify");
            for token in &self.justify {
                output.push_str(&format!(" {}", sexpr_atom_or_string(token)));
            }
            output.push(')');
        }
        if self.hide {
            output.push_str(" hide");
        }
        if let Some(href) = &self.href {
            output.push_str(&format!(" (href {})", sexpr_string(href)));
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KicadColor {
    pub red: f64,
    pub green: f64,
    pub blue: f64,
    pub alpha: f64,
}

impl KicadColor {
    /// write inline color sexpr。
    pub(crate) fn write_inline_color_sexpr(self, output: &mut String) {
        output.push_str(&format!(
            " (color {} {} {} {})",
            format_number(self.red),
            format_number(self.green),
            format_number(self.blue),
            format_number(self.alpha)
        ));
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KicadMargins {
    pub left: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
}

/// default kicad text effects。
pub(crate) fn default_kicad_text_effects() -> KicadTextEffects {
    KicadTextEffects {
        font_size: Some(KicadSize {
            width: 1.27,
            height: 1.27,
        }),
        font_thickness: None,
        font_bold: None,
        font_italic: None,
        font_color: None,
        justify: Vec::new(),
        hide: false,
        href: None,
    }
}

/// parse text effects。
pub(crate) fn parse_text_effects(node: &Sexp) -> KicadTextEffects {
    let items = list_items(node);
    let font = child(items, "font");
    let font_items = font.map(list_items).unwrap_or_default();
    KicadTextEffects {
        font_size: child(font_items, "size").and_then(parse_size),
        font_thickness: child_value(font_items, "thickness").and_then(|value| value.parse().ok()),
        font_bold: parse_effect_bool(font_items, "bold"),
        font_italic: parse_effect_bool(font_items, "italic"),
        font_color: child(font_items, "color").and_then(parse_color),
        justify: child(items, "justify")
            .map(|justify| {
                list_items(justify)
                    .iter()
                    .skip(1)
                    .filter_map(atom_text)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default(),
        hide: has_effect_flag(items, "hide"),
        href: child_value(items, "href"),
    }
}

fn parse_effect_bool(items: &[Sexp], name: &str) -> Option<bool> {
    child(items, name)
        .and_then(|node| list_value(node, 1).and_then(parse_kicad_bool_value))
        .or_else(|| has_effect_flag(items, name).then_some(true))
}

fn has_effect_flag(items: &[Sexp], name: &str) -> bool {
    items
        .iter()
        .skip(1)
        .any(|item| atom_text(item) == Some(name) || head(item) == Some(name))
}

/// parse color。
pub(crate) fn parse_color(node: &Sexp) -> Option<KicadColor> {
    let items = list_items(node);
    Some(KicadColor {
        red: atom_text(items.get(1)?)?.parse().ok()?,
        green: atom_text(items.get(2)?)?.parse().ok()?,
        blue: atom_text(items.get(3)?)?.parse().ok()?,
        alpha: atom_text(items.get(4)?)?.parse().ok()?,
    })
}

/// parse stroke。
pub(crate) fn parse_stroke(node: &Sexp) -> KicadStroke {
    let items = list_items(node);
    KicadStroke {
        width: child_value(items, "width").and_then(|value| value.parse().ok()),
        stroke_type: child_value(items, "type"),
        color: child(items, "color").and_then(parse_color),
    }
}

/// parse fill。
pub(crate) fn parse_fill(node: &Sexp) -> KicadFill {
    let items = list_items(node);
    KicadFill {
        fill_type: child_value(items, "type"),
        color: child(items, "color").and_then(parse_color),
    }
}

/// parse margins。
pub(crate) fn parse_margins(node: &Sexp) -> Option<KicadMargins> {
    let items = list_items(node);
    Some(KicadMargins {
        left: atom_text(items.get(1)?)?.parse().ok()?,
        top: atom_text(items.get(2)?)?.parse().ok()?,
        right: atom_text(items.get(3)?)?.parse().ok()?,
        bottom: atom_text(items.get(4)?)?.parse().ok()?,
    })
}

/// write optional bool sexpr。
pub(crate) fn write_optional_bool_sexpr(
    output: &mut String,
    indent: usize,
    name: &str,
    value: Option<bool>,
) {
    if let Some(value) = value {
        let pad = " ".repeat(indent);
        output.push_str(&format!(
            "{}({} {})\n",
            pad,
            name,
            if value { "yes" } else { "no" }
        ));
    }
}

/// write inline optional bool sexpr。
pub(crate) fn write_inline_optional_bool_sexpr(
    output: &mut String,
    name: &str,
    value: Option<bool>,
) {
    if let Some(value) = value {
        output.push_str(&format!(" ({} {})", name, if value { "yes" } else { "no" }));
    }
}

/// write inline text effects。
pub(crate) fn write_inline_text_effects(output: &mut String, effects: Option<&KicadTextEffects>) {
    match effects {
        Some(effects) => effects.write_inline_effects_sexpr(output),
        None => output.push_str(" (effects (font (size 1.27 1.27)))"),
    }
}

/// write text effects line。
pub(crate) fn write_text_effects_line(
    output: &mut String,
    indent: usize,
    effects: Option<&KicadTextEffects>,
) {
    match effects {
        Some(effects) => effects.write_effects_sexpr(output, indent),
        None => {
            let pad = " ".repeat(indent);
            output.push_str(&format!("{pad}(effects (font (size 1.27 1.27)))\n"));
        }
    }
}

/// write inline stroke。
pub(crate) fn write_inline_stroke(
    output: &mut String,
    stroke: Option<&KicadStroke>,
    default_width: f64,
) {
    match stroke {
        Some(stroke) => {
            output.push_str(" (stroke");
            output.push_str(&format!(
                " (width {})",
                format_number(stroke.width.unwrap_or(default_width))
            ));
            if let Some(stroke_type) = &stroke.stroke_type {
                output.push_str(&format!(" (type {})", sexpr_atom_or_string(stroke_type)));
            } else {
                output.push_str(" (type default)");
            }
            if let Some(color) = stroke.color {
                color.write_inline_color_sexpr(output);
            }
            output.push(')');
        }
        None => output.push_str(&format!(
            " (stroke (width {}) (type default))",
            format_number(default_width)
        )),
    }
}

/// write inline optional fill。
pub(crate) fn write_inline_optional_fill(output: &mut String, fill: Option<&KicadFill>) {
    if let Some(fill) = fill {
        write_inline_fill(output, Some(fill));
    }
}

/// write inline fill。
pub(crate) fn write_inline_fill(output: &mut String, fill: Option<&KicadFill>) {
    match fill {
        Some(fill) => {
            output.push_str(" (fill");
            if let Some(fill_type) = &fill.fill_type {
                output.push_str(&format!(" (type {})", sexpr_atom_or_string(fill_type)));
            } else if fill.color.is_none() {
                output.push_str(" (type none)");
            }
            if let Some(color) = fill.color {
                color.write_inline_color_sexpr(output);
            }
            output.push(')');
        }
        None => output.push_str(" (fill (type none))"),
    }
}

/// kicad margins value。
pub(crate) fn kicad_margins_value(margins: KicadMargins) -> serde_json::Value {
    serde_json::json!({
        "left": margins.left,
        "top": margins.top,
        "right": margins.right,
        "bottom": margins.bottom,
    })
}

/// kicad color value。
pub(crate) fn kicad_color_value(color: KicadColor) -> serde_json::Value {
    serde_json::json!({
        "red": color.red,
        "green": color.green,
        "blue": color.blue,
        "alpha": color.alpha,
    })
}

/// kicad stroke value。
pub(crate) fn kicad_stroke_value(stroke: &KicadStroke) -> serde_json::Value {
    serde_json::json!({
        "width": stroke.width,
        "type": stroke.stroke_type,
        "color": stroke.color.map(kicad_color_value),
    })
}

/// kicad fill value。
pub(crate) fn kicad_fill_value(fill: &KicadFill) -> serde_json::Value {
    serde_json::json!({
        "type": fill.fill_type,
        "color": fill.color.map(kicad_color_value),
    })
}

/// kicad text effects value。
pub(crate) fn kicad_text_effects_value(effects: &KicadTextEffects) -> serde_json::Value {
    serde_json::json!({
        "font_size": effects.font_size.map(crate::kicad_size_value),
        "font_thickness": effects.font_thickness,
        "font_bold": effects.font_bold,
        "font_italic": effects.font_italic,
        "font_color": effects.font_color.map(kicad_color_value),
        "justify": effects.justify,
        "hide": effects.hide,
        "href": effects.href,
    })
}
