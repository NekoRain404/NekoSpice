// Canvas rendering pipeline for KiCad schematic scenes.
// Draws sheets, graphics, symbols, wires, buses, labels, text,
// junctions, no-connects, and selection highlights.

use crate::viewport::{CanvasViewport, item_visible};
use eframe::egui::{self, Align2, FontId, Pos2, Rect, Stroke};
use osl_kicad::{KicadBoundingBox, KicadCanvasScene, KicadPoint};

pub(crate) mod colors;
mod primitives;

pub(crate) use primitives::{draw_bounds, draw_grid, draw_line};

/// Render the full schematic scene into the canvas area.
pub(crate) fn draw_scene(
    painter: &egui::Painter,
    rect: Rect,
    scene: &KicadCanvasScene,
    viewport: CanvasViewport,
    visible_bounds: KicadBoundingBox,
) {
    // Layer 1: Hierarchical sheets (background fill + border)
    for sheet in &scene.sheets {
        if !item_visible(sheet.bounds, visible_bounds) {
            continue;
        }
        primitives::draw_sheet(painter, rect, viewport, sheet);
    }

    // Layer 2: Rule areas (translucent fill outlines)
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
            colors::RULE_AREA,
            1.5,
        );
    }

    // Layer 3: Top-level graphic elements (not part of symbols)
    for graphic in &scene.graphics {
        if !item_visible(graphic.bounds(), visible_bounds) {
            continue;
        }
        primitives::draw_graphic(painter, rect, viewport, graphic, colors::GRAPHIC);
    }

    // Layer 4: Symbol bodies, pins, and labels
    for symbol in &scene.symbols {
        if !item_visible(symbol.bounds, visible_bounds) {
            continue;
        }
        // Symbol graphic body
        for graphic in &symbol.graphics {
            primitives::draw_graphic(
                painter,
                rect,
                viewport,
                graphic,
                colors::SYMBOL_BODY,
            );
        }
        // Symbol pin stubs with name and number labels
        for pin in &symbol.pins {
            primitives::draw_line(
                painter,
                rect,
                viewport,
                pin.start,
                pin.end,
                colors::SYMBOL_PIN,
                1.5,
            );
            // Pin name near the body end of the pin
            if !pin.name.is_empty() {
                let name_pos = viewport.world_to_screen(
                    rect,
                    KicadPoint {
                        x: pin.end.x + 1.0,
                        y: pin.end.y - 1.5,
                    },
                );
                painter.text(
                    name_pos,
                    Align2::LEFT_TOP,
                    &pin.name,
                    FontId::proportional(8.0),
                    colors::SYMBOL_PIN_NAME,
                );
            }
            // Pin number near the external end of the pin
            if !pin.number.is_empty() {
                let num_pos = viewport.world_to_screen(
                    rect,
                    KicadPoint {
                        x: pin.start.x + 1.0,
                        y: pin.start.y + 1.5,
                    },
                );
                painter.text(
                    num_pos,
                    Align2::LEFT_BOTTOM,
                    &pin.number,
                    FontId::proportional(7.0),
                    colors::SYMBOL_PIN_NUMBER,
                );
            }
        }
        // Symbol reference and value labels
        if let Some(bounds) = symbol.bounds {
            let ref_pos = viewport.world_to_screen(rect, bounds.min);
            painter.text(
                ref_pos,
                Align2::LEFT_BOTTOM,
                &symbol.reference,
                FontId::proportional(12.0),
                colors::SYMBOL_REFERENCE,
            );
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
                    colors::SYMBOL_VALUE,
                );
            }
        }
    }

    // Layer 5: Wires (green polylines)
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
            colors::WIRE,
            2.0,
        );
    }

    // Layer 6: Buses (blue polylines)
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
            colors::BUS,
            3.0,
        );
    }

    // Layer 7: Bus entries
    for entry in &scene.bus_entries {
        if !item_visible(entry.bounds, visible_bounds) {
            continue;
        }
        primitives::draw_bus_entry(painter, rect, viewport, entry);
    }

    // Layer 8: Directive labels (netclass flags with bounds box)
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
                colors::LABEL_DIRECTIVE_BOUNDS,
                1.0,
            );
        }
        if let Some(at) = label.at {
            painter.text(
                viewport.world_to_screen(rect, KicadPoint { x: at.x, y: at.y }),
                Align2::LEFT_TOP,
                &label.text,
                FontId::proportional(12.0),
                colors::LABEL_DIRECTIVE,
            );
        }
    }

    // Layer 9: Net labels (local/global/hierarchical)
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
                colors::LABEL_LOCAL,
            );
        }
    }

    // Layer 10: Free text items
    for text in &scene.text_items {
        if !item_visible(text.bounds, visible_bounds) {
            continue;
        }
        if let Some(at) = text.at {
            let color = if text.is_spice_directive {
                colors::TEXT_SPICE_DIRECTIVE
            } else {
                colors::TEXT
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

    // Layer 11: Text boxes (border only)
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
                colors::TEXT_BOX_BORDER,
                1.0,
            );
        }
    }

    // Layer 12: Junctions (filled green dots)
    for junction in &scene.junctions {
        if !junction.bounds.intersects(visible_bounds) {
            continue;
        }
        let center = viewport.world_to_screen(rect, junction.at);
        painter.circle_filled(center, 3.0, colors::JUNCTION);
    }

    // Layer 13: No-connect markers (X marks)
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
            Stroke::new(1.5, colors::NO_CONNECT),
        );
        painter.line_segment(
            [
                Pos2::new(center.x - size, center.y + size),
                Pos2::new(center.x + size, center.y - size),
            ],
            Stroke::new(1.5, colors::NO_CONNECT),
        );
    }
}
