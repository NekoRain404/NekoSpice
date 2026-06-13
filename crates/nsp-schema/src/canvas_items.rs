// Canvas item type definitions for schematic rendering.
// Covers: NspCanvasSymbol, NspCanvasWire, NspCanvasBus,
// NspCanvasLabel, NspCanvasGraphic, NspCanvasPin, etc.

#[derive(Debug, Clone, PartialEq)]
pub struct NspCanvasSymbol {
    pub uuid: Option<String>,
    pub lib_id: String,
    pub reference: String,
    pub value: String,
    /// Position from the Reference property in the schematic
    pub reference_at: Option<NspAt>,
    /// Font size from the Reference property effects
    pub reference_effects: Option<NspTextEffects>,
    /// Position from the Value property in the schematic
    pub value_at: Option<NspAt>,
    /// Font size from the Value property effects
    pub value_effects: Option<NspTextEffects>,
    pub at: NspAt,
    pub mirror: Option<String>,
    pub graphics: Vec<NspCanvasGraphic>,
    pub pins: Vec<NspCanvasPin>,
    pub pin_names: Option<NspPinDisplay>,
    pub pin_numbers: Option<NspPinDisplay>,
    pub unit_name: Option<String>,
    pub bounds: Option<NspBoundingBox>,
}

impl NspCanvasSymbol {
    fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "uuid": self.uuid,
            "lib_id": self.lib_id,
            "reference": self.reference,
            "value": self.value,
            "at": schema_at_value(self.at),
            "mirror": self.mirror,
            "unit_name": self.unit_name,
            "pin_names": self.pin_names.as_ref().map(schema_pin_display_value),
            "pin_numbers": self.pin_numbers.as_ref().map(schema_pin_display_value),
            "bounds": self.bounds.map(schema_bounding_box_value),
            "graphics": self.graphics.iter().map(NspCanvasGraphic::to_json_value).collect::<Vec<_>>(),
            "pins": self.pins.iter().map(NspCanvasPin::to_json_value).collect::<Vec<_>>(),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NspCanvasSheet {
    pub uuid: Option<String>,
    pub name: String,
    pub file: String,
    pub at: Option<NspAt>,
    pub size: Option<NspSize>,
    pub stroke: Option<NspStroke>,
    pub fill: Option<NspFill>,
    pub pins: Vec<NspCanvasSheetPin>,
    pub bounds: Option<NspBoundingBox>,
}

impl NspCanvasSheet {
    fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "uuid": self.uuid,
            "name": self.name,
            "file": self.file,
            "at": self.at.map(schema_at_value),
            "size": self.size.map(schema_size_value),
            "stroke": self.stroke.as_ref().map(schema_stroke_value),
            "fill": self.fill.as_ref().map(schema_fill_value),
            "pin_count": self.pins.len(),
            "pins": self.pins.iter().map(NspCanvasSheetPin::to_json_value).collect::<Vec<_>>(),
            "bounds": self.bounds.map(schema_bounding_box_value),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NspCanvasSheetPin {
    pub uuid: Option<String>,
    pub name: String,
    pub pin_type: String,
    pub at: Option<NspAt>,
    pub effects: Option<NspTextEffects>,
    pub bounds: Option<NspBoundingBox>,
}

impl NspCanvasSheetPin {
    fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "uuid": self.uuid,
            "name": self.name,
            "pin_type": self.pin_type,
            "at": self.at.map(schema_at_value),
            "effects": self.effects.as_ref().map(schema_text_effects_value),
            "bounds": self.bounds.map(schema_bounding_box_value),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NspCanvasRuleArea {
    pub uuid: Option<String>,
    pub points: Vec<NspPoint>,
    pub stroke: Option<NspStroke>,
    pub fill: Option<NspFill>,
    pub bounds: Option<NspBoundingBox>,
}

impl NspCanvasRuleArea {
    fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "uuid": self.uuid,
            "points": schema_points_value(&self.points),
            "stroke": self.stroke.as_ref().map(schema_stroke_value),
            "fill": self.fill.as_ref().map(schema_fill_value),
            "bounds": self.bounds.map(schema_bounding_box_value),
        })
    }

    pub(crate) fn hits_point(&self, point: NspPoint) -> bool {
        if schema_fill_is_solid(self.fill.as_ref())
            && schema_polygon_contains_point(&self.points, point)
        {
            return true;
        }
        schema_closed_polyline_hits_point(&self.points, self.stroke.as_ref(), point)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NspCanvasDirectiveLabel {
    pub uuid: Option<String>,
    pub text: String,
    pub at: Option<NspAt>,
    pub length: Option<f64>,
    pub shape: Option<String>,
    pub effects: Option<NspTextEffects>,
    pub properties: Vec<NspProperty>,
    pub bounds: Option<NspBoundingBox>,
}

impl NspCanvasDirectiveLabel {
    fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "uuid": self.uuid,
            "text": self.text,
            "at": self.at.map(schema_at_value),
            "length": self.length,
            "shape": self.shape,
            "effects": self.effects.as_ref().map(schema_text_effects_value),
            "properties": self.properties.iter().map(schema_property_value).collect::<Vec<_>>(),
            "bounds": self.bounds.map(schema_bounding_box_value),
        })
    }
}


// NspCanvasGraphic enum and impl (arc, bezier, polyline, etc).
include!("canvas_items_graphic_impl.rs");

// Simple leaf canvas types (image, table, pin, wire, bus, label, text, etc).
include!("canvas_items_leaf_impl.rs");

// Canvas item bounds calculation helpers.
include!("canvas_items_bounds_impl.rs");
