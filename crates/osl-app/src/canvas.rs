// Canvas rendering pipeline for KiCad schematic scenes.
// Draws sheets, graphics, symbols, wires, buses, labels, text,
// junctions, no-connects, and selection highlights.

use crate::viewport::{CanvasViewport, item_visible};
use eframe::egui::{self, Align2, FontId, Pos2, Rect, Stroke};
use osl_kicad::{KicadBoundingBox, KicadCanvasScene, KicadPoint};

pub(crate) mod colors;
mod primitives;

pub(crate) use primitives::{draw_bounds, draw_grid, draw_line};

/// Compute text offset and alignment for pin name or number based on pin direction.
///
/// `is_name=true` positions at the body end; `is_name=false` at the external end.
/// Returns (offset_x, offset_y, alignment).
fn pin_text_offsets(
    start: &KicadPoint,
    end: &KicadPoint,
    is_name: bool,
) -> (f64, f64, Align2) {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let dist = (dx * dx + dy * dy).sqrt();

    if dist < 1e-6 {
        return (1.0, -1.5, Align2::LEFT_TOP);
    }

    // Normalize direction
    let nx = dx / dist;
    let ny = dy / dist;

    // Perpendicular offset for text placement (always to the "right" of pin direction)
    let perp_x = ny;   // rotate 90 degrees
    let perp_y = -nx;

    let offset = if is_name { 0.8 } else { 0.8 };

    if is_name {
        // Name at body end, offset perpendicular to pin direction
        let align = if perp_x.abs() > perp_y.abs() {
            if perp_x > 0.0 { Align2::LEFT_CENTER } else { Align2::RIGHT_CENTER }
        } else {
            if perp_y > 0.0 { Align2::CENTER_TOP } else { Align2::CENTER_BOTTOM }
        };
        (perp_x * offset, perp_y * offset, align)
    } else {
        // Number at external end, offset perpendicular on opposite side
        let align = if perp_x.abs() > perp_y.abs() {
            if perp_x > 0.0 { Align2::RIGHT_CENTER } else { Align2::LEFT_CENTER }
        } else {
            if perp_y > 0.0 { Align2::CENTER_BOTTOM } else { Align2::CENTER_TOP }
        };
        (-perp_x * offset, -perp_y * offset, align)
    }
}

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

            // Pin name near the body end (end point) of the pin
            // Position based on pin direction for correct placement
            if !pin.name.is_empty() {
                let (name_offset_x, name_offset_y, name_align) =
                    pin_text_offsets(&pin.start, &pin.end, true);
                let name_pos = viewport.world_to_screen(
                    rect,
                    KicadPoint {
                        x: pin.end.x + name_offset_x,
                        y: pin.end.y + name_offset_y,
                    },
                );
                // Use font size from name_effects if available
                let font_size = pin.name_effects
                    .as_ref()
                    .and_then(|e| e.font_size)
                    .map(|s| s.width as f32)
                    .unwrap_or(8.0)
                    .max(5.0);
                painter.text(
                    name_pos,
                    name_align,
                    &pin.name,
                    FontId::proportional(font_size),
                    colors::SYMBOL_PIN_NAME,
                );
            }

            // Pin number near the external end (start point) of the pin
            if !pin.number.is_empty() {
                let (num_offset_x, num_offset_y, num_align) =
                    pin_text_offsets(&pin.start, &pin.end, false);
                let num_pos = viewport.world_to_screen(
                    rect,
                    KicadPoint {
                        x: pin.start.x + num_offset_x,
                        y: pin.start.y + num_offset_y,
                    },
                );
                let font_size = pin.number_effects
                    .as_ref()
                    .and_then(|e| e.font_size)
                    .map(|s| s.width as f32)
                    .unwrap_or(7.0)
                    .max(5.0);
                painter.text(
                    num_pos,
                    num_align,
                    &pin.number,
                    FontId::monospace(font_size),
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
