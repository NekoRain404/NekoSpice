//! JSON serialization helpers for schema canvas scene data.

use nsp_core::json_escape;

use crate::{NspBoundingBox, NspProperty, schema_at_value, schema_text_effects_value};

/// json option。
pub(crate) fn json_option(value: Option<&str>) -> String {
    match value {
        Some(value) => format!("\"{}\"", json_escape(value)),
        None => "null".to_string(),
    }
}

/// json u64 option。
pub(crate) fn json_u64_option(value: Option<u64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_string())
}

/// json bool option。
pub(crate) fn json_bool_option(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "true",
        Some(false) => "false",
        None => "null",
    }
}

/// schema bounding box json。
pub(crate) fn schema_bounding_box_json(bounds: NspBoundingBox) -> String {
    format!(
        concat!(
            "{{ ",
            "\"min\": {{ \"x\": {}, \"y\": {} }}, ",
            "\"max\": {{ \"x\": {}, \"y\": {} }}, ",
            "\"width\": {}, ",
            "\"height\": {} ",
            "}}"
        ),
        bounds.min.x,
        bounds.min.y,
        bounds.max.x,
        bounds.max.y,
        bounds.width(),
        bounds.height()
    )
}

/// schema bounding box value。
pub(crate) fn schema_bounding_box_value(bounds: NspBoundingBox) -> serde_json::Value {
    serde_json::json!({
        "min": {
            "x": bounds.min.x,
            "y": bounds.min.y,
        },
        "max": {
            "x": bounds.max.x,
            "y": bounds.max.y,
        },
        "width": bounds.width(),
        "height": bounds.height(),
    })
}

/// schema property value。
pub(crate) fn schema_property_value(property: &NspProperty) -> serde_json::Value {
    serde_json::json!({
        "name": property.name,
        "value": property.value,
        "id": property.id,
        "at": property.at.map(schema_at_value),
        "hide": property.hide,
        "show_name": property.show_name,
        "do_not_autoplace": property.do_not_autoplace,
        "effects": property.effects.as_ref().map(schema_text_effects_value),
    })
}
