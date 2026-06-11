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
pub(crate) fn draw_graphic(
    painter: &egui::Painter,
    rect: Rect,
    viewport: CanvasViewport,
    graphic: &KicadCanvasGraphic,
    color: Color32,
) {
    match graphic {
        KicadCanvasGraphic::Polyline { points, .. } => {
            draw_polyline(painter, rect, viewport, points, false, color, 1.5);
        }
        KicadCanvasGraphic::Bezier { points, .. } => {
            // Quadratic bezier approximation via sampled polyline
            if points.len() >= 3 {
                let sampled = quadratic_bezier_sample(points, 24);
                draw_polyline(painter, rect, viewport, &sampled, false, color, 1.5);
            } else if !points.is_empty() {
                draw_polyline(painter, rect, viewport, points, false, color, 1.5);
            }
        }
        KicadCanvasGraphic::Rectangle { start, end, .. } => {
            let s = viewport.world_to_screen(rect, *start);
            let e = viewport.world_to_screen(rect, *end);
            painter.rect_stroke(
                Rect::from_two_pos(s, e),
                0.0,
                Stroke::new(1.5, color),
                StrokeKind::Inside,
            );
        }
        KicadCanvasGraphic::Circle { center, radius, .. } => {
            painter.circle_stroke(
                viewport.world_to_screen(rect, *center),
                (*radius as f32 * viewport.zoom).abs(),
                Stroke::new(1.5, color),
            );
        }
        KicadCanvasGraphic::Arc {
            start, mid, end, ..
        } => {
            // Use proper arc sampling instead of 3-point V-shape
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
