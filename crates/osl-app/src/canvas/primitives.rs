use crate::viewport::CanvasViewport;
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Stroke, StrokeKind, Vec2};
use osl_kicad::{
    KicadBoundingBox, KicadCanvasBusEntry, KicadCanvasGraphic, KicadCanvasSheet, KicadPoint,
};

pub(crate) fn draw_grid(painter: &egui::Painter, rect: Rect, viewport: CanvasViewport) {
    let major = (10.0 * viewport.zoom).max(16.0);
    let origin = rect.center() + viewport.pan;
    let stroke = Stroke::new(1.0, Color32::from_rgb(224, 229, 234));

    let mut x = origin.x % major;
    while x < rect.width() {
        let screen_x = rect.left() + x;
        painter.line_segment(
            [
                Pos2::new(screen_x, rect.top()),
                Pos2::new(screen_x, rect.bottom()),
            ],
            stroke,
        );
        x += major;
    }

    let mut y = origin.y % major;
    while y < rect.height() {
        let screen_y = rect.top() + y;
        painter.line_segment(
            [
                Pos2::new(rect.left(), screen_y),
                Pos2::new(rect.right(), screen_y),
            ],
            stroke,
        );
        y += major;
    }
}

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
    painter.rect_filled(sheet_rect, 0.0, Color32::from_rgb(245, 248, 255));
    painter.rect_stroke(
        sheet_rect,
        0.0,
        Stroke::new(1.5, Color32::from_rgb(90, 120, 190)),
        StrokeKind::Inside,
    );
    painter.text(
        sheet_rect.left_top() + Vec2::new(4.0, 4.0),
        Align2::LEFT_TOP,
        &sheet.name,
        FontId::monospace(12.0),
        Color32::from_rgb(50, 80, 150),
    );
}

pub(crate) fn draw_graphic(
    painter: &egui::Painter,
    rect: Rect,
    viewport: CanvasViewport,
    graphic: &KicadCanvasGraphic,
    color: Color32,
) {
    match graphic {
        KicadCanvasGraphic::Polyline { points, .. } | KicadCanvasGraphic::Bezier { points, .. } => {
            draw_polyline(painter, rect, viewport, points, false, color, 1.5);
        }
        KicadCanvasGraphic::Rectangle { start, end, .. } => {
            let start = viewport.world_to_screen(rect, *start);
            let end = viewport.world_to_screen(rect, *end);
            painter.rect_stroke(
                Rect::from_two_pos(start, end),
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
            let mut points = vec![*start];
            if let Some(mid) = mid {
                points.push(*mid);
            }
            points.push(*end);
            draw_polyline(painter, rect, viewport, &points, false, color, 1.5);
        }
        KicadCanvasGraphic::Text { text, at, .. } => {
            if let Some(at) = at {
                painter.text(
                    viewport.world_to_screen(rect, KicadPoint { x: at.x, y: at.y }),
                    Align2::LEFT_TOP,
                    text,
                    FontId::monospace(12.0),
                    color,
                );
            }
        }
    }
}

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
        draw_line(
            painter, rect, viewport, segment[0], segment[1], color, width,
        );
    }
    if closed
        && points.len() > 2
        && let Some(last) = points.last()
    {
        draw_line(painter, rect, viewport, *last, points[0], color, width);
    }
}

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
        Color32::from_rgb(70, 95, 220),
        2.0,
    );
}

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
