// Canvas rendering pipeline for KiCad schematic scenes.
// Draws sheets, graphics, symbols, wires, buses, labels, text,
// junctions, no-connects, and selection highlights.

use crate::viewport::{CanvasViewport, item_visible};
use eframe::egui::{self, Align2, FontId, Pos2, Rect, Stroke};
use osl_kicad::{KicadAt, KicadBoundingBox, KicadCanvasScene, KicadPoint};

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

    let offset = 0.8;

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

/// Transform a property point through a symbol's position and mirror.
///
/// This mirrors the KiCad coordinate transform: first apply mirror,
/// then rotate by the symbol rotation, then translate to the symbol position.
fn transform_property_point(
    local: KicadPoint,
    symbol_at: KicadAt,
    mirror: Option<&str>,
) -> KicadPoint {
    // Apply mirror first
    let mut mirrored = local;
    if let Some(mirror_str) = mirror {
        if mirror_str.contains('x') {
            mirrored.y = -mirrored.y;
        }
        if mirror_str.contains('y') {
            mirrored.x = -mirrored.x;
        }
    }
    // Apply rotation
    let rotation = symbol_at.rotation % 360.0;
    let normalized = if rotation < 0.0 { rotation + 360.0 } else { rotation };
    let rotated = match normalized.round() as i32 {
        0 => mirrored,
        90 => KicadPoint { x: -mirrored.y, y: mirrored.x },
        180 => KicadPoint { x: -mirrored.x, y: -mirrored.y },
        270 => KicadPoint { x: mirrored.y, y: -mirrored.x },
        _ => {
            let radians = rotation.to_radians();
            KicadPoint {
                x: mirrored.x * radians.cos() - mirrored.y * radians.sin(),
                y: mirrored.x * radians.sin() + mirrored.y * radians.cos(),
            }
        }
    };
    // Translate to symbol position
    KicadPoint {
        x: symbol_at.x + rotated.x,
        y: symbol_at.y + rotated.y,
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
            // Draw pin shape based on pin type
            primitives::draw_pin(
                painter,
                rect,
                viewport,
                pin,
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
        // Use property positions when available, otherwise fall back to bounding box
        if !symbol.reference.is_empty() {
            let ref_font_size = symbol.reference_effects
                .as_ref()
                .and_then(|e| e.font_size)
                .map(|s| s.width as f32)
                .unwrap_or(12.0)
                .max(6.0);

            if let Some(ref_at) = symbol.reference_at {
                // Transform property position through symbol's at/mirror
                let prop_point = transform_property_point(
                    KicadPoint { x: ref_at.x, y: ref_at.y },
                    symbol.at,
                    symbol.mirror.as_deref(),
                );
                let screen_pos = viewport.world_to_screen(rect, prop_point);
                painter.text(
                    screen_pos,
                    Align2::LEFT_TOP,
                    &symbol.reference,
                    FontId::proportional(ref_font_size),
                    colors::SYMBOL_REFERENCE,
                );
            } else if let Some(bounds) = symbol.bounds {
                // Fallback: use bounding box position
                let ref_pos = viewport.world_to_screen(rect, bounds.min);
                painter.text(
                    ref_pos,
                    Align2::LEFT_BOTTOM,
                    &symbol.reference,
                    FontId::proportional(ref_font_size),
                    colors::SYMBOL_REFERENCE,
                );
            }
        }

        if !symbol.value.is_empty() && symbol.value != symbol.reference {
            let val_font_size = symbol.value_effects
                .as_ref()
                .and_then(|e| e.font_size)
                .map(|s| s.width as f32)
                .unwrap_or(11.0)
                .max(6.0);

            if let Some(val_at) = symbol.value_at {
                // Transform property position through symbol's at/mirror
                let prop_point = transform_property_point(
                    KicadPoint { x: val_at.x, y: val_at.y },
                    symbol.at,
                    symbol.mirror.as_deref(),
                );
                let screen_pos = viewport.world_to_screen(rect, prop_point);
                painter.text(
                    screen_pos,
                    Align2::LEFT_TOP,
                    &symbol.value,
                    FontId::proportional(val_font_size),
                    colors::SYMBOL_VALUE,
                );
            } else if let Some(bounds) = symbol.bounds {
                // Fallback: use bounding box position
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
                    FontId::proportional(val_font_size),
                    colors::SYMBOL_VALUE,
                );
            }
        }
    }

    // Layer 5: Wires (green polylines)
    // KiCad default wire width is 0.1524mm (6mil) when stroke width is None or 0.0
    const KICAD_DEFAULT_WIRE_WIDTH_MM: f64 = 0.1524;
    for wire in &scene.wires {
        if !item_visible(wire.bounds, visible_bounds) {
            continue;
        }
        let wire_width = wire.stroke.as_ref()
            .and_then(|s| s.width)
            .filter(|w| *w > 0.0)
            .unwrap_or(KICAD_DEFAULT_WIRE_WIDTH_MM);
        let screen_width = (wire_width as f32 * viewport.zoom).max(1.0);
        primitives::draw_polyline(
            painter,
            rect,
            viewport,
            &wire.points,
            false,
            colors::WIRE,
            screen_width,
        );
    }

    // Layer 6: Buses (blue polylines)
    // KiCad default bus width is 0.3048mm (12mil) when stroke width is None or 0.0
    const KICAD_DEFAULT_BUS_WIDTH_MM: f64 = 0.3048;
    for bus in &scene.buses {
        if !item_visible(bus.bounds, visible_bounds) {
            continue;
        }
        let bus_width = bus.stroke.as_ref()
            .and_then(|s| s.width)
            .filter(|w| *w > 0.0)
            .unwrap_or(KICAD_DEFAULT_BUS_WIDTH_MM);
        let screen_width = (bus_width as f32 * viewport.zoom).max(1.5);
        primitives::draw_polyline(
            painter,
            rect,
            viewport,
            &bus.points,
            false,
            colors::BUS,
            screen_width,
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
            primitives::draw_rotated_text(
                painter,
                rect,
                viewport,
                at,
                &label.text,
                12.0,
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
            let label_color = match label.kind {
                osl_kicad::KicadLabelKind::Local => colors::LABEL_LOCAL,
                osl_kicad::KicadLabelKind::Global => colors::LABEL_GLOBAL,
                osl_kicad::KicadLabelKind::Hierarchical => colors::LABEL_HIERARCHICAL,
            };
            let font_size = label.effects
                .as_ref()
                .and_then(|e| e.font_size)
                .map(|s| s.width as f32)
                .unwrap_or(12.0)
                .max(6.0);
            primitives::draw_rotated_text(
                painter,
                rect,
                viewport,
                at,
                &label.text,
                font_size,
                label_color,
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
            let font_size = text.effects
                .as_ref()
                .and_then(|e| e.font_size)
                .map(|s| s.width as f32)
                .unwrap_or(12.0)
                .max(6.0);
            primitives::draw_rotated_text(
                painter,
                rect,
                viewport,
                at,
                &text.text,
                font_size,
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
    // KiCad default junction diameter is 36 mils (0.9144mm)
    const KICAD_DEFAULT_JUNCTION_DIAM_MM: f64 = 0.9144;
    for junction in &scene.junctions {
        if !junction.bounds.intersects(visible_bounds) {
            continue;
        }
        let center = viewport.world_to_screen(rect, junction.at);
        let radius = (KICAD_DEFAULT_JUNCTION_DIAM_MM as f32 * viewport.zoom * 0.5).max(2.0);
        painter.circle_filled(center, radius, colors::JUNCTION);
    }

    // Layer 13: No-connect markers (X marks)
    // KiCad default no-connect size is 48 mils (1.2192mm)
    const KICAD_DEFAULT_NOCONNECT_SIZE_MM: f64 = 1.2192;
    for marker in &scene.no_connects {
        if !marker.bounds.intersects(visible_bounds) {
            continue;
        }
        let center = viewport.world_to_screen(rect, marker.at);
        let size = (KICAD_DEFAULT_NOCONNECT_SIZE_MM as f32 * viewport.zoom * 0.5).max(3.0);
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


/// Draw a semi-transparent hover highlight around a hovered item's bounding box.
///
/// Uses a lighter, translucent color compared to the solid selection highlight,
/// giving the user subtle visual feedback before clicking.
pub(crate) fn draw_hover_highlight(
    painter: &egui::Painter,
    rect: Rect,
    viewport: CanvasViewport,
    bounds: osl_kicad::KicadBoundingBox,
) {
    primitives::draw_bounds(
        painter,
        rect,
        viewport,
        bounds,
        colors::HOVER_HIGHLIGHT,
        1.5,
    );
}

