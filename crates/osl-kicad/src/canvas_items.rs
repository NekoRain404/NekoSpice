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


// KicadCanvasGraphic enum and impl (arc, bezier, polyline, etc).
include!("canvas_items_graphic_impl.rs");

// Simple leaf canvas types (image, table, pin, wire, bus, label, text, etc).
include!("canvas_items_leaf_impl.rs");

// Canvas item bounds calculation helpers.
include!("canvas_items_bounds_impl.rs");
