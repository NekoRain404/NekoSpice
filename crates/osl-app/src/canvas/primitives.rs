// Canvas drawing primitives for KiCad schematic rendering.
// Provides grid, sheet, graphic (polyline/arc/bezier/circle/rect/text),
// wire, bus, label, junction, no-connect, and selection bounds drawing.

use crate::viewport::CanvasViewport;
use super::colors;
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Stroke, StrokeKind, Vec2};
use osl_kicad::{
    KicadBoundingBox, KicadCanvasBusEntry, KicadCanvasGraphic, KicadCanvasSheet, KicadPoint,
    sample_kicad_arc_points,
};

// ---------------------------------------------------------------------------
// Grid
// ---------------------------------------------------------------------------

/// Draw a background grid with minor and major lines.
///
/// Minor lines are drawn at every 2.54mm (100mil) step.
/// Major lines are drawn at every 12.7mm (500mil) step.
/// Major lines use a slightly darker color and thicker stroke.
pub(crate) fn draw_grid(painter: &egui::Painter, rect: Rect, viewport: CanvasViewport) {
    // Minor grid step: 2.54mm in world space
    let minor_step = (2.54 * viewport.zoom).max(4.0);
    // Major grid step: 5x minor (12.7mm)
    let major_step = minor_step * 5.0;

    let origin = rect.center() + viewport.pan;
    let minor_stroke = Stroke::new(0.5, colors::GRID_MINOR);
    let major_stroke = Stroke::new(1.0, colors::GRID_MAJOR);

    // Draw vertical lines
    let start_x = origin.x % minor_step;
    let mut x = start_x;
    while x < rect.width() {
        let screen_x = rect.left() + x;
        let is_major = (x % major_step).abs() < 0.5 || ((major_step - (x % major_step)).abs() < 0.5);
        let stroke = if is_major { major_stroke } else { minor_stroke };
        painter.line_segment(
            [
                Pos2::new(screen_x, rect.top()),
                Pos2::new(screen_x, rect.bottom()),
            ],
            stroke,
        );
        x += minor_step;
    }

    // Draw horizontal lines
    let start_y = origin.y % minor_step;
    let mut y = start_y;
    while y < rect.height() {
        let screen_y = rect.top() + y;
        let is_major = (y % major_step).abs() < 0.5 || ((major_step - (y % major_step)).abs() < 0.5);
        let stroke = if is_major { major_stroke } else { minor_stroke };
        painter.line_segment(
            [
                Pos2::new(rect.left(), screen_y),
                Pos2::new(rect.right(), screen_y),
            ],
            stroke,
        );
        y += minor_step;
    }
}

// ---------------------------------------------------------------------------
// Sheet border
// ---------------------------------------------------------------------------

/// Draw a KiCad hierarchical sheet box with name label.
pub(crate) fn draw_sheet(
    painter: &egui::Painter,
    rect: Rect,
    viewport: CanvasViewport,
    sheet: &KicadCanvasSheet,
) {
    let Some(at) = sheet.at else {
        return;
    };
    let Some(size) = sheet.size else {
        return;
    };
    let start = viewport.world_to_screen(rect, KicadPoint { x: at.x, y: at.y });
    let end = viewport.world_to_screen(
        rect,
        KicadPoint {
            x: at.x + size.width,
            y: at.y + size.height,
        },
    );
    let sheet_rect = Rect::from_two_pos(start, end);
    painter.rect_filled(sheet_rect, 0.0, colors::SHEET_FILL);
    painter.rect_stroke(
        sheet_rect,
        0.0,
        Stroke::new(1.5, colors::SHEET_BORDER),
        StrokeKind::Inside,
    );
    painter.text(
        sheet_rect.left_top() + Vec2::new(4.0, 4.0),
        Align2::LEFT_TOP,
        &sheet.name,
        FontId::monospace(12.0),
        colors::SHEET_NAME,
    );
}

// ---------------------------------------------------------------------------
// Graphic element drawing
// ---------------------------------------------------------------------------

/// Resolve font size from KiCad text effects, defaulting to 12.0.
fn resolve_font_size(effects: &Option<osl_kicad::KicadTextEffects>) -> f32 {
    effects
        .as_ref()
        .and_then(|e| e.font_size)
        .map(|size| size.width as f32)
        .unwrap_or(12.0)
        .max(6.0)
}

/// Draw a KiCad canvas graphic element (polyline, bezier, rectangle, circle, arc, text).
///
/// Supports solid fills via KiCad fill data. Filled rectangles and circles
/// get a translucent background fill matching the KiCad rendering style.
pub(crate) fn draw_graphic(
    painter: &egui::Painter,
    rect: Rect,
    viewport: CanvasViewport,
    graphic: &KicadCanvasGraphic,
    color: Color32,
) {
    match graphic {
        KicadCanvasGraphic::Polyline { points, fill, .. } => {
            // Draw fill if present and closed
            if fill.as_ref().and_then(|f| f.fill_type.as_deref()).is_some_and(|ft| !ft.eq_ignore_ascii_case("none")) && points.len() > 2 {
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
            draw_polyline(painter, rect, viewport, points, false, color, 1.5);
        }
        KicadCanvasGraphic::Bezier { points, fill, .. } => {
            if points.len() >= 3 {
                let sampled = quadratic_bezier_sample(points, 24);
                if fill.as_ref().and_then(|f| f.fill_type.as_deref()).is_some_and(|ft| !ft.eq_ignore_ascii_case("none")) && sampled.len() > 2 {
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
                draw_polyline(painter, rect, viewport, &sampled, false, color, 1.5);
            } else if !points.is_empty() {
                draw_polyline(painter, rect, viewport, points, false, color, 1.5);
            }
        }
        KicadCanvasGraphic::Rectangle { start, end, fill, .. } => {
            let s = viewport.world_to_screen(rect, *start);
            let e = viewport.world_to_screen(rect, *end);
            let r = Rect::from_two_pos(s, e);
            // Draw fill if present
            if fill.as_ref().and_then(|f| f.fill_type.as_deref()).is_some_and(|ft| !ft.eq_ignore_ascii_case("none")) {
                painter.rect_filled(r, 0.0, Color32::from_rgba_premultiplied(200, 200, 200, 30));
            }
            painter.rect_stroke(r, 0.0, Stroke::new(1.5, color), StrokeKind::Inside);
        }
        KicadCanvasGraphic::Circle { center, radius, fill, .. } => {
            let center_screen = viewport.world_to_screen(rect, *center);
            let radius_screen = (*radius as f32 * viewport.zoom).abs();
            // Draw fill if present
            if fill.as_ref().and_then(|f| f.fill_type.as_deref()).is_some_and(|ft| !ft.eq_ignore_ascii_case("none")) {
                painter.circle_filled(center_screen, radius_screen, Color32::from_rgba_premultiplied(200, 200, 200, 30));
            }
            painter.circle_stroke(center_screen, radius_screen, Stroke::new(1.5, color));
        }
        KicadCanvasGraphic::Arc {
            start, mid, end, ..
        } => {
            let sampled = sample_kicad_arc_points(*start, *mid, *end);
            draw_polyline(painter, rect, viewport, &sampled, false, color, 1.5);
        }
        KicadCanvasGraphic::Text {
            text, at, effects, ..
        } => {
            if let Some(at) = at {
                let font_size = resolve_font_size(effects);
                painter.text(
                    viewport.world_to_screen(rect, KicadPoint { x: at.x, y: at.y }),
                    Align2::LEFT_TOP,
                    text,
                    FontId::proportional(font_size),
                    color,
                );
            }
        }
    }
}



// ---------------------------------------------------------------------------
// Polyline / line / bus entry
// ---------------------------------------------------------------------------

/// Draw a polyline (open or closed) from world-space KiCad points.
pub(crate) fn draw_polyline(
    painter: &egui::Painter,
    rect: Rect,
    viewport: CanvasViewport,
    points: &[KicadPoint],
    closed: bool,
    color: Color32,
    width: f32,
) {
    for segment in points.windows(2) {
        draw_line(painter, rect, viewport, segment[0], segment[1], color, width);
    }
    if closed && points.len() > 2 && let Some(last) = points.last() {
        draw_line(painter, rect, viewport, *last, points[0], color, width);
    }
}

/// Draw a single line segment between two world-space points.
pub(crate) fn draw_line(
    painter: &egui::Painter,
    rect: Rect,
    viewport: CanvasViewport,
    start: KicadPoint,
    end: KicadPoint,
    color: Color32,
    width: f32,
) {
    painter.line_segment(
        [
            viewport.world_to_screen(rect, start),
            viewport.world_to_screen(rect, end),
        ],
        Stroke::new(width, color),
    );
}

/// Draw a bus entry line from its start point to computed end point.
pub(crate) fn draw_bus_entry(
    painter: &egui::Painter,
    rect: Rect,
    viewport: CanvasViewport,
    entry: &KicadCanvasBusEntry,
) {
    draw_line(
        painter,
        rect,
        viewport,
        entry.at,
        entry.end(),
        colors::BUS_ENTRY,
        2.0,
    );
}

/// Draw a schematic pin with its shape decoration.
///
/// KiCad pin shapes:
/// - `line`: plain straight line (default)
/// - `inverted`: line + circle at body end (active-low input)
/// - `clock`: line + triangle at pin root (clock input)
/// - `inverted_clock`: line + triangle + circle at body end
/// - `input_low`: line + bar at body end
/// - `clock_low`: line + triangle + bar at body end
/// - `falling_edge_clock`: line + triangle at body end
/// - `non_logic`: line + X at body end
pub(crate) fn draw_pin(
    painter: &egui::Painter,
    rect: Rect,
    viewport: CanvasViewport,
    pin: &osl_kicad::KicadCanvasPin,
) {
    let color = colors::SYMBOL_PIN;
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
    let radius = (dist * 0.25).min(1.5).max(0.5);
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
            let tip = Pos2::new(
                center.x + nx as f32 * r,
                center.y + ny as f32 * r,
            );
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
            let tip = Pos2::new(
                center.x + nx as f32 * r,
                center.y + ny as f32 * r,
            );
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
            let bar1 = Pos2::new(
                center.x - ny as f32 * r,
                center.y + nx as f32 * r,
            );
            let bar2 = Pos2::new(
                center.x + ny as f32 * r,
                center.y - nx as f32 * r,
            );
            painter.line_segment([bar1, bar2], Stroke::new(width, color));
        }
        "clock_low" | "falling_edge_clock" => {
            // Triangle at body end
            let center = viewport.world_to_screen(rect, pin.start);
            let r = triangle_size as f32 * viewport.zoom;
            let tip = Pos2::new(
                center.x + nx as f32 * r,
                center.y + ny as f32 * r,
            );
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
            let p1 = Pos2::new(
                center.x - r,
                center.y - r,
            );
            let p2 = Pos2::new(
                center.x + r,
                center.y + r,
            );
            let p3 = Pos2::new(
                center.x + r,
                center.y - r,
            );
            let p4 = Pos2::new(
                center.x - r,
                center.y + r,
            );
            painter.line_segment([p1, p2], Stroke::new(width, color));
            painter.line_segment([p3, p4], Stroke::new(width, color));
        }
        _ => {
            // "line" or unknown — already drawn as a plain line above
        }
    }
}

// ---------------------------------------------------------------------------
// Selection bounds
// ---------------------------------------------------------------------------

/// Draw a selection highlight rectangle around the given bounding box.
pub(crate) fn draw_bounds(
    painter: &egui::Painter,
    rect: Rect,
    viewport: CanvasViewport,
    bounds: KicadBoundingBox,
    color: Color32,
    width: f32,
) {
    let min = viewport.world_to_screen(rect, bounds.min);
    let max = viewport.world_to_screen(rect, bounds.max);
    painter.rect_stroke(
        Rect::from_two_pos(min, max),
        0.0,
        Stroke::new(width, color),
        StrokeKind::Inside,
    );
}

// ---------------------------------------------------------------------------
// Bezier approximation
// ---------------------------------------------------------------------------

/// Approximate a quadratic bezier curve with sampled points.
/// For n control points, fits consecutive quadratic bezier segments.
fn quadratic_bezier_sample(points: &[KicadPoint], segments: usize) -> Vec<KicadPoint> {
    if points.len() < 3 {
        return points.to_vec();
    }
    let mut result = Vec::with_capacity(segments + 1);
    let p0 = points[0];
    let p1 = points[1];
    let p2 = *points.last().unwrap();
    for i in 0..=segments {
        let t = i as f64 / segments as f64;
        let inv = 1.0 - t;
        let x = inv * inv * p0.x + 2.0 * inv * t * p1.x + t * t * p2.x;
        let y = inv * inv * p0.y + 2.0 * inv * t * p1.y + t * t * p2.y;
        result.push(KicadPoint { x, y });
    }
    result
}
