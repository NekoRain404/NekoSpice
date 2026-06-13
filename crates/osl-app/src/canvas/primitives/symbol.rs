//! 符号渲染图元。绘制引脚、轮廓和标号文本。
//!
use super::super::colors::SchematicColors;
use super::draw_rotated_text;
use super::{draw_line, draw_polyline, quadratic_bezier_sample};
use crate::viewport::CanvasViewport;
use eframe::egui::{self, Color32, Pos2, Rect, Stroke, StrokeKind};
/// Resolve font size from KiCad text effects, with a minimum of 6pt.
fn resolve_font_size(effects: &Option<osl_kicad::KicadTextEffects>) -> f32 {
    effects
        .as_ref()
        .and_then(|e| e.font_size)
        .map(|size| size.width as f32)
        .unwrap_or(12.0)
        .max(6.0)
}

use osl_kicad::{KicadCanvasGraphic, sample_kicad_arc_points};

/// Returns at least `min_width` pixels for visibility.
fn resolve_stroke_width(
    stroke: &Option<osl_kicad::KicadStroke>,
    viewport: CanvasViewport,
    min_width: f32,
) -> f32 {
    stroke
        .as_ref()
        .and_then(|s| s.width)
        .filter(|w| *w > 0.0)
        .map(|w| (w as f32 * viewport.zoom).max(min_width))
        .unwrap_or(min_width)
}

/// Draw a KiCad canvas graphic element (polyline, bezier, rectangle, circle, arc, text).
///
/// Supports solid fills via KiCad fill data. Filled rectangles and circles
/// get a translucent background fill matching the KiCad rendering style.
/// Stroke widths are resolved from KiCad data when available.
pub(crate) fn draw_graphic(
    painter: &egui::Painter,
    rect: Rect,
    viewport: CanvasViewport,
    graphic: &KicadCanvasGraphic,
    color: Color32,
) {
    match graphic {
        KicadCanvasGraphic::Polyline {
            points,
            fill,
            stroke,
            ..
        } => {
            // Draw fill if present and closed
            if fill
                .as_ref()
                .and_then(|f| f.fill_type.as_deref())
                .is_some_and(|ft| !ft.eq_ignore_ascii_case("none"))
                && points.len() > 2
            {
                let screen_points: Vec<Pos2> = points
                    .iter()
                    .map(|p| viewport.world_to_screen(rect, *p))
                    .collect();
                painter.add(egui::Shape::convex_polygon(
                    screen_points,
                    Color32::from_rgba_premultiplied(200, 200, 200, 30),
                    egui::Stroke::NONE,
                ));
            }
            let width = resolve_stroke_width(stroke, viewport, 1.0);
            draw_polyline(painter, rect, viewport, points, false, color, width);
        }
        KicadCanvasGraphic::Bezier {
            points,
            fill,
            stroke,
            ..
        } => {
            if points.len() >= 3 {
                let sampled = quadratic_bezier_sample(points, 24);
                if fill
                    .as_ref()
                    .and_then(|f| f.fill_type.as_deref())
                    .is_some_and(|ft| !ft.eq_ignore_ascii_case("none"))
                    && sampled.len() > 2
                {
                    let screen_points: Vec<Pos2> = sampled
                        .iter()
                        .map(|p| viewport.world_to_screen(rect, *p))
                        .collect();
                    painter.add(egui::Shape::convex_polygon(
                        screen_points,
                        Color32::from_rgba_premultiplied(200, 200, 200, 30),
                        egui::Stroke::NONE,
                    ));
                }
                let width = resolve_stroke_width(stroke, viewport, 1.0);
                draw_polyline(painter, rect, viewport, &sampled, false, color, width);
            } else if !points.is_empty() {
                let width = resolve_stroke_width(stroke, viewport, 1.0);
                draw_polyline(painter, rect, viewport, points, false, color, width);
            }
        }
        KicadCanvasGraphic::Rectangle {
            start,
            end,
            fill,
            stroke,
            ..
        } => {
            let s = viewport.world_to_screen(rect, *start);
            let e = viewport.world_to_screen(rect, *end);
            let r = Rect::from_two_pos(s, e);
            // Draw fill if present
            if fill
                .as_ref()
                .and_then(|f| f.fill_type.as_deref())
                .is_some_and(|ft| !ft.eq_ignore_ascii_case("none"))
            {
                painter.rect_filled(r, 0.0, Color32::from_rgba_premultiplied(200, 200, 200, 30));
            }
            let width = resolve_stroke_width(stroke, viewport, 1.0);
            painter.rect_stroke(r, 0.0, Stroke::new(width, color), StrokeKind::Inside);
        }
        KicadCanvasGraphic::Circle {
            center,
            radius,
            fill,
            stroke,
            ..
        } => {
            let center_screen = viewport.world_to_screen(rect, *center);
            let radius_screen = (*radius as f32 * viewport.zoom).abs();
            // Draw fill if present
            if fill
                .as_ref()
                .and_then(|f| f.fill_type.as_deref())
                .is_some_and(|ft| !ft.eq_ignore_ascii_case("none"))
            {
                painter.circle_filled(
                    center_screen,
                    radius_screen,
                    Color32::from_rgba_premultiplied(200, 200, 200, 30),
                );
            }
            let width = resolve_stroke_width(stroke, viewport, 1.0);
            painter.circle_stroke(center_screen, radius_screen, Stroke::new(width, color));
        }
        KicadCanvasGraphic::Arc {
            start,
            mid,
            end,
            stroke,
            ..
        } => {
            let sampled = sample_kicad_arc_points(*start, *mid, *end);
            let width = resolve_stroke_width(stroke, viewport, 1.0);
            draw_polyline(painter, rect, viewport, &sampled, false, color, width);
        }
        KicadCanvasGraphic::Text {
            text, at, effects, ..
        } => {
            if let Some(at) = at {
                let font_size = resolve_font_size(effects);
                draw_rotated_text(painter, rect, viewport, *at, text, font_size, color);
            }
        }
    }
}

/// draw pin。
pub(crate) fn draw_pin(
    painter: &egui::Painter,
    rect: Rect,
    viewport: CanvasViewport,
    pin: &osl_kicad::KicadCanvasPin,
    sc: SchematicColors,
) {
    let color = sc.symbol_pin;
    let width = 1.5;

    // Draw the main pin line (always drawn)
    draw_line(painter, rect, viewport, pin.start, pin.end, color, width);

    // Compute direction vector from start to end (pin root to connection point)
    let dx = pin.end.x - pin.start.x;
    let dy = pin.end.y - pin.start.y;
    let dist = (dx * dx + dy * dy).sqrt();
    if dist < 1e-6 {
        return;
    }
    let nx = dx / dist;
    let ny = dy / dist;

    // Decorator radius (relative to pin length)
    let radius = (dist * 0.25).clamp(0.5, 1.5);
    let triangle_size = radius * 1.5;

    match pin.shape.as_str() {
        "inverted" => {
            // Circle at the body end (start point) indicating active-low
            let center = viewport.world_to_screen(rect, pin.start);
            let r = radius as f32 * viewport.zoom;
            painter.circle_stroke(center, r, Stroke::new(width, color));
        }
        "clock" => {
            // Triangle at the pin root pointing toward the connection
            let center = viewport.world_to_screen(rect, pin.start);
            let r = triangle_size as f32 * viewport.zoom;
            // Triangle vertices pointing in pin direction
            let tip = Pos2::new(center.x + nx as f32 * r, center.y + ny as f32 * r);
            let base1 = Pos2::new(
                center.x - ny as f32 * r * 0.5,
                center.y + nx as f32 * r * 0.5,
            );
            let base2 = Pos2::new(
                center.x + ny as f32 * r * 0.5,
                center.y - nx as f32 * r * 0.5,
            );
            painter.line_segment([tip, base1], Stroke::new(width, color));
            painter.line_segment([base1, base2], Stroke::new(width, color));
            painter.line_segment([base2, tip], Stroke::new(width, color));
        }
        "inverted_clock" => {
            // Triangle + circle
            let center = viewport.world_to_screen(rect, pin.start);
            let r = triangle_size as f32 * viewport.zoom;
            let tip = Pos2::new(center.x + nx as f32 * r, center.y + ny as f32 * r);
            let base1 = Pos2::new(
                center.x - ny as f32 * r * 0.5,
                center.y + nx as f32 * r * 0.5,
            );
            let base2 = Pos2::new(
                center.x + ny as f32 * r * 0.5,
                center.y - nx as f32 * r * 0.5,
            );
            painter.line_segment([tip, base1], Stroke::new(width, color));
            painter.line_segment([base1, base2], Stroke::new(width, color));
            painter.line_segment([base2, tip], Stroke::new(width, color));
            // Circle at body end
            let circle_r = radius as f32 * viewport.zoom;
            painter.circle_stroke(center, circle_r, Stroke::new(width, color));
        }
        "input_low" => {
            // Bar at body end
            let center = viewport.world_to_screen(rect, pin.start);
            let r = radius as f32 * viewport.zoom;
            let bar1 = Pos2::new(center.x - ny as f32 * r, center.y + nx as f32 * r);
            let bar2 = Pos2::new(center.x + ny as f32 * r, center.y - nx as f32 * r);
            painter.line_segment([bar1, bar2], Stroke::new(width, color));
        }
        "clock_low" | "falling_edge_clock" => {
            // Triangle at body end
            let center = viewport.world_to_screen(rect, pin.start);
            let r = triangle_size as f32 * viewport.zoom;
            let tip = Pos2::new(center.x + nx as f32 * r, center.y + ny as f32 * r);
            let base1 = Pos2::new(
                center.x - ny as f32 * r * 0.5,
                center.y + nx as f32 * r * 0.5,
            );
            let base2 = Pos2::new(
                center.x + ny as f32 * r * 0.5,
                center.y - nx as f32 * r * 0.5,
            );
            painter.line_segment([tip, base1], Stroke::new(width, color));
            painter.line_segment([base1, base2], Stroke::new(width, color));
            painter.line_segment([base2, tip], Stroke::new(width, color));
        }
        "non_logic" => {
            // X at body end
            let center = viewport.world_to_screen(rect, pin.start);
            let r = radius as f32 * viewport.zoom;
            let p1 = Pos2::new(center.x - r, center.y - r);
            let p2 = Pos2::new(center.x + r, center.y + r);
            let p3 = Pos2::new(center.x + r, center.y - r);
            let p4 = Pos2::new(center.x - r, center.y + r);
            painter.line_segment([p1, p2], Stroke::new(width, color));
            painter.line_segment([p3, p4], Stroke::new(width, color));
        }
        _ => {
            // "line" or unknown — already drawn as a plain line above
        }
    }
}
