//! schema coordinate types — NspPoint, NspAt, NspSize with S-expression parsing.

use crate::sexpr::{Sexp, atom_text, direct_children, format_number, list_items};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NspPoint {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NspSize {
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NspAt {
    pub x: f64,
    pub y: f64,
    pub rotation: f64,
}

impl NspAt {
    /// point。
    pub(crate) fn point(self) -> NspPoint {
        NspPoint {
            x: self.x,
            y: self.y,
        }
    }
}

/// parse points。
pub(crate) fn parse_points(node: &Sexp) -> Vec<NspPoint> {
    direct_children(list_items(node), "xy")
        .filter_map(parse_xy)
        .collect()
}

/// parse xy。
pub(crate) fn parse_xy(node: &Sexp) -> Option<NspPoint> {
    let items = list_items(node);
    Some(NspPoint {
        x: atom_text(items.get(1)?)?.parse().ok()?,
        y: atom_text(items.get(2)?)?.parse().ok()?,
    })
}

/// parse image at。
pub(crate) fn parse_image_at(node: &Sexp) -> Option<NspPoint> {
    let items = list_items(node);
    Some(NspPoint {
        x: atom_text(items.get(1)?)?.parse().ok()?,
        y: atom_text(items.get(2)?)?.parse().ok()?,
    })
}

/// parse size。
pub(crate) fn parse_size(node: &Sexp) -> Option<NspSize> {
    let items = list_items(node);
    Some(NspSize {
        width: atom_text(items.get(1)?)?.parse().ok()?,
        height: atom_text(items.get(2)?)?.parse().ok()?,
    })
}

/// parse at。
pub(crate) fn parse_at(node: &Sexp) -> Option<NspAt> {
    let items = list_items(node);
    Some(NspAt {
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
pub(crate) fn write_points_sexpr(output: &mut String, points: &[NspPoint]) {
    let points = points
        .iter()
        .map(|point| format!("(xy {} {})", format_number(point.x), format_number(point.y)))
        .collect::<Vec<_>>()
        .join(" ");
    output.push_str(&format!(" (pts {})", points));
}

/// schema point value。
pub(crate) fn schema_point_value(point: NspPoint) -> serde_json::Value {
    serde_json::json!({
        "x": point.x,
        "y": point.y,
    })
}

/// schema points value。
pub(crate) fn schema_points_value(points: &[NspPoint]) -> serde_json::Value {
    serde_json::Value::Array(
        points
            .iter()
            .map(|point| schema_point_value(*point))
            .collect(),
    )
}

/// schema size value。
pub(crate) fn schema_size_value(size: NspSize) -> serde_json::Value {
    serde_json::json!({
        "width": size.width,
        "height": size.height,
    })
}

/// schema at value。
pub(crate) fn schema_at_value(at: NspAt) -> serde_json::Value {
    serde_json::json!({
        "x": at.x,
        "y": at.y,
        "rotation": at.rotation,
    })
}
