use crate::canvas::KicadCanvasGraphic;
use crate::coordinates::{
    KicadAt, KicadPoint, parse_at, parse_points, parse_xy, write_points_sexpr,
};
use crate::geometry::KicadBoundingBoxBuilder;
use crate::sexpr::{
    Sexp, atom_text, child, child_value, format_number, head, list_items, list_value, sexpr_string,
};
use crate::style::{
    KicadFill, KicadStroke, KicadTextEffects, parse_fill, parse_stroke, parse_text_effects,
    write_inline_fill, write_inline_optional_fill, write_inline_stroke, write_inline_text_effects,
    write_optional_bool_sexpr,
};
use crate::transform::{transform_local_at, transform_local_point};
use crate::{parse_kicad_bool_value, parse_optional_bool_child};

#[derive(Debug, Clone, PartialEq)]
pub struct KicadSymbolGraphic {
    pub graphic: KicadGraphic,
    pub unit: u32,
    pub body_style: u32,
    pub private: bool,
    pub stroke: Option<KicadStroke>,
    pub fill: Option<KicadFill>,
    pub uuid: Option<String>,
    pub locked: Option<bool>,
}

impl KicadSymbolGraphic {
    pub(crate) fn include_in_bounds(&self, bounds: &mut KicadBoundingBoxBuilder) {
        self.graphic.include_in_bounds(bounds);
    }

    pub(crate) fn transformed(
        &self,
        symbol_at: KicadAt,
        mirror: Option<&str>,
    ) -> KicadCanvasGraphic {
        self.graphic
            .transformed(symbol_at, mirror)
            .with_uuid(self.uuid.clone())
            .with_style(self.stroke.clone(), self.fill.clone())
    }

    pub(crate) fn write_symbol_graphic_sexpr(&self, output: &mut String, indent: usize) {
        self.graphic.write_symbol_graphic_sexpr(
            output,
            indent,
            KicadSymbolGraphicFormat {
                private: self.private,
                stroke: self.stroke.as_ref(),
                fill: self.fill.as_ref(),
                uuid: self.uuid.as_deref(),
                locked: self.locked,
            },
        );
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct KicadSymbolGraphicFormat<'a> {
    private: bool,
    stroke: Option<&'a KicadStroke>,
    fill: Option<&'a KicadFill>,
    uuid: Option<&'a str>,
    locked: Option<bool>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum KicadGraphic {
    Polyline {
        points: Vec<KicadPoint>,
    },
    Bezier {
        points: Vec<KicadPoint>,
    },
    Rectangle {
        start: KicadPoint,
        end: KicadPoint,
    },
    Circle {
        center: KicadPoint,
        radius: f64,
    },
    Arc {
        start: KicadPoint,
        mid: Option<KicadPoint>,
        end: KicadPoint,
    },
    Text {
        text: String,
        at: Option<KicadAt>,
        effects: Option<KicadTextEffects>,
    },
}

impl KicadGraphic {
    pub(crate) fn include_in_bounds(&self, bounds: &mut KicadBoundingBoxBuilder) {
        match self {
            Self::Polyline { points } => {
                for point in points {
                    bounds.include(*point);
                }
            }
            Self::Bezier { points } => {
                for point in points {
                    bounds.include(*point);
                }
            }
            Self::Rectangle { start, end } => {
                bounds.include(*start);
                bounds.include(*end);
            }
            Self::Circle { center, radius } => {
                bounds.include(KicadPoint {
                    x: center.x - radius,
                    y: center.y - radius,
                });
                bounds.include(KicadPoint {
                    x: center.x + radius,
                    y: center.y + radius,
                });
            }
            Self::Arc { start, mid, end } => {
                bounds.include(*start);
                if let Some(mid) = mid {
                    bounds.include(*mid);
                }
                bounds.include(*end);
            }
            Self::Text { at, .. } => {
                if let Some(at) = at {
                    bounds.include(at.point());
                }
            }
        }
    }

    pub(crate) fn transformed(
        &self,
        symbol_at: KicadAt,
        mirror: Option<&str>,
    ) -> KicadCanvasGraphic {
        match self {
            Self::Polyline { points } => KicadCanvasGraphic::Polyline {
                uuid: None,
                points: points
                    .iter()
                    .map(|point| transform_local_point(*point, symbol_at, mirror))
                    .collect(),
                stroke: None,
                fill: None,
            },
            Self::Bezier { points } => KicadCanvasGraphic::Bezier {
                uuid: None,
                points: points
                    .iter()
                    .map(|point| transform_local_point(*point, symbol_at, mirror))
                    .collect(),
                stroke: None,
                fill: None,
            },
            Self::Rectangle { start, end } => KicadCanvasGraphic::Rectangle {
                uuid: None,
                start: transform_local_point(*start, symbol_at, mirror),
                end: transform_local_point(*end, symbol_at, mirror),
                stroke: None,
                fill: None,
            },
            Self::Circle { center, radius } => KicadCanvasGraphic::Circle {
                uuid: None,
                center: transform_local_point(*center, symbol_at, mirror),
                radius: *radius,
                stroke: None,
                fill: None,
            },
            Self::Arc { start, mid, end } => KicadCanvasGraphic::Arc {
                uuid: None,
                start: transform_local_point(*start, symbol_at, mirror),
                mid: mid.map(|point| transform_local_point(point, symbol_at, mirror)),
                end: transform_local_point(*end, symbol_at, mirror),
                stroke: None,
                fill: None,
            },
            Self::Text { text, at, effects } => KicadCanvasGraphic::Text {
                uuid: None,
                text: text.clone(),
                at: at.map(|at| transform_local_at(at, symbol_at, mirror)),
                effects: effects.clone(),
                stroke: None,
                fill: None,
            },
        }
    }

    pub(crate) fn to_canvas_graphic(&self) -> KicadCanvasGraphic {
        match self {
            Self::Polyline { points } => KicadCanvasGraphic::Polyline {
                uuid: None,
                points: points.clone(),
                stroke: None,
                fill: None,
            },
            Self::Bezier { points } => KicadCanvasGraphic::Bezier {
                uuid: None,
                points: points.clone(),
                stroke: None,
                fill: None,
            },
            Self::Rectangle { start, end } => KicadCanvasGraphic::Rectangle {
                uuid: None,
                start: *start,
                end: *end,
                stroke: None,
                fill: None,
            },
            Self::Circle { center, radius } => KicadCanvasGraphic::Circle {
                uuid: None,
                center: *center,
                radius: *radius,
                stroke: None,
                fill: None,
            },
            Self::Arc { start, mid, end } => KicadCanvasGraphic::Arc {
                uuid: None,
                start: *start,
                mid: *mid,
                end: *end,
                stroke: None,
                fill: None,
            },
            Self::Text { text, at, effects } => KicadCanvasGraphic::Text {
                uuid: None,
                text: text.clone(),
                at: *at,
                effects: effects.clone(),
                stroke: None,
                fill: None,
            },
        }
    }

    pub(crate) fn write_symbol_graphic_sexpr(
        &self,
        output: &mut String,
        indent: usize,
        format: KicadSymbolGraphicFormat<'_>,
    ) {
        let pad = " ".repeat(indent);
        let private = if format.private { " private" } else { "" };
        match self {
            Self::Polyline { points } => {
                output.push_str(&format!("{}(polyline{}", pad, private));
                write_points_sexpr(output, points);
                write_inline_stroke(output, format.stroke, 0.254);
                write_inline_fill(output, format.fill);
                write_symbol_graphic_metadata(output, format.uuid, format.locked);
                output.push_str(")\n");
            }
            Self::Bezier { points } => {
                output.push_str(&format!("{}(bezier{}", pad, private));
                write_points_sexpr(output, points);
                write_inline_stroke(output, format.stroke, 0.254);
                write_inline_fill(output, format.fill);
                write_symbol_graphic_metadata(output, format.uuid, format.locked);
                output.push_str(")\n");
            }
            Self::Rectangle { start, end } => {
                output.push_str(&format!(
                    "{}(rectangle{} (start {} {}) (end {} {})",
                    pad,
                    private,
                    format_number(start.x),
                    format_number(start.y),
                    format_number(end.x),
                    format_number(end.y)
                ));
                write_inline_stroke(output, format.stroke, 0.254);
                write_inline_fill(output, format.fill);
                write_symbol_graphic_metadata(output, format.uuid, format.locked);
                output.push_str(")\n");
            }
            Self::Circle { center, radius } => {
                output.push_str(&format!(
                    "{}(circle{} (center {} {}) (radius {})",
                    pad,
                    private,
                    format_number(center.x),
                    format_number(center.y),
                    format_number(*radius)
                ));
                write_inline_stroke(output, format.stroke, 0.254);
                write_inline_fill(output, format.fill);
                write_symbol_graphic_metadata(output, format.uuid, format.locked);
                output.push_str(")\n");
            }
            Self::Arc { start, mid, end } => {
                let mid = mid.unwrap_or(KicadPoint {
                    x: (start.x + end.x) / 2.0,
                    y: (start.y + end.y) / 2.0,
                });
                output.push_str(&format!(
                    "{}(arc{} (start {} {}) (mid {} {}) (end {} {})",
                    pad,
                    private,
                    format_number(start.x),
                    format_number(start.y),
                    format_number(mid.x),
                    format_number(mid.y),
                    format_number(end.x),
                    format_number(end.y)
                ));
                write_inline_stroke(output, format.stroke, 0.254);
                write_inline_fill(output, format.fill);
                write_symbol_graphic_metadata(output, format.uuid, format.locked);
                output.push_str(")\n");
            }
            Self::Text { text, at, effects } => {
                output.push_str(&format!("{}(text{} {}", pad, private, sexpr_string(text)));
                if let Some(at) = at {
                    output.push_str(&format!(
                        " (at {} {} {})",
                        format_number(at.x),
                        format_number(at.y),
                        format_number(at.rotation)
                    ));
                }
                write_inline_text_effects(output, effects.as_ref());
                write_symbol_graphic_metadata(output, format.uuid, format.locked);
                output.push_str(")\n");
            }
        }
    }
}

fn write_symbol_graphic_metadata(output: &mut String, uuid: Option<&str>, locked: Option<bool>) {
    if let Some(uuid) = uuid {
        output.push_str(&format!(" (uuid {})", sexpr_string(uuid)));
    }
    if locked == Some(true) {
        output.push_str(" (locked yes)");
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadSchematicGraphic {
    pub graphic: KicadGraphic,
    pub uuid: Option<String>,
    pub stroke: Option<KicadStroke>,
    pub fill: Option<KicadFill>,
    pub locked: Option<bool>,
}

impl KicadSchematicGraphic {
    pub(crate) fn to_canvas_graphic(&self) -> KicadCanvasGraphic {
        self.graphic
            .to_canvas_graphic()
            .with_uuid(self.uuid.clone())
            .with_style(self.stroke.clone(), self.fill.clone())
    }

    pub(crate) fn write_schematic_graphic_sexpr(&self, output: &mut String, indent: usize) {
        let pad = " ".repeat(indent);
        match &self.graphic {
            KicadGraphic::Polyline { points } => {
                output.push_str(&format!("{}(polyline", pad));
                write_points_sexpr(output, points);
                write_inline_stroke(output, self.stroke.as_ref(), 0.0);
                write_inline_optional_fill(output, self.fill.as_ref());
                self.write_uuid(output);
                self.write_locked(output);
                output.push_str(")\n");
            }
            KicadGraphic::Bezier { points } => {
                output.push_str(&format!("{}(bezier", pad));
                write_points_sexpr(output, points);
                write_inline_stroke(output, self.stroke.as_ref(), 0.0);
                write_inline_fill(output, self.fill.as_ref());
                self.write_uuid(output);
                self.write_locked(output);
                output.push_str(")\n");
            }
            KicadGraphic::Rectangle { start, end } => {
                output.push_str(&format!(
                    "{}(rectangle (start {} {}) (end {} {})",
                    pad,
                    format_number(start.x),
                    format_number(start.y),
                    format_number(end.x),
                    format_number(end.y)
                ));
                write_inline_stroke(output, self.stroke.as_ref(), 0.0);
                write_inline_fill(output, self.fill.as_ref());
                self.write_uuid(output);
                self.write_locked(output);
                output.push_str(")\n");
            }
            KicadGraphic::Circle { center, radius } => {
                output.push_str(&format!(
                    "{}(circle (center {} {}) (radius {})",
                    pad,
                    format_number(center.x),
                    format_number(center.y),
                    format_number(*radius)
                ));
                write_inline_stroke(output, self.stroke.as_ref(), 0.0);
                write_inline_fill(output, self.fill.as_ref());
                self.write_uuid(output);
                self.write_locked(output);
                output.push_str(")\n");
            }
            KicadGraphic::Arc { start, mid, end } => {
                let mid = mid.unwrap_or(KicadPoint {
                    x: (start.x + end.x) / 2.0,
                    y: (start.y + end.y) / 2.0,
                });
                output.push_str(&format!(
                    "{}(arc (start {} {}) (mid {} {}) (end {} {})",
                    pad,
                    format_number(start.x),
                    format_number(start.y),
                    format_number(mid.x),
                    format_number(mid.y),
                    format_number(end.x),
                    format_number(end.y)
                ));
                write_inline_stroke(output, self.stroke.as_ref(), 0.0);
                write_inline_fill(output, self.fill.as_ref());
                self.write_uuid(output);
                self.write_locked(output);
                output.push_str(")\n");
            }
            KicadGraphic::Text { text, at, effects } => {
                output.push_str(&format!("{}(text {}", pad, sexpr_string(text)));
                if let Some(at) = at {
                    output.push_str(&format!(
                        " (at {} {} {})",
                        format_number(at.x),
                        format_number(at.y),
                        format_number(at.rotation)
                    ));
                }
                write_inline_text_effects(output, effects.as_ref());
                self.write_uuid(output);
                self.write_locked(output);
                output.push_str(")\n");
            }
        }
    }

    fn write_uuid(&self, output: &mut String) {
        if let Some(uuid) = &self.uuid {
            output.push_str(&format!(" (uuid {})", sexpr_string(uuid)));
        }
    }

    fn write_locked(&self, output: &mut String) {
        if self.locked == Some(true) {
            output.push_str(" (locked yes)");
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadRuleArea {
    pub points: Vec<KicadPoint>,
    pub stroke: Option<KicadStroke>,
    pub fill: Option<KicadFill>,
    pub uuid: Option<String>,
    pub locked: Option<bool>,
    pub exclude_from_sim: Option<bool>,
    pub in_bom: Option<bool>,
    pub on_board: Option<bool>,
    pub dnp: Option<bool>,
}

impl KicadRuleArea {
    pub(crate) fn write_rule_area_sexpr(&self, output: &mut String, indent: usize) {
        let pad = " ".repeat(indent);
        output.push_str(&format!("{}(rule_area\n", pad));
        if let Some(locked) = self.locked {
            output.push_str(&format!(
                "{}  (locked {})\n",
                pad,
                if locked { "yes" } else { "no" }
            ));
        }
        write_optional_bool_sexpr(
            output,
            indent + 2,
            "exclude_from_sim",
            self.exclude_from_sim,
        );
        write_optional_bool_sexpr(output, indent + 2, "in_bom", self.in_bom);
        write_optional_bool_sexpr(output, indent + 2, "on_board", self.on_board);
        write_optional_bool_sexpr(output, indent + 2, "dnp", self.dnp);
        output.push_str(&format!("{}  (polyline", pad));
        write_points_sexpr(output, &self.points);
        write_inline_stroke(output, self.stroke.as_ref(), 0.0);
        write_inline_fill(output, self.fill.as_ref());
        if let Some(uuid) = &self.uuid {
            output.push_str(&format!(" (uuid {})", sexpr_string(uuid)));
        }
        output.push_str(")\n");
        output.push_str(&format!("{})\n", pad));
    }
}

pub(crate) fn parse_schematic_graphic(node: &Sexp) -> Option<KicadSchematicGraphic> {
    match head(node)? {
        "polyline" | "bezier" | "rectangle" | "circle" | "arc" => {
            let items = list_items(node);
            Some(KicadSchematicGraphic {
                graphic: parse_graphic(node)?,
                uuid: child_value(items, "uuid"),
                stroke: child(items, "stroke").map(parse_stroke),
                fill: child(items, "fill").map(parse_fill),
                locked: parse_optional_bool_child(items, "locked"),
            })
        }
        _ => None,
    }
}

pub(crate) fn parse_rule_area(node: &Sexp) -> Option<KicadRuleArea> {
    let items = list_items(node);
    let polyline = child(items, "polyline")?;
    let polyline_items = list_items(polyline);
    Some(KicadRuleArea {
        points: child(polyline_items, "pts")
            .map(parse_points)
            .unwrap_or_default(),
        stroke: child(polyline_items, "stroke").map(parse_stroke),
        fill: child(polyline_items, "fill").map(parse_fill),
        uuid: child_value(polyline_items, "uuid"),
        locked: parse_optional_bool_child(items, "locked"),
        exclude_from_sim: child_value(items, "exclude_from_sim").and_then(parse_kicad_bool_value),
        in_bom: child_value(items, "in_bom").and_then(parse_kicad_bool_value),
        on_board: child_value(items, "on_board").and_then(parse_kicad_bool_value),
        dnp: child_value(items, "dnp").and_then(parse_kicad_bool_value),
    })
}

pub(crate) fn parse_symbol_graphic(node: &Sexp) -> Option<KicadSymbolGraphic> {
    match head(node)? {
        "polyline" | "bezier" | "rectangle" | "circle" | "arc" | "text" => {
            let items = list_items(node);
            Some(KicadSymbolGraphic {
                graphic: parse_graphic(node)?,
                unit: 0,
                body_style: 0,
                private: items
                    .iter()
                    .skip(1)
                    .any(|item| atom_text(item) == Some("private")),
                stroke: child(items, "stroke").map(parse_stroke),
                fill: child(items, "fill").map(parse_fill),
                uuid: child_value(items, "uuid"),
                locked: parse_optional_bool_child(items, "locked"),
            })
        }
        _ => None,
    }
}

fn parse_graphic(node: &Sexp) -> Option<KicadGraphic> {
    let items = list_items(node);
    match head(node)? {
        "polyline" => {
            let points = child(items, "pts").map(parse_points).unwrap_or_default();
            (!points.is_empty()).then_some(KicadGraphic::Polyline { points })
        }
        "bezier" => {
            let points = child(items, "pts").map(parse_points).unwrap_or_default();
            (points.len() == 4).then_some(KicadGraphic::Bezier { points })
        }
        "rectangle" => Some(KicadGraphic::Rectangle {
            start: child(items, "start").and_then(parse_xy)?,
            end: child(items, "end").and_then(parse_xy)?,
        }),
        "circle" => {
            let center = child(items, "center").and_then(parse_xy)?;
            let radius = child_value(items, "radius")
                .and_then(|value| value.parse().ok())
                .or_else(|| {
                    child(items, "end")
                        .and_then(parse_xy)
                        .map(|end| ((end.x - center.x).powi(2) + (end.y - center.y).powi(2)).sqrt())
                })?;
            Some(KicadGraphic::Circle { center, radius })
        }
        "arc" => Some(KicadGraphic::Arc {
            start: child(items, "start").and_then(parse_xy)?,
            mid: child(items, "mid").and_then(parse_xy),
            end: child(items, "end").and_then(parse_xy)?,
        }),
        "text" => Some(KicadGraphic::Text {
            text: list_value(node, 1)?,
            at: child(items, "at").and_then(parse_at),
            effects: child(items, "effects").map(parse_text_effects),
        }),
        _ => None,
    }
}
