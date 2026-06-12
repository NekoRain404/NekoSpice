/// Canvas drawing primitives for KiCad schematic rendering.
///
/// Organized into focused sub-modules:
/// - `grid` — background grid lines
/// - `sheet` — hierarchical sheet boxes
/// - `symbol` — symbol graphic elements and pin shapes
/// - `text` — rotated text rendering
///
/// This barrel module re-exports all public drawing functions and
/// contains shared utilities (polyline, line, bus entry, bounds, bezier).

mod grid;
mod sheet;
mod symbol;
mod text;

use crate::viewport::CanvasViewport;
use super::colors::SchematicColors;
use eframe::egui::{self, Color32, Rect, Stroke, StrokeKind};
use osl_kicad::{KicadBoundingBox, KicadCanvasBusEntry, KicadPoint};

pub(crate) use grid::draw_grid;
pub(crate) use sheet::draw_sheet;
pub(crate) use symbol::{draw_graphic, draw_pin};
pub(crate) use text::draw_rotated_text;

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
    sc: SchematicColors,
) {
    draw_line(
        painter,
        rect,
        viewport,
        entry.at,
        entry.end(),
        sc.bus_entry,
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
pub(super) fn quadratic_bezier_sample(points: &[KicadPoint], segments: usize) -> Vec<KicadPoint> {
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
