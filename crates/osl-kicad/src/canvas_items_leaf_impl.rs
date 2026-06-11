#[derive(Debug, Clone, PartialEq)]
pub struct KicadCanvasImage {
    pub uuid: Option<String>,
    pub at: Option<KicadPoint>,
    pub scale: f64,
    pub data_base64: String,
    pub mime_type: String,
    pub image_size: Option<KicadSize>,
    pub bounds: Option<KicadBoundingBox>,
}

impl KicadCanvasImage {
    fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "uuid": self.uuid,
            "at": self.at.map(kicad_point_value),
            "scale": self.scale,
            "mime_type": self.mime_type,
            "image_size": self.image_size.map(kicad_size_value),
            "data_base64": self.data_base64,
            "bounds": self.bounds.map(kicad_bounding_box_value),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadCanvasTable {
    pub uuid: Option<String>,
    pub column_count: usize,
    pub column_widths: Vec<f64>,
    pub row_heights: Vec<f64>,
    pub cells: Vec<KicadCanvasTableCell>,
    pub bounds: Option<KicadBoundingBox>,
}

impl KicadCanvasTable {
    fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "uuid": self.uuid,
            "column_count": self.column_count,
            "column_widths": self.column_widths,
            "row_heights": self.row_heights,
            "cell_count": self.cells.len(),
            "cells": self.cells.iter().map(KicadCanvasTableCell::to_json_value).collect::<Vec<_>>(),
            "bounds": self.bounds.map(kicad_bounding_box_value),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadCanvasTableCell {
    pub uuid: Option<String>,
    pub text: String,
    pub at: Option<KicadAt>,
    pub size: Option<KicadSize>,
    pub margins: Option<KicadMargins>,
    pub column_span: usize,
    pub row_span: usize,
    pub fill: Option<KicadFill>,
    pub effects: Option<KicadTextEffects>,
    pub bounds: Option<KicadBoundingBox>,
}

impl KicadCanvasTableCell {
    fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "uuid": self.uuid,
            "text": self.text,
            "at": self.at.map(kicad_at_value),
            "size": self.size.map(kicad_size_value),
            "margins": self.margins.map(kicad_margins_value),
            "column_span": self.column_span,
            "row_span": self.row_span,
            "fill": self.fill.as_ref().map(kicad_fill_value),
            "effects": self.effects.as_ref().map(kicad_text_effects_value),
            "bounds": self.bounds.map(kicad_bounding_box_value),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadCanvasPin {
    pub number: String,
    pub name: String,
    pub electrical_type: String,
    /// Pin shape: "line" (default), "inverted", "clock", "inverted_clock",
    /// "input_low", "clock_low", "falling_edge_clock", "non_logic"
    pub shape: String,
    pub start: KicadPoint,
    pub end: KicadPoint,
    pub alternates: Vec<KicadPinAlternate>,
    pub name_effects: Option<KicadTextEffects>,
    pub number_effects: Option<KicadTextEffects>,
}

impl KicadCanvasPin {
    fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "number": self.number,
            "name": self.name,
            "electrical_type": self.electrical_type,
            "shape": self.shape,
            "start": kicad_point_value(self.start),
            "end": kicad_point_value(self.end),
            "alternate_count": self.alternates.len(),
            "name_effects": self.name_effects.as_ref().map(kicad_text_effects_value),
            "number_effects": self.number_effects.as_ref().map(kicad_text_effects_value),
            "alternates": self.alternates.iter().map(kicad_pin_alternate_value).collect::<Vec<_>>(),
            "bounds": kicad_points_bounds(&[self.start, self.end], KICAD_CANVAS_LINE_BOUNDS_PADDING).map(kicad_bounding_box_value),
        })
    }

    fn from_pin_def(pin: &KicadPinDef, symbol_at: KicadAt, mirror: Option<&str>) -> Option<Self> {
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
pub struct KicadCanvasWire {
    pub uuid: Option<String>,
    pub points: Vec<KicadPoint>,
    pub stroke: Option<KicadStroke>,
    pub bounds: Option<KicadBoundingBox>,
}

impl KicadCanvasWire {
    fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "uuid": self.uuid,
            "points": kicad_points_value(&self.points),
            "stroke": self.stroke.as_ref().map(kicad_stroke_value),
            "bounds": self.bounds.map(kicad_bounding_box_value),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadCanvasBus {
    pub uuid: Option<String>,
    pub points: Vec<KicadPoint>,
    pub stroke: Option<KicadStroke>,
    pub bounds: Option<KicadBoundingBox>,
}

impl KicadCanvasBus {
    fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "uuid": self.uuid,
            "points": kicad_points_value(&self.points),
            "stroke": self.stroke.as_ref().map(kicad_stroke_value),
            "bounds": self.bounds.map(kicad_bounding_box_value),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadCanvasBusEntry {
    pub uuid: Option<String>,
    pub at: KicadPoint,
    pub size: KicadSize,
    pub stroke: Option<KicadStroke>,
    pub bounds: Option<KicadBoundingBox>,
}

impl KicadCanvasBusEntry {
    pub fn end(&self) -> KicadPoint {
        KicadPoint {
            x: self.at.x + self.size.width,
            y: self.at.y + self.size.height,
        }
    }

    fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "uuid": self.uuid,
            "at": kicad_point_value(self.at),
            "size": kicad_size_value(self.size),
            "end": kicad_point_value(self.end()),
            "stroke": self.stroke.as_ref().map(kicad_stroke_value),
            "bounds": self.bounds.map(kicad_bounding_box_value),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadCanvasLabel {
    pub uuid: Option<String>,
    pub text: String,
    pub kind: KicadLabelKind,
    pub at: Option<KicadAt>,
    pub effects: Option<KicadTextEffects>,
    pub bounds: Option<KicadBoundingBox>,
}

impl KicadCanvasLabel {
    fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "uuid": self.uuid,
            "text": self.text,
            "kind": self.kind.as_str(),
            "at": self.at.map(kicad_at_value),
            "effects": self.effects.as_ref().map(kicad_text_effects_value),
            "bounds": self.bounds.map(kicad_bounding_box_value),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadCanvasText {
    pub uuid: Option<String>,
    pub text: String,
    pub at: Option<KicadAt>,
    pub is_spice_directive: bool,
    pub effects: Option<KicadTextEffects>,
    pub bounds: Option<KicadBoundingBox>,
}

impl KicadCanvasText {
    fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "uuid": self.uuid,
            "text": self.text,
            "at": self.at.map(kicad_at_value),
            "is_spice_directive": self.is_spice_directive,
            "effects": self.effects.as_ref().map(kicad_text_effects_value),
            "bounds": self.bounds.map(kicad_bounding_box_value),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadCanvasTextBox {
    pub uuid: Option<String>,
    pub text: String,
    pub at: Option<KicadAt>,
    pub size: Option<KicadSize>,
    pub margins: Option<KicadMargins>,
    pub stroke: Option<KicadStroke>,
    pub fill: Option<KicadFill>,
    pub effects: Option<KicadTextEffects>,
    pub bounds: Option<KicadBoundingBox>,
}

impl KicadCanvasTextBox {
    fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "uuid": self.uuid,
            "text": self.text,
            "at": self.at.map(kicad_at_value),
            "size": self.size.map(kicad_size_value),
            "margins": self.margins.map(kicad_margins_value),
            "stroke": self.stroke.as_ref().map(kicad_stroke_value),
            "fill": self.fill.as_ref().map(kicad_fill_value),
            "effects": self.effects.as_ref().map(kicad_text_effects_value),
            "bounds": self.bounds.map(kicad_bounding_box_value),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadCanvasJunction {
    pub uuid: Option<String>,
    pub at: KicadPoint,
    pub diameter: Option<f64>,
    pub color: Option<KicadColor>,
    pub bounds: KicadBoundingBox,
}

impl KicadCanvasJunction {
    fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "uuid": self.uuid,
            "at": kicad_point_value(self.at),
            "diameter": self.diameter,
            "color": self.color.map(kicad_color_value),
            "bounds": kicad_bounding_box_value(self.bounds),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadCanvasNoConnect {
    pub uuid: Option<String>,
    pub at: KicadPoint,
    pub bounds: KicadBoundingBox,
}

impl KicadCanvasNoConnect {
    fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "uuid": self.uuid,
            "at": kicad_point_value(self.at),
            "bounds": kicad_bounding_box_value(self.bounds),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadCanvasGroup {
    pub uuid: Option<String>,
    pub name: String,
    pub locked: Option<bool>,
    pub members: Vec<String>,
    pub bounds: Option<KicadBoundingBox>,
}

impl KicadCanvasGroup {
    fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "uuid": self.uuid,
            "name": self.name,
            "locked": self.locked,
            "member_count": self.members.len(),
            "members": self.members,
            "bounds": self.bounds.map(kicad_bounding_box_value),
        })
    }
}

