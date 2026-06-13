#[derive(Debug, Clone, PartialEq)]
pub enum NspCanvasGraphic {
    Polyline {
        uuid: Option<String>,
        points: Vec<NspPoint>,
        stroke: Option<NspStroke>,
        fill: Option<NspFill>,
    },
    Bezier {
        uuid: Option<String>,
        points: Vec<NspPoint>,
        stroke: Option<NspStroke>,
        fill: Option<NspFill>,
    },
    Rectangle {
        uuid: Option<String>,
        start: NspPoint,
        end: NspPoint,
        stroke: Option<NspStroke>,
        fill: Option<NspFill>,
    },
    Circle {
        uuid: Option<String>,
        center: NspPoint,
        radius: f64,
        stroke: Option<NspStroke>,
        fill: Option<NspFill>,
    },
    Arc {
        uuid: Option<String>,
        start: NspPoint,
        mid: Option<NspPoint>,
        end: NspPoint,
        stroke: Option<NspStroke>,
        fill: Option<NspFill>,
    },
    Text {
        uuid: Option<String>,
        text: String,
        at: Option<NspAt>,
        effects: Option<NspTextEffects>,
        stroke: Option<NspStroke>,
        fill: Option<NspFill>,
    },
}

impl NspCanvasGraphic {
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
                "points": schema_points_value(points),
                "stroke": stroke.as_ref().map(schema_stroke_value),
                "fill": fill.as_ref().map(schema_fill_value),
                "bounds": self.bounds().map(schema_bounding_box_value),
            }),
            Self::Bezier {
                uuid,
                points,
                stroke,
                fill,
            } => serde_json::json!({
                "kind": "bezier",
                "uuid": uuid,
                "points": schema_points_value(points),
                "stroke": stroke.as_ref().map(schema_stroke_value),
                "fill": fill.as_ref().map(schema_fill_value),
                "bounds": self.bounds().map(schema_bounding_box_value),
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
                "start": schema_point_value(*start),
                "end": schema_point_value(*end),
                "stroke": stroke.as_ref().map(schema_stroke_value),
                "fill": fill.as_ref().map(schema_fill_value),
                "bounds": self.bounds().map(schema_bounding_box_value),
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
                "center": schema_point_value(*center),
                "radius": radius,
                "stroke": stroke.as_ref().map(schema_stroke_value),
                "fill": fill.as_ref().map(schema_fill_value),
                "bounds": self.bounds().map(schema_bounding_box_value),
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
                "start": schema_point_value(*start),
                "mid": mid.map(schema_point_value),
                "end": schema_point_value(*end),
                "stroke": stroke.as_ref().map(schema_stroke_value),
                "fill": fill.as_ref().map(schema_fill_value),
                "bounds": self.bounds().map(schema_bounding_box_value),
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
                "at": at.map(schema_at_value),
                "effects": effects.as_ref().map(schema_text_effects_value),
                "stroke": stroke.as_ref().map(schema_stroke_value),
                "fill": fill.as_ref().map(schema_fill_value),
                "bounds": self.bounds().map(schema_bounding_box_value),
            }),
        }
    }

    pub(crate) fn with_style(
        mut self,
        stroke: Option<NspStroke>,
        fill: Option<NspFill>,
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

    pub(crate) fn hits_point(&self, point: NspPoint) -> bool {
        match self {
            Self::Polyline { points, stroke, .. } => {
                schema_polyline_hits_point(points, stroke.as_ref(), point)
            }
            Self::Bezier { points, stroke, .. } => {
                schema_bezier_hits_point(points, stroke.as_ref(), point)
            }
            Self::Rectangle {
                start,
                end,
                stroke,
                fill,
                ..
            } => schema_rectangle_hits_point(*start, *end, stroke.as_ref(), fill.as_ref(), point),
            Self::Circle {
                center,
                radius,
                stroke,
                fill,
                ..
            } => schema_circle_hits_point(*center, *radius, stroke.as_ref(), fill.as_ref(), point),
            Self::Arc {
                start,
                mid,
                end,
                stroke,
                ..
            } => schema_arc_hits_point(*start, *mid, *end, stroke.as_ref(), point),
            Self::Text {
                text, at, effects, ..
            } => schema_text_bounds(text, *at, effects.as_ref())
                .is_some_and(|bounds| bounds.contains(point)),
        }
    }

    pub(crate) fn include_in_bounds(&self, bounds: &mut NspBoundingBoxBuilder) {
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
                bounds.include(NspPoint {
                    x: center.x - radius,
                    y: center.y - radius,
                });
                bounds.include(NspPoint {
                    x: center.x + radius,
                    y: center.y + radius,
                });
            }
            Self::Arc {
                start, mid, end, ..
            } => {
                for point in sample_arc_points(*start, *mid, *end) {
                    bounds.include(point);
                }
            }
            Self::Text {
                text, at, effects, ..
            } => {
                if let Some(text_bounds) = schema_text_bounds(text, *at, effects.as_ref()) {
                    bounds.include_box(text_bounds);
                }
            }
        }
    }

    pub fn bounds(&self) -> Option<NspBoundingBox> {
        match self {
            Self::Polyline { points, .. } | Self::Bezier { points, .. } => {
                schema_points_bounds(points, SCHEMA_CANVAS_LINE_BOUNDS_PADDING)
            }
            Self::Rectangle { start, end, .. } => {
                schema_points_bounds(&[*start, *end], SCHEMA_CANVAS_LINE_BOUNDS_PADDING)
            }
            Self::Circle { center, radius, .. } => Some(NspBoundingBox {
                min: NspPoint {
                    x: center.x - radius,
                    y: center.y - radius,
                },
                max: NspPoint {
                    x: center.x + radius,
                    y: center.y + radius,
                },
            }),
            Self::Arc {
                start, mid, end, ..
            } => schema_points_bounds(
                &sample_arc_points(*start, *mid, *end),
                SCHEMA_CANVAS_LINE_BOUNDS_PADDING,
            ),
            Self::Text {
                text, at, effects, ..
            } => schema_text_bounds(text, *at, effects.as_ref()),
        }
    }
}

