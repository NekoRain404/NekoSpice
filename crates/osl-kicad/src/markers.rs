//! Schematic markers — junctions and no-connect markers.

use crate::coordinates::{KicadPoint, parse_at};
use crate::sexpr::{Sexp, child, child_value, format_number, list_items, sexpr_string};
use crate::style::{KicadColor, parse_color};

#[derive(Debug, Clone, PartialEq)]
pub struct KicadJunction {
    pub at: KicadPoint,
    pub diameter: Option<f64>,
    pub color: Option<KicadColor>,
    pub uuid: Option<String>,
}

impl KicadJunction {
    /// write junction sexpr。
    pub(crate) fn write_junction_sexpr(&self, output: &mut String, indent: usize) {
        let pad = " ".repeat(indent);
        output.push_str(&format!(
            "{}(junction\n{}  (at {} {})\n{}  (diameter {})\n{} ",
            pad,
            pad,
            format_number(self.at.x),
            format_number(self.at.y),
            pad,
            format_number(self.diameter.unwrap_or(0.0)),
            pad
        ));
        self.color
            .unwrap_or(KicadColor {
                red: 0.0,
                green: 0.0,
                blue: 0.0,
                alpha: 0.0,
            })
            .write_inline_color_sexpr(output);
        output.push('\n');
        if let Some(uuid) = &self.uuid {
            output.push_str(&format!("{}  (uuid {})\n", pad, sexpr_string(uuid)));
        }
        output.push_str(&format!("{})\n", pad));
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadNoConnect {
    pub at: KicadPoint,
    pub uuid: Option<String>,
}

impl KicadNoConnect {
    /// write no connect sexpr。
    pub(crate) fn write_no_connect_sexpr(&self, output: &mut String, indent: usize) {
        let pad = " ".repeat(indent);
        output.push_str(&format!(
            "{}(no_connect\n{}  (at {} {})\n",
            pad,
            pad,
            format_number(self.at.x),
            format_number(self.at.y)
        ));
        if let Some(uuid) = &self.uuid {
            output.push_str(&format!("{}  (uuid {})\n", pad, sexpr_string(uuid)));
        }
        output.push_str(&format!("{})\n", pad));
    }
}

/// parse junction。
pub(crate) fn parse_junction(node: &Sexp) -> Option<KicadJunction> {
    let items = list_items(node);
    let at = child(items, "at").and_then(parse_at)?;
    Some(KicadJunction {
        at: KicadPoint { x: at.x, y: at.y },
        diameter: child_value(items, "diameter").and_then(|value| value.parse().ok()),
        color: child(items, "color").and_then(parse_color),
        uuid: child_value(items, "uuid"),
    })
}

/// parse no connect。
pub(crate) fn parse_no_connect(node: &Sexp) -> Option<KicadNoConnect> {
    let items = list_items(node);
    let at = child(items, "at").and_then(parse_at)?;
    Some(KicadNoConnect {
        at: KicadPoint { x: at.x, y: at.y },
        uuid: child_value(items, "uuid"),
    })
}
