#[derive(Debug, Clone, PartialEq)]
pub struct NspCanvasImage {
    pub uuid: Option<String>,
    pub at: Option<NspPoint>,
    pub scale: f64,
    pub data_base64: String,
    pub mime_type: String,
    pub image_size: Option<NspSize>,
    pub bounds: Option<NspBoundingBox>,
}

impl NspCanvasImage {
    fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "uuid": self.uuid,
            "at": self.at.map(schema_point_value),
            "scale": self.scale,
            "mime_type": self.mime_type,
            "image_size": self.image_size.map(schema_size_value),
            "data_base64": self.data_base64,
            "bounds": self.bounds.map(schema_bounding_box_value),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NspCanvasTable {
    pub uuid: Option<String>,
    pub column_count: usize,
    pub column_widths: Vec<f64>,
    pub row_heights: Vec<f64>,
    pub cells: Vec<NspCanvasTableCell>,
    pub bounds: Option<NspBoundingBox>,
}

impl NspCanvasTable {
    fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "uuid": self.uuid,
            "column_count": self.column_count,
            "column_widths": self.column_widths,
            "row_heights": self.row_heights,
            "cell_count": self.cells.len(),
            "cells": self.cells.iter().map(NspCanvasTableCell::to_json_value).collect::<Vec<_>>(),
            "bounds": self.bounds.map(schema_bounding_box_value),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NspCanvasTableCell {
    pub uuid: Option<String>,
    pub text: String,
    pub at: Option<NspAt>,
    pub size: Option<NspSize>,
    pub margins: Option<NspMargins>,
    pub column_span: usize,
    pub row_span: usize,
    pub fill: Option<NspFill>,
    pub effects: Option<NspTextEffects>,
    pub bounds: Option<NspBoundingBox>,
}

impl NspCanvasTableCell {
    fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "uuid": self.uuid,
            "text": self.text,
            "at": self.at.map(schema_at_value),
            "size": self.size.map(schema_size_value),
            "margins": self.margins.map(schema_margins_value),
            "column_span": self.column_span,
            "row_span": self.row_span,
            "fill": self.fill.as_ref().map(schema_fill_value),
            "effects": self.effects.as_ref().map(schema_text_effects_value),
            "bounds": self.bounds.map(schema_bounding_box_value),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NspCanvasPin {
    pub number: String,
    pub name: String,
    pub electrical_type: String,
    /// Pin shape: "line" (default), "inverted", "clock", "inverted_clock",
    /// "input_low", "clock_low", "falling_edge_clock", "non_logic"
    pub shape: String,
    pub start: NspPoint,
    pub end: NspPoint,
    pub alternates: Vec<NspPinAlternate>,
    pub name_effects: Option<NspTextEffects>,
    pub number_effects: Option<NspTextEffects>,
}

impl NspCanvasPin {
    fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "number": self.number,
            "name": self.name,
            "electrical_type": self.electrical_type,
            "shape": self.shape,
            "start": schema_point_value(self.start),
            "end": schema_point_value(self.end),
            "alternate_count": self.alternates.len(),
            "name_effects": self.name_effects.as_ref().map(schema_text_effects_value),
            "number_effects": self.number_effects.as_ref().map(schema_text_effects_value),
            "alternates": self.alternates.iter().map(schema_pin_alternate_value).collect::<Vec<_>>(),
            "bounds": schema_points_bounds(&[self.start, self.end], SCHEMA_CANVAS_LINE_BOUNDS_PADDING).map(schema_bounding_box_value),
        })
    }

    fn from_pin_def(pin: &NspPinDef, symbol_at: NspAt, mirror: Option<&str>) -> Option<Self> {
        let pin_at = pin.at?;
        let local_start = pin_at.point();
        let local_end = pin_body_end(pin_at, pin.length.unwrap_or(0.0));

        Some(Self {
            number: pin.number().to_string(),
            name: pin.name().to_string(),
            electrical_type: pin.electrical_type.clone(),
            shape: pin.shape.clone(),
            start: transform_local_point(local_start, symbol_at, mirror),
            end: transform_local_point(local_end, symbol_at, mirror),
            alternates: pin.alternates.clone(),
            name_effects: pin.name_effects().cloned(),
            number_effects: pin.number_effects().cloned(),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NspCanvasWire {
    pub uuid: Option<String>,
    pub points: Vec<NspPoint>,
    pub stroke: Option<NspStroke>,
    pub bounds: Option<NspBoundingBox>,
}

impl NspCanvasWire {
    fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "uuid": self.uuid,
            "points": schema_points_value(&self.points),
            "stroke": self.stroke.as_ref().map(schema_stroke_value),
            "bounds": self.bounds.map(schema_bounding_box_value),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NspCanvasBus {
    pub uuid: Option<String>,
    pub points: Vec<NspPoint>,
    pub stroke: Option<NspStroke>,
    pub bounds: Option<NspBoundingBox>,
}

impl NspCanvasBus {
    fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "uuid": self.uuid,
            "points": schema_points_value(&self.points),
            "stroke": self.stroke.as_ref().map(schema_stroke_value),
            "bounds": self.bounds.map(schema_bounding_box_value),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NspCanvasBusEntry {
    pub uuid: Option<String>,
    pub at: NspPoint,
    pub size: NspSize,
    pub stroke: Option<NspStroke>,
    pub bounds: Option<NspBoundingBox>,
}

impl NspCanvasBusEntry {
    pub fn end(&self) -> NspPoint {
        NspPoint {
            x: self.at.x + self.size.width,
            y: self.at.y + self.size.height,
        }
    }

    fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "uuid": self.uuid,
            "at": schema_point_value(self.at),
            "size": schema_size_value(self.size),
            "end": schema_point_value(self.end()),
            "stroke": self.stroke.as_ref().map(schema_stroke_value),
            "bounds": self.bounds.map(schema_bounding_box_value),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NspCanvasLabel {
    pub uuid: Option<String>,
    pub text: String,
    pub kind: NspLabelKind,
    pub at: Option<NspAt>,
    pub effects: Option<NspTextEffects>,
    pub bounds: Option<NspBoundingBox>,
}

impl NspCanvasLabel {
    fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "uuid": self.uuid,
            "text": self.text,
            "kind": self.kind.as_str(),
            "at": self.at.map(schema_at_value),
            "effects": self.effects.as_ref().map(schema_text_effects_value),
            "bounds": self.bounds.map(schema_bounding_box_value),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NspCanvasText {
    pub uuid: Option<String>,
    pub text: String,
    pub at: Option<NspAt>,
    pub is_spice_directive: bool,
    pub effects: Option<NspTextEffects>,
    pub bounds: Option<NspBoundingBox>,
}

impl NspCanvasText {
    fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "uuid": self.uuid,
            "text": self.text,
            "at": self.at.map(schema_at_value),
            "is_spice_directive": self.is_spice_directive,
            "effects": self.effects.as_ref().map(schema_text_effects_value),
            "bounds": self.bounds.map(schema_bounding_box_value),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NspCanvasTextBox {
    pub uuid: Option<String>,
    pub text: String,
    pub at: Option<NspAt>,
    pub size: Option<NspSize>,
    pub margins: Option<NspMargins>,
    pub stroke: Option<NspStroke>,
    pub fill: Option<NspFill>,
    pub effects: Option<NspTextEffects>,
    pub bounds: Option<NspBoundingBox>,
}

impl NspCanvasTextBox {
    fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "uuid": self.uuid,
            "text": self.text,
            "at": self.at.map(schema_at_value),
            "size": self.size.map(schema_size_value),
            "margins": self.margins.map(schema_margins_value),
            "stroke": self.stroke.as_ref().map(schema_stroke_value),
            "fill": self.fill.as_ref().map(schema_fill_value),
            "effects": self.effects.as_ref().map(schema_text_effects_value),
            "bounds": self.bounds.map(schema_bounding_box_value),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NspCanvasJunction {
    pub uuid: Option<String>,
    pub at: NspPoint,
    pub diameter: Option<f64>,
    pub color: Option<NspColor>,
    pub bounds: NspBoundingBox,
}

impl NspCanvasJunction {
    fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "uuid": self.uuid,
            "at": schema_point_value(self.at),
            "diameter": self.diameter,
            "color": self.color.map(schema_color_value),
            "bounds": schema_bounding_box_value(self.bounds),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NspCanvasNoConnect {
    pub uuid: Option<String>,
    pub at: NspPoint,
    pub bounds: NspBoundingBox,
}

impl NspCanvasNoConnect {
    fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "uuid": self.uuid,
            "at": schema_point_value(self.at),
            "bounds": schema_bounding_box_value(self.bounds),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NspCanvasGroup {
    pub uuid: Option<String>,
    pub name: String,
    pub locked: Option<bool>,
    pub members: Vec<String>,
    pub bounds: Option<NspBoundingBox>,
}

impl NspCanvasGroup {
    fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "uuid": self.uuid,
            "name": self.name,
            "locked": self.locked,
            "member_count": self.members.len(),
            "members": self.members,
            "bounds": self.bounds.map(schema_bounding_box_value),
        })
    }
}

