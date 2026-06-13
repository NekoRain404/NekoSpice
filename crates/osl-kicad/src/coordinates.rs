//! KiCad coordinate types — KicadPoint, KicadAt, KicadSize with S-expression parsing.

use crate::sexpr::{Sexp, atom_text, direct_children, format_number, list_items};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KicadPoint {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KicadSize {
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KicadAt {
    pub x: f64,
    pub y: f64,
    pub rotation: f64,
}

impl KicadAt {
    /// point。
    pub(crate) fn point(self) -> KicadPoint {
        KicadPoint {
            x: self.x,
            y: self.y,
        }
    }
}

/// parse points。
pub(crate) fn parse_points(node: &Sexp) -> Vec<KicadPoint> {
    direct_children(list_items(node), "xy")
        .filter_map(parse_xy)
        .collect()
}

/// parse xy。
pub(crate) fn parse_xy(node: &Sexp) -> Option<KicadPoint> {
    let items = list_items(node);
    Some(KicadPoint {
        x: atom_text(items.get(1)?)?.parse().ok()?,
        y: atom_text(items.get(2)?)?.parse().ok()?,
    })
}

/// parse image at。
pub(crate) fn parse_image_at(node: &Sexp) -> Option<KicadPoint> {
    let items = list_items(node);
    Some(KicadPoint {
        x: atom_text(items.get(1)?)?.parse().ok()?,
        y: atom_text(items.get(2)?)?.parse().ok()?,
    })
}

/// parse size。
pub(crate) fn parse_size(node: &Sexp) -> Option<KicadSize> {
    let items = list_items(node);
    Some(KicadSize {
        width: atom_text(items.get(1)?)?.parse().ok()?,
        height: atom_text(items.get(2)?)?.parse().ok()?,
    })
}

/// parse at。
pub(crate) fn parse_at(node: &Sexp) -> Option<KicadAt> {
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

/// write points sexpr。
pub(crate) fn write_points_sexpr(output: &mut String, points: &[KicadPoint]) {
    let points = points
        .iter()
        .map(|point| format!("(xy {} {})", format_number(point.x), format_number(point.y)))
        .collect::<Vec<_>>()
        .join(" ");
    output.push_str(&format!(" (pts {})", points));
}

/// kicad point value。
pub(crate) fn kicad_point_value(point: KicadPoint) -> serde_json::Value {
    serde_json::json!({
        "x": point.x,
        "y": point.y,
    })
}

/// kicad points value。
pub(crate) fn kicad_points_value(points: &[KicadPoint]) -> serde_json::Value {
    serde_json::Value::Array(
        points
            .iter()
            .map(|point| kicad_point_value(*point))
            .collect(),
    )
}

/// kicad size value。
pub(crate) fn kicad_size_value(size: KicadSize) -> serde_json::Value {
    serde_json::json!({
        "width": size.width,
        "height": size.height,
    })
}

/// kicad at value。
pub(crate) fn kicad_at_value(at: KicadAt) -> serde_json::Value {
    serde_json::json!({
        "x": at.x,
        "y": at.y,
        "rotation": at.rotation,
    })
}
