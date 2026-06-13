//! schema graphic elements — polylines, rectangles, arcs, circles, and filled polygons.

use crate::canvas::NspCanvasGraphic;
use crate::coordinates::{NspAt, NspPoint, parse_at, parse_points, parse_xy, write_points_sexpr};
use crate::geometry::NspBoundingBoxBuilder;
use crate::sexpr::{
    Sexp, atom_text, child, child_value, format_number, head, list_items, list_value, sexpr_string,
};
use crate::style::{
    NspFill, NspStroke, NspTextEffects, parse_fill, parse_stroke, parse_text_effects,
    write_inline_fill, write_inline_optional_fill, write_inline_stroke, write_inline_text_effects,
    write_optional_bool_sexpr,
};
use crate::transform::{transform_local_at, transform_local_point};
use crate::util::{parse_bool_value, parse_optional_bool_child};

#[derive(Debug, Clone, PartialEq)]
pub struct NspSymbolGraphic {
    pub graphic: NspGraphic,
    pub unit: u32,
    pub body_style: u32,
    pub private: bool,
    pub stroke: Option<NspStroke>,
    pub fill: Option<NspFill>,
    pub uuid: Option<String>,
    pub locked: Option<bool>,
}

impl NspSymbolGraphic {
    /// include in bounds。
    pub(crate) fn include_in_bounds(&self, bounds: &mut NspBoundingBoxBuilder) {
        self.graphic.include_in_bounds(bounds);
    }

    /// transformed。
    pub(crate) fn transformed(&self, symbol_at: NspAt, mirror: Option<&str>) -> NspCanvasGraphic {
        self.graphic
            .transformed(symbol_at, mirror)
            .with_uuid(self.uuid.clone())
            .with_style(self.stroke.clone(), self.fill.clone())
    }

    /// write symbol graphic sexpr。
    pub(crate) fn write_symbol_graphic_sexpr(&self, output: &mut String, indent: usize) {
        self.graphic.write_symbol_graphic_sexpr(
            output,
            indent,
            NspSymbolGraphicFormat {
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
pub(crate) struct NspSymbolGraphicFormat<'a> {
    private: bool,
    stroke: Option<&'a NspStroke>,
    fill: Option<&'a NspFill>,
    uuid: Option<&'a str>,
    locked: Option<bool>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NspGraphic {
    Polyline {
        points: Vec<NspPoint>,
    },
    Bezier {
        points: Vec<NspPoint>,
    },
    Rectangle {
        start: NspPoint,
        end: NspPoint,
    },
    Circle {
        center: NspPoint,
        radius: f64,
    },
    Arc {
        start: NspPoint,
        mid: Option<NspPoint>,
        end: NspPoint,
    },
    Text {
        text: String,
        at: Option<NspAt>,
        effects: Option<NspTextEffects>,
    },
}

impl NspGraphic {
    /// include in bounds。
    pub(crate) fn include_in_bounds(&self, bounds: &mut NspBoundingBoxBuilder) {
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
                bounds.include(NspPoint {
                    x: center.x - radius,
                    y: center.y - radius,
                });
                bounds.include(NspPoint {
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

    /// transformed。
    pub(crate) fn transformed(&self, symbol_at: NspAt, mirror: Option<&str>) -> NspCanvasGraphic {
        match self {
            Self::Polyline { points } => NspCanvasGraphic::Polyline {
                uuid: None,
                points: points
                    .iter()
                    .map(|point| transform_local_point(*point, symbol_at, mirror))
                    .collect(),
                stroke: None,
                fill: None,
            },
            Self::Bezier { points } => NspCanvasGraphic::Bezier {
                uuid: None,
                points: points
                    .iter()
                    .map(|point| transform_local_point(*point, symbol_at, mirror))
                    .collect(),
                stroke: None,
                fill: None,
            },
            Self::Rectangle { start, end } => NspCanvasGraphic::Rectangle {
                uuid: None,
                start: transform_local_point(*start, symbol_at, mirror),
                end: transform_local_point(*end, symbol_at, mirror),
                stroke: None,
                fill: None,
            },
            Self::Circle { center, radius } => NspCanvasGraphic::Circle {
                uuid: None,
                center: transform_local_point(*center, symbol_at, mirror),
                radius: *radius,
                stroke: None,
                fill: None,
            },
            Self::Arc { start, mid, end } => NspCanvasGraphic::Arc {
                uuid: None,
                start: transform_local_point(*start, symbol_at, mirror),
                mid: mid.map(|point| transform_local_point(point, symbol_at, mirror)),
                end: transform_local_point(*end, symbol_at, mirror),
                stroke: None,
                fill: None,
            },
            Self::Text { text, at, effects } => NspCanvasGraphic::Text {
                uuid: None,
                text: text.clone(),
                at: at.map(|at| transform_local_at(at, symbol_at, mirror)),
                effects: effects.clone(),
                stroke: None,
                fill: None,
            },
        }
    }

    /// to canvas graphic。
    pub(crate) fn to_canvas_graphic(&self) -> NspCanvasGraphic {
        match self {
            Self::Polyline { points } => NspCanvasGraphic::Polyline {
                uuid: None,
                points: points.clone(),
                stroke: None,
                fill: None,
            },
            Self::Bezier { points } => NspCanvasGraphic::Bezier {
                uuid: None,
                points: points.clone(),
                stroke: None,
                fill: None,
            },
            Self::Rectangle { start, end } => NspCanvasGraphic::Rectangle {
                uuid: None,
                start: *start,
                end: *end,
                stroke: None,
                fill: None,
            },
            Self::Circle { center, radius } => NspCanvasGraphic::Circle {
                uuid: None,
                center: *center,
                radius: *radius,
                stroke: None,
                fill: None,
            },
            Self::Arc { start, mid, end } => NspCanvasGraphic::Arc {
                uuid: None,
                start: *start,
                mid: *mid,
                end: *end,
                stroke: None,
                fill: None,
            },
            Self::Text { text, at, effects } => NspCanvasGraphic::Text {
                uuid: None,
                text: text.clone(),
                at: *at,
                effects: effects.clone(),
                stroke: None,
                fill: None,
            },
        }
    }

    /// write symbol graphic sexpr。
    pub(crate) fn write_symbol_graphic_sexpr(
        &self,
        output: &mut String,
        indent: usize,
        format: NspSymbolGraphicFormat<'_>,
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
                let mid = mid.unwrap_or(NspPoint {
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
pub struct NspSchematicGraphic {
    pub graphic: NspGraphic,
    pub uuid: Option<String>,
    pub stroke: Option<NspStroke>,
    pub fill: Option<NspFill>,
    pub locked: Option<bool>,
}

impl NspSchematicGraphic {
    /// to canvas graphic。
    pub(crate) fn to_canvas_graphic(&self) -> NspCanvasGraphic {
        self.graphic
            .to_canvas_graphic()
            .with_uuid(self.uuid.clone())
            .with_style(self.stroke.clone(), self.fill.clone())
    }

    /// write schematic graphic sexpr。
    pub(crate) fn write_schematic_graphic_sexpr(&self, output: &mut String, indent: usize) {
        let pad = " ".repeat(indent);
        match &self.graphic {
            NspGraphic::Polyline { points } => {
                output.push_str(&format!("{}(polyline", pad));
                write_points_sexpr(output, points);
                write_inline_stroke(output, self.stroke.as_ref(), 0.0);
                write_inline_optional_fill(output, self.fill.as_ref());
                self.write_uuid(output);
                self.write_locked(output);
                output.push_str(")\n");
            }
            NspGraphic::Bezier { points } => {
                output.push_str(&format!("{}(bezier", pad));
                write_points_sexpr(output, points);
                write_inline_stroke(output, self.stroke.as_ref(), 0.0);
                write_inline_fill(output, self.fill.as_ref());
                self.write_uuid(output);
                self.write_locked(output);
                output.push_str(")\n");
            }
            NspGraphic::Rectangle { start, end } => {
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
            NspGraphic::Circle { center, radius } => {
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
            NspGraphic::Arc { start, mid, end } => {
                let mid = mid.unwrap_or(NspPoint {
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
            NspGraphic::Text { text, at, effects } => {
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
pub struct NspRuleArea {
    pub points: Vec<NspPoint>,
    pub stroke: Option<NspStroke>,
    pub fill: Option<NspFill>,
    pub uuid: Option<String>,
    pub locked: Option<bool>,
    pub exclude_from_sim: Option<bool>,
    pub in_bom: Option<bool>,
    pub on_board: Option<bool>,
    pub dnp: Option<bool>,
}

impl NspRuleArea {
    /// write rule area sexpr。
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

/// parse schematic graphic。
pub(crate) fn parse_schematic_graphic(node: &Sexp) -> Option<NspSchematicGraphic> {
    match head(node)? {
        "polyline" | "bezier" | "rectangle" | "circle" | "arc" => {
            let items = list_items(node);
            Some(NspSchematicGraphic {
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

/// parse rule area。
pub(crate) fn parse_rule_area(node: &Sexp) -> Option<NspRuleArea> {
    let items = list_items(node);
    let polyline = child(items, "polyline")?;
    let polyline_items = list_items(polyline);
    Some(NspRuleArea {
        points: child(polyline_items, "pts")
            .map(parse_points)
            .unwrap_or_default(),
        stroke: child(polyline_items, "stroke").map(parse_stroke),
        fill: child(polyline_items, "fill").map(parse_fill),
        uuid: child_value(polyline_items, "uuid"),
        locked: parse_optional_bool_child(items, "locked"),
        exclude_from_sim: child_value(items, "exclude_from_sim").and_then(parse_bool_value),
        in_bom: child_value(items, "in_bom").and_then(parse_bool_value),
        on_board: child_value(items, "on_board").and_then(parse_bool_value),
        dnp: child_value(items, "dnp").and_then(parse_bool_value),
    })
}

/// parse symbol graphic。
pub(crate) fn parse_symbol_graphic(node: &Sexp) -> Option<NspSymbolGraphic> {
    match head(node)? {
        "polyline" | "bezier" | "rectangle" | "circle" | "arc" | "text" => {
            let items = list_items(node);
            Some(NspSymbolGraphic {
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

fn parse_graphic(node: &Sexp) -> Option<NspGraphic> {
    let items = list_items(node);
    match head(node)? {
        "polyline" => {
            let points = child(items, "pts").map(parse_points).unwrap_or_default();
            (!points.is_empty()).then_some(NspGraphic::Polyline { points })
        }
        "bezier" => {
            let points = child(items, "pts").map(parse_points).unwrap_or_default();
            (points.len() == 4).then_some(NspGraphic::Bezier { points })
        }
        "rectangle" => Some(NspGraphic::Rectangle {
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
            Some(NspGraphic::Circle { center, radius })
        }
        "arc" => Some(NspGraphic::Arc {
            start: child(items, "start").and_then(parse_xy)?,
            mid: child(items, "mid").and_then(parse_xy),
            end: child(items, "end").and_then(parse_xy)?,
        }),
        "text" => Some(NspGraphic::Text {
            text: list_value(node, 1)?,
            at: child(items, "at").and_then(parse_at),
            effects: child(items, "effects").map(parse_text_effects),
        }),
        _ => None,
    }
}
