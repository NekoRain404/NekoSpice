use crate::viewport::{CanvasViewport, item_visible};
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Stroke, StrokeKind, Vec2};
use osl_kicad::{
    KicadBoundingBox, KicadCanvasBusEntry, KicadCanvasGraphic, KicadCanvasScene, KicadCanvasSheet,
    KicadPoint,
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

pub(crate) fn draw_scene(
    painter: &egui::Painter,
    rect: Rect,
    scene: &KicadCanvasScene,
    viewport: CanvasViewport,
    visible_bounds: KicadBoundingBox,
) {
    for sheet in &scene.sheets {
        if !item_visible(sheet.bounds, visible_bounds) {
            continue;
        }
        draw_sheet(painter, rect, viewport, sheet);
    }
    for rule_area in &scene.rule_areas {
        if !item_visible(rule_area.bounds, visible_bounds) {
            continue;
        }
        draw_polyline(
            painter,
            rect,
            viewport,
            &rule_area.points,
            true,
            Color32::from_rgb(150, 110, 20),
            1.5,
        );
    }
    for graphic in &scene.graphics {
        if !item_visible(graphic.bounds(), visible_bounds) {
            continue;
        }
        draw_graphic(
            painter,
            rect,
            viewport,
            graphic,
            Color32::from_rgb(90, 90, 90),
        );
    }
    for symbol in &scene.symbols {
        if !item_visible(symbol.bounds, visible_bounds) {
            continue;
        }
        for graphic in &symbol.graphics {
            draw_graphic(
                painter,
                rect,
                viewport,
                graphic,
                Color32::from_rgb(25, 25, 25),
            );
        }
        for pin in &symbol.pins {
            draw_line(
                painter,
                rect,
                viewport,
                pin.start,
                pin.end,
                Color32::from_rgb(30, 30, 30),
                1.5,
            );
        }
        if let Some(bounds) = symbol.bounds {
            let label_pos = viewport.world_to_screen(rect, bounds.min);
            painter.text(
                label_pos,
                Align2::LEFT_BOTTOM,
                &symbol.reference,
                FontId::monospace(12.0),
                Color32::from_rgb(25, 25, 25),
            );
        }
    }
    for wire in &scene.wires {
        if !item_visible(wire.bounds, visible_bounds) {
            continue;
        }
        draw_polyline(
            painter,
            rect,
            viewport,
            &wire.points,
            false,
            Color32::from_rgb(0, 150, 72),
            2.0,
        );
    }
    for bus in &scene.buses {
        if !item_visible(bus.bounds, visible_bounds) {
            continue;
        }
        draw_polyline(
            painter,
            rect,
            viewport,
            &bus.points,
            false,
            Color32::from_rgb(70, 95, 220),
            3.0,
        );
    }
    for entry in &scene.bus_entries {
        if !item_visible(entry.bounds, visible_bounds) {
            continue;
        }
        draw_bus_entry(painter, rect, viewport, entry);
    }
    for label in &scene.directive_labels {
        if !item_visible(label.bounds, visible_bounds) {
            continue;
        }
        if let Some(bounds) = label.bounds {
            draw_bounds(
                painter,
                rect,
                viewport,
                bounds,
                Color32::from_rgb(180, 95, 35),
                1.0,
            );
        }
        if let Some(at) = label.at {
            painter.text(
                viewport.world_to_screen(rect, KicadPoint { x: at.x, y: at.y }),
                Align2::LEFT_TOP,
                &label.text,
                FontId::monospace(12.0),
                Color32::from_rgb(150, 65, 20),
            );
        }
    }
    for label in &scene.labels {
        if !item_visible(label.bounds, visible_bounds) {
            continue;
        }
        if let Some(at) = label.at {
            painter.text(
                viewport.world_to_screen(rect, KicadPoint { x: at.x, y: at.y }),
                Align2::LEFT_TOP,
                &label.text,
                FontId::monospace(12.0),
                Color32::from_rgb(0, 95, 180),
            );
        }
    }
    for text in &scene.text_items {
        if !item_visible(text.bounds, visible_bounds) {
            continue;
        }
        if let Some(at) = text.at {
            let color = if text.is_spice_directive {
                Color32::from_rgb(165, 45, 45)
            } else {
                Color32::from_rgb(55, 55, 55)
            };
            painter.text(
                viewport.world_to_screen(rect, KicadPoint { x: at.x, y: at.y }),
                Align2::LEFT_TOP,
                &text.text,
                FontId::monospace(12.0),
                color,
            );
        }
    }
    for text_box in &scene.text_boxes {
        if !item_visible(text_box.bounds, visible_bounds) {
            continue;
        }
        if let Some(bounds) = text_box.bounds {
            draw_bounds(
                painter,
                rect,
                viewport,
                bounds,
                Color32::from_rgb(120, 120, 120),
                1.0,
            );
        }
    }
    for junction in &scene.junctions {
        if !junction.bounds.intersects(visible_bounds) {
            continue;
        }
        let center = viewport.world_to_screen(rect, junction.at);
        painter.circle_filled(center, 3.0, Color32::from_rgb(0, 150, 72));
    }
    for marker in &scene.no_connects {
        if !marker.bounds.intersects(visible_bounds) {
            continue;
        }
        let center = viewport.world_to_screen(rect, marker.at);
        let size = 5.0;
        painter.line_segment(
            [
                Pos2::new(center.x - size, center.y - size),
                Pos2::new(center.x + size, center.y + size),
            ],
            Stroke::new(1.5, Color32::from_rgb(55, 55, 55)),
        );
        painter.line_segment(
            [
                Pos2::new(center.x - size, center.y + size),
                Pos2::new(center.x + size, center.y - size),
            ],
            Stroke::new(1.5, Color32::from_rgb(55, 55, 55)),
        );
    }
}

fn draw_sheet(
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

fn draw_graphic(
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

fn draw_polyline(
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

fn draw_line(
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

fn draw_bus_entry(
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
