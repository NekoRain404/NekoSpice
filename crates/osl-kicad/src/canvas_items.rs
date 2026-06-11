// Canvas item type definitions for schematic rendering.
// Covers: KicadCanvasSymbol, KicadCanvasWire, KicadCanvasBus,
// KicadCanvasLabel, KicadCanvasGraphic, KicadCanvasPin, etc.

#[derive(Debug, Clone, PartialEq)]
pub struct KicadCanvasSymbol {
    pub uuid: Option<String>,
    pub lib_id: String,
    pub reference: String,
    pub value: String,
    pub at: KicadAt,
    pub mirror: Option<String>,
    pub graphics: Vec<KicadCanvasGraphic>,
    pub pins: Vec<KicadCanvasPin>,
    pub pin_names: Option<KicadPinDisplay>,
    pub pin_numbers: Option<KicadPinDisplay>,
    pub unit_name: Option<String>,
    pub bounds: Option<KicadBoundingBox>,
}

impl KicadCanvasSymbol {
    fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "uuid": self.uuid,
            "lib_id": self.lib_id,
            "reference": self.reference,
            "value": self.value,
            "at": kicad_at_value(self.at),
            "mirror": self.mirror,
            "unit_name": self.unit_name,
            "pin_names": self.pin_names.as_ref().map(kicad_pin_display_value),
            "pin_numbers": self.pin_numbers.as_ref().map(kicad_pin_display_value),
            "bounds": self.bounds.map(kicad_bounding_box_value),
            "graphics": self.graphics.iter().map(KicadCanvasGraphic::to_json_value).collect::<Vec<_>>(),
            "pins": self.pins.iter().map(KicadCanvasPin::to_json_value).collect::<Vec<_>>(),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadCanvasSheet {
    pub uuid: Option<String>,
    pub name: String,
    pub file: String,
    pub at: Option<KicadAt>,
    pub size: Option<KicadSize>,
    pub stroke: Option<KicadStroke>,
    pub fill: Option<KicadFill>,
    pub pins: Vec<KicadCanvasSheetPin>,
    pub bounds: Option<KicadBoundingBox>,
}

impl KicadCanvasSheet {
    fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "uuid": self.uuid,
            "name": self.name,
            "file": self.file,
            "at": self.at.map(kicad_at_value),
            "size": self.size.map(kicad_size_value),
            "stroke": self.stroke.as_ref().map(kicad_stroke_value),
            "fill": self.fill.as_ref().map(kicad_fill_value),
            "pin_count": self.pins.len(),
            "pins": self.pins.iter().map(KicadCanvasSheetPin::to_json_value).collect::<Vec<_>>(),
            "bounds": self.bounds.map(kicad_bounding_box_value),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadCanvasSheetPin {
    pub uuid: Option<String>,
    pub name: String,
    pub pin_type: String,
    pub at: Option<KicadAt>,
    pub effects: Option<KicadTextEffects>,
    pub bounds: Option<KicadBoundingBox>,
}

impl KicadCanvasSheetPin {
    fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "uuid": self.uuid,
            "name": self.name,
            "pin_type": self.pin_type,
            "at": self.at.map(kicad_at_value),
            "effects": self.effects.as_ref().map(kicad_text_effects_value),
            "bounds": self.bounds.map(kicad_bounding_box_value),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadCanvasRuleArea {
    pub uuid: Option<String>,
    pub points: Vec<KicadPoint>,
    pub stroke: Option<KicadStroke>,
    pub fill: Option<KicadFill>,
    pub bounds: Option<KicadBoundingBox>,
}

impl KicadCanvasRuleArea {
    fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "uuid": self.uuid,
            "points": kicad_points_value(&self.points),
            "stroke": self.stroke.as_ref().map(kicad_stroke_value),
            "fill": self.fill.as_ref().map(kicad_fill_value),
            "bounds": self.bounds.map(kicad_bounding_box_value),
        })
    }

    pub(crate) fn hits_point(&self, point: KicadPoint) -> bool {
        if kicad_fill_is_solid(self.fill.as_ref())
            && kicad_polygon_contains_point(&self.points, point)
        {
            return true;
        }
        kicad_closed_polyline_hits_point(&self.points, self.stroke.as_ref(), point)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadCanvasDirectiveLabel {
    pub uuid: Option<String>,
    pub text: String,
    pub at: Option<KicadAt>,
    pub length: Option<f64>,
    pub shape: Option<String>,
    pub effects: Option<KicadTextEffects>,
    pub properties: Vec<KicadProperty>,
    pub bounds: Option<KicadBoundingBox>,
}

impl KicadCanvasDirectiveLabel {
    fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "uuid": self.uuid,
            "text": self.text,
            "at": self.at.map(kicad_at_value),
            "length": self.length,
            "shape": self.shape,
            "effects": self.effects.as_ref().map(kicad_text_effects_value),
            "properties": self.properties.iter().map(kicad_property_value).collect::<Vec<_>>(),
            "bounds": self.bounds.map(kicad_bounding_box_value),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum KicadCanvasGraphic {
    Polyline {
        uuid: Option<String>,
        points: Vec<KicadPoint>,
        stroke: Option<KicadStroke>,
        fill: Option<KicadFill>,
    },
    Bezier {
        uuid: Option<String>,
        points: Vec<KicadPoint>,
        stroke: Option<KicadStroke>,
        fill: Option<KicadFill>,
    },
    Rectangle {
        uuid: Option<String>,
        start: KicadPoint,
        end: KicadPoint,
        stroke: Option<KicadStroke>,
        fill: Option<KicadFill>,
    },
    Circle {
        uuid: Option<String>,
        center: KicadPoint,
        radius: f64,
        stroke: Option<KicadStroke>,
        fill: Option<KicadFill>,
    },
    Arc {
        uuid: Option<String>,
        start: KicadPoint,
        mid: Option<KicadPoint>,
        end: KicadPoint,
        stroke: Option<KicadStroke>,
        fill: Option<KicadFill>,
    },
    Text {
        uuid: Option<String>,
        text: String,
        at: Option<KicadAt>,
        effects: Option<KicadTextEffects>,
        stroke: Option<KicadStroke>,
        fill: Option<KicadFill>,
    },
}

impl KicadCanvasGraphic {
    fn to_json_value(&self) -> serde_json::Value {
        match self {
            Self::Polyline {
                uuid,
                points,
                stroke,
                fill,
            } => serde_json::json!({
                "kind": "polyline",
                "uuid": uuid,
                "points": kicad_points_value(points),
                "stroke": stroke.as_ref().map(kicad_stroke_value),
                "fill": fill.as_ref().map(kicad_fill_value),
                "bounds": self.bounds().map(kicad_bounding_box_value),
            }),
            Self::Bezier {
                uuid,
                points,
                stroke,
                fill,
            } => serde_json::json!({
                "kind": "bezier",
                "uuid": uuid,
                "points": kicad_points_value(points),
                "stroke": stroke.as_ref().map(kicad_stroke_value),
                "fill": fill.as_ref().map(kicad_fill_value),
                "bounds": self.bounds().map(kicad_bounding_box_value),
            }),
            Self::Rectangle {
                uuid,
                start,
                end,
                stroke,
                fill,
            } => serde_json::json!({
                "kind": "rectangle",
                "uuid": uuid,
                "start": kicad_point_value(*start),
                "end": kicad_point_value(*end),
                "stroke": stroke.as_ref().map(kicad_stroke_value),
                "fill": fill.as_ref().map(kicad_fill_value),
                "bounds": self.bounds().map(kicad_bounding_box_value),
            }),
            Self::Circle {
                uuid,
                center,
                radius,
                stroke,
                fill,
            } => serde_json::json!({
                "kind": "circle",
                "uuid": uuid,
                "center": kicad_point_value(*center),
                "radius": radius,
                "stroke": stroke.as_ref().map(kicad_stroke_value),
                "fill": fill.as_ref().map(kicad_fill_value),
                "bounds": self.bounds().map(kicad_bounding_box_value),
            }),
            Self::Arc {
                uuid,
                start,
                mid,
                end,
                stroke,
                fill,
            } => serde_json::json!({
                "kind": "arc",
                "uuid": uuid,
                "start": kicad_point_value(*start),
                "mid": mid.map(kicad_point_value),
                "end": kicad_point_value(*end),
                "stroke": stroke.as_ref().map(kicad_stroke_value),
                "fill": fill.as_ref().map(kicad_fill_value),
                "bounds": self.bounds().map(kicad_bounding_box_value),
            }),
            Self::Text {
                uuid,
                text,
                at,
                effects,
                stroke,
                fill,
            } => serde_json::json!({
                "kind": "text",
                "uuid": uuid,
                "text": text,
                "at": at.map(kicad_at_value),
                "effects": effects.as_ref().map(kicad_text_effects_value),
                "stroke": stroke.as_ref().map(kicad_stroke_value),
                "fill": fill.as_ref().map(kicad_fill_value),
                "bounds": self.bounds().map(kicad_bounding_box_value),
            }),
        }
    }

    pub(crate) fn with_style(
        mut self,
        stroke: Option<KicadStroke>,
        fill: Option<KicadFill>,
    ) -> Self {
        match &mut self {
            Self::Polyline {
                stroke: graphic_stroke,
                fill: graphic_fill,
                ..
            }
            | Self::Bezier {
                stroke: graphic_stroke,
                fill: graphic_fill,
                ..
            }
            | Self::Rectangle {
                stroke: graphic_stroke,
                fill: graphic_fill,
                ..
            }
            | Self::Circle {
                stroke: graphic_stroke,
                fill: graphic_fill,
                ..
            }
            | Self::Arc {
                stroke: graphic_stroke,
                fill: graphic_fill,
                ..
            }
            | Self::Text {
                stroke: graphic_stroke,
                fill: graphic_fill,
                ..
            } => {
                *graphic_stroke = stroke;
                *graphic_fill = fill;
            }
        }
        self
    }

    pub(crate) fn with_uuid(mut self, uuid: Option<String>) -> Self {
        match &mut self {
            Self::Polyline {
                uuid: graphic_uuid, ..
            }
            | Self::Bezier {
                uuid: graphic_uuid, ..
            }
            | Self::Rectangle {
                uuid: graphic_uuid, ..
            }
            | Self::Circle {
                uuid: graphic_uuid, ..
            }
            | Self::Arc {
                uuid: graphic_uuid, ..
            }
            | Self::Text {
                uuid: graphic_uuid, ..
            } => {
                *graphic_uuid = uuid;
            }
        }
        self
    }

    pub(crate) fn uuid(&self) -> Option<String> {
        match self {
            Self::Polyline { uuid, .. }
            | Self::Bezier { uuid, .. }
            | Self::Rectangle { uuid, .. }
            | Self::Circle { uuid, .. }
            | Self::Arc { uuid, .. }
            | Self::Text { uuid, .. } => uuid.clone(),
        }
    }

    pub(crate) fn kind(&self) -> &'static str {
        match self {
            Self::Polyline { .. } => "polyline",
            Self::Bezier { .. } => "bezier",
            Self::Rectangle { .. } => "rectangle",
            Self::Circle { .. } => "circle",
            Self::Arc { .. } => "arc",
            Self::Text { .. } => "text",
        }
    }

    pub(crate) fn hits_point(&self, point: KicadPoint) -> bool {
        match self {
            Self::Polyline { points, stroke, .. } => {
                kicad_polyline_hits_point(points, stroke.as_ref(), point)
            }
            Self::Bezier { points, stroke, .. } => {
                kicad_bezier_hits_point(points, stroke.as_ref(), point)
            }
            Self::Rectangle {
                start,
                end,
                stroke,
                fill,
                ..
            } => kicad_rectangle_hits_point(*start, *end, stroke.as_ref(), fill.as_ref(), point),
            Self::Circle {
                center,
                radius,
                stroke,
                fill,
                ..
            } => kicad_circle_hits_point(*center, *radius, stroke.as_ref(), fill.as_ref(), point),
            Self::Arc {
                start,
                mid,
                end,
                stroke,
                ..
            } => kicad_arc_hits_point(*start, *mid, *end, stroke.as_ref(), point),
            Self::Text {
                text, at, effects, ..
            } => kicad_text_bounds(text, *at, effects.as_ref())
                .is_some_and(|bounds| bounds.contains(point)),
        }
    }

    pub(crate) fn include_in_bounds(&self, bounds: &mut KicadBoundingBoxBuilder) {
        match self {
            Self::Polyline { points, .. } => {
                for point in points {
                    bounds.include(*point);
                }
            }
            Self::Bezier { points, .. } => {
                for point in points {
                    bounds.include(*point);
                }
            }
            Self::Rectangle { start, end, .. } => {
                bounds.include(*start);
                bounds.include(*end);
            }
            Self::Circle { center, radius, .. } => {
                bounds.include(KicadPoint {
                    x: center.x - radius,
                    y: center.y - radius,
                });
                bounds.include(KicadPoint {
                    x: center.x + radius,
                    y: center.y + radius,
                });
            }
            Self::Arc {
                start, mid, end, ..
            } => {
                for point in sample_kicad_arc_points(*start, *mid, *end) {
                    bounds.include(point);
                }
            }
            Self::Text {
                text, at, effects, ..
            } => {
                if let Some(text_bounds) = kicad_text_bounds(text, *at, effects.as_ref()) {
                    bounds.include_box(text_bounds);
                }
            }
        }
    }

    pub fn bounds(&self) -> Option<KicadBoundingBox> {
        match self {
            Self::Polyline { points, .. } | Self::Bezier { points, .. } => {
                kicad_points_bounds(points, KICAD_CANVAS_LINE_BOUNDS_PADDING)
            }
            Self::Rectangle { start, end, .. } => {
                kicad_points_bounds(&[*start, *end], KICAD_CANVAS_LINE_BOUNDS_PADDING)
            }
            Self::Circle { center, radius, .. } => Some(KicadBoundingBox {
                min: KicadPoint {
                    x: center.x - radius,
                    y: center.y - radius,
                },
                max: KicadPoint {
                    x: center.x + radius,
                    y: center.y + radius,
                },
            }),
            Self::Arc {
                start, mid, end, ..
            } => kicad_points_bounds(
                &sample_kicad_arc_points(*start, *mid, *end),
                KICAD_CANVAS_LINE_BOUNDS_PADDING,
            ),
            Self::Text {
                text, at, effects, ..
            } => kicad_text_bounds(text, *at, effects.as_ref()),
        }
    }
}

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

#[allow(clippy::too_many_arguments)]
fn kicad_canvas_item_bounds(
    symbols: &[KicadCanvasSymbol],
    sheets: &[KicadCanvasSheet],
    graphics: &[KicadCanvasGraphic],
    images: &[KicadCanvasImage],
    tables: &[KicadCanvasTable],
    rule_areas: &[KicadCanvasRuleArea],
    wires: &[KicadCanvasWire],
    buses: &[KicadCanvasBus],
    bus_entries: &[KicadCanvasBusEntry],
    directive_labels: &[KicadCanvasDirectiveLabel],
    labels: &[KicadCanvasLabel],
    text_items: &[KicadCanvasText],
    text_boxes: &[KicadCanvasTextBox],
    junctions: &[KicadCanvasJunction],
    no_connects: &[KicadCanvasNoConnect],
) -> BTreeMap<String, KicadBoundingBox> {
    let mut item_bounds = BTreeMap::new();
    for symbol in symbols {
        insert_canvas_item_bounds(&mut item_bounds, symbol.uuid.as_deref(), symbol.bounds);
    }
    for sheet in sheets {
        insert_canvas_item_bounds(&mut item_bounds, sheet.uuid.as_deref(), sheet.bounds);
    }
    for graphic in graphics {
        insert_canvas_item_bounds(
            &mut item_bounds,
            graphic.uuid().as_deref(),
            graphic.bounds(),
        );
    }
    for image in images {
        insert_canvas_item_bounds(&mut item_bounds, image.uuid.as_deref(), image.bounds);
    }
    for table in tables {
        insert_canvas_item_bounds(&mut item_bounds, table.uuid.as_deref(), table.bounds);
        for cell in &table.cells {
            insert_canvas_item_bounds(&mut item_bounds, cell.uuid.as_deref(), cell.bounds);
        }
    }
    for rule_area in rule_areas {
        insert_canvas_item_bounds(
            &mut item_bounds,
            rule_area.uuid.as_deref(),
            rule_area.bounds,
        );
    }
    for wire in wires {
        insert_canvas_item_bounds(&mut item_bounds, wire.uuid.as_deref(), wire.bounds);
    }
    for bus in buses {
        insert_canvas_item_bounds(&mut item_bounds, bus.uuid.as_deref(), bus.bounds);
    }
    for entry in bus_entries {
        insert_canvas_item_bounds(&mut item_bounds, entry.uuid.as_deref(), entry.bounds);
    }
    for label in directive_labels {
        insert_canvas_item_bounds(&mut item_bounds, label.uuid.as_deref(), label.bounds);
    }
    for label in labels {
        insert_canvas_item_bounds(&mut item_bounds, label.uuid.as_deref(), label.bounds);
    }
    for text in text_items {
        insert_canvas_item_bounds(&mut item_bounds, text.uuid.as_deref(), text.bounds);
    }
    for text_box in text_boxes {
        insert_canvas_item_bounds(&mut item_bounds, text_box.uuid.as_deref(), text_box.bounds);
    }
    for junction in junctions {
        insert_canvas_item_bounds(
            &mut item_bounds,
            junction.uuid.as_deref(),
            Some(junction.bounds),
        );
    }
    for marker in no_connects {
        insert_canvas_item_bounds(
            &mut item_bounds,
            marker.uuid.as_deref(),
            Some(marker.bounds),
        );
    }
    item_bounds
}

fn canvas_symbol_bounds(
    graphics: &[KicadCanvasGraphic],
    pins: &[KicadCanvasPin],
) -> Option<KicadBoundingBox> {
    let mut bounds = KicadBoundingBoxBuilder::default();
    for graphic in graphics {
        if let Some(graphic_bounds) = graphic.bounds() {
            bounds.include_box(graphic_bounds);
        } else {
            graphic.include_in_bounds(&mut bounds);
        }
    }
    for pin in pins {
        if let Some(pin_bounds) =
            kicad_points_bounds(&[pin.start, pin.end], KICAD_CANVAS_LINE_BOUNDS_PADDING)
        {
            bounds.include_box(pin_bounds);
        }
    }
    bounds.finish()
}

fn insert_canvas_item_bounds(
    item_bounds: &mut BTreeMap<String, KicadBoundingBox>,
    uuid: Option<&str>,
    bounds: Option<KicadBoundingBox>,
) {
    if let (Some(uuid), Some(bounds)) = (uuid, bounds) {
        item_bounds.insert(uuid.to_string(), bounds);
    }
}
