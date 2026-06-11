use crate::viewport::{CanvasViewport, item_visible};
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Stroke};
use osl_kicad::{KicadBoundingBox, KicadCanvasScene, KicadPoint};

mod primitives;

pub(crate) use primitives::{draw_bounds, draw_grid, draw_line};

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
        primitives::draw_sheet(painter, rect, viewport, sheet);
    }
    for rule_area in &scene.rule_areas {
        if !item_visible(rule_area.bounds, visible_bounds) {
            continue;
        }
        primitives::draw_polyline(
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
        primitives::draw_graphic(
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
            primitives::draw_graphic(
                painter,
                rect,
                viewport,
                graphic,
                Color32::from_rgb(25, 25, 25),
            );
        }
        for pin in &symbol.pins {
            primitives::draw_line(
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
            // Reference label (e.g. R1, C2, U3) above the symbol
            let ref_pos = viewport.world_to_screen(rect, bounds.min);
            painter.text(
                ref_pos,
                Align2::LEFT_BOTTOM,
                &symbol.reference,
                FontId::proportional(12.0),
                Color32::from_rgb(25, 25, 25),
            );
            // Value label (e.g. 10k, 100nF) below the symbol
            if !symbol.value.is_empty() && symbol.value != symbol.reference {
                let val_pos = viewport.world_to_screen(
                    rect,
                    KicadPoint {
                        x: bounds.min.x,
                        y: bounds.max.y + 2.0,
                    },
                );
                painter.text(
                    val_pos,
                    Align2::LEFT_TOP,
                    &symbol.value,
                    FontId::proportional(11.0),
                    Color32::from_rgb(80, 80, 80),
                );
            }
        }
    }
    for wire in &scene.wires {
        if !item_visible(wire.bounds, visible_bounds) {
            continue;
        }
        primitives::draw_polyline(
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
        primitives::draw_polyline(
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
        primitives::draw_bus_entry(painter, rect, viewport, entry);
    }
    for label in &scene.directive_labels {
        if !item_visible(label.bounds, visible_bounds) {
            continue;
        }
        if let Some(bounds) = label.bounds {
            primitives::draw_bounds(
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
                FontId::proportional(12.0),
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
                FontId::proportional(12.0),
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
                FontId::proportional(12.0),
                color,
            );
        }
    }
    for text_box in &scene.text_boxes {
        if !item_visible(text_box.bounds, visible_bounds) {
            continue;
        }
        if let Some(bounds) = text_box.bounds {
            primitives::draw_bounds(
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
