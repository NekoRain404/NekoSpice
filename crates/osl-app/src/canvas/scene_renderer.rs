//! 原理图场景渲染器。按层（Layer 1-13）顺序渲染完整原理图到画布。
//!
//! 渲染层次：sheets → rule areas → graphics → symbols → wires → buses →
//! junctions → directives → net labels → text → text boxes → junction dots → no-connects。

use crate::viewport::{CanvasViewport, item_visible};
use super::colors::SchematicColors;
use super::primitives;
use super::transforms::{pin_text_offsets, transform_property_point};
use eframe::egui::{self, Align2, FontId, Pos2, Stroke};
use osl_kicad::{KicadAt, KicadBoundingBox, KicadCanvasScene, KicadPoint};

/// 渲染完整原理图场景到画布区域。
///
/// 按 13 个图层顺序绘制所有原理图元素，仅渲染可见范围内的内容。
pub(crate) fn draw_scene(
    painter: &egui::Painter,
    rect: egui::Rect,
    scene: &KicadCanvasScene,
    viewport: CanvasViewport,
    visible_bounds: KicadBoundingBox,
    colors: SchematicColors,
) {
    draw_sheets(painter, rect, scene, viewport, visible_bounds, colors);
    draw_rule_areas(painter, rect, scene, viewport, visible_bounds, colors);
    draw_graphics(painter, rect, scene, viewport, visible_bounds, colors);
    draw_symbols(painter, rect, scene, viewport, visible_bounds, colors);
    draw_wires(painter, rect, scene, viewport, visible_bounds, colors);
    draw_buses(painter, rect, scene, viewport, visible_bounds, colors);
    draw_directive_labels(painter, rect, scene, viewport, visible_bounds, colors);
    draw_net_labels(painter, rect, scene, viewport, visible_bounds, colors);
    draw_text_items(painter, rect, scene, viewport, visible_bounds, colors);
    draw_text_boxes(painter, rect, scene, viewport, visible_bounds, colors);
    draw_junction_dots(painter, rect, scene, viewport, visible_bounds, colors);
    draw_no_connects(painter, rect, scene, viewport, visible_bounds, colors);
}

// ── Layer 1: 层次化图纸 ──────────────────────────────────────────────

fn draw_sheets(
    painter: &egui::Painter,
    rect: egui::Rect,
    scene: &KicadCanvasScene,
    viewport: CanvasViewport,
    visible_bounds: KicadBoundingBox,
    colors: SchematicColors,
) {
    for sheet in &scene.sheets {
        if !item_visible(sheet.bounds, visible_bounds) {
            continue;
        }
        primitives::draw_sheet(painter, rect, viewport, sheet, colors);
    }
}

// ── Layer 2: 规则区域 ────────────────────────────────────────────────

fn draw_rule_areas(
    painter: &egui::Painter,
    rect: egui::Rect,
    scene: &KicadCanvasScene,
    viewport: CanvasViewport,
    visible_bounds: KicadBoundingBox,
    colors: SchematicColors,
) {
    for rule_area in &scene.rule_areas {
        if !item_visible(rule_area.bounds, visible_bounds) {
            continue;
        }
        primitives::draw_polyline(
            painter, rect, viewport, &rule_area.points, true, colors.rule_area, 1.5,
        );
    }
}

// ── Layer 3: 顶层图形 ────────────────────────────────────────────────

fn draw_graphics(
    painter: &egui::Painter,
    rect: egui::Rect,
    scene: &KicadCanvasScene,
    viewport: CanvasViewport,
    visible_bounds: KicadBoundingBox,
    colors: SchematicColors,
) {
    for graphic in &scene.graphics {
        if !item_visible(graphic.bounds(), visible_bounds) {
            continue;
        }
        primitives::draw_graphic(painter, rect, viewport, graphic, colors.graphic);
    }
}

// ── Layer 4: 符号 ────────────────────────────────────────────────────

fn draw_symbols(
    painter: &egui::Painter,
    rect: egui::Rect,
    scene: &KicadCanvasScene,
    viewport: CanvasViewport,
    visible_bounds: KicadBoundingBox,
    colors: SchematicColors,
) {
    for symbol in &scene.symbols {
        if !item_visible(symbol.bounds, visible_bounds) {
            continue;
        }

        // 符号图形主体
        for graphic in &symbol.graphics {
            primitives::draw_graphic(painter, rect, viewport, graphic, colors.symbol_body);
        }

        // 引脚线段、名称和编号
        for pin in &symbol.pins {
            primitives::draw_pin(painter, rect, viewport, pin, colors);

            // 引脚名称（靠近主体端）
            if !pin.name.is_empty() {
                let (ox, oy, align) = pin_text_offsets(&pin.start, &pin.end, true);
                let pos = viewport.world_to_screen(
                    rect,
                    KicadPoint { x: pin.end.x + ox, y: pin.end.y + oy },
                );
                let fs = pin.name_effects.as_ref().and_then(|e| e.font_size).map(|s| s.width as f32).unwrap_or(8.0).max(5.0);
                painter.text(pos, align, &pin.name, FontId::proportional(fs), colors.symbol_pin_name);
            }

            // 引脚编号（靠近外部端）
            if !pin.number.is_empty() {
                let (ox, oy, align) = pin_text_offsets(&pin.start, &pin.end, false);
                let pos = viewport.world_to_screen(
                    rect,
                    KicadPoint { x: pin.start.x + ox, y: pin.start.y + oy },
                );
                let fs = pin.number_effects.as_ref().and_then(|e| e.font_size).map(|s| s.width as f32).unwrap_or(7.0).max(5.0);
                painter.text(pos, align, &pin.number, FontId::monospace(fs), colors.symbol_pin_number);
            }
        }

        // 符号参考标识（R1、C2 等）
        draw_symbol_property(
            painter, rect, viewport, symbol,
            &symbol.reference, symbol.reference_at.as_ref(), symbol.reference_effects.as_ref(),
            !symbol.reference.is_empty(),
            Align2::LEFT_TOP, colors.symbol_reference, 12.0,
        );

        // 符号值（100nF、10k 等）
        if !symbol.value.is_empty() {
            draw_symbol_property(
                painter, rect, viewport, symbol,
                &symbol.value, symbol.value_at.as_ref(), symbol.value_effects.as_ref(),
                true,
                Align2::LEFT_TOP, colors.symbol_value, 12.0,
            );
        }
    }
}

/// 绘制符号的单个属性（reference 或 value）。
///
/// 若提供了显式 `at` 坐标，则使用坐标变换定位；否则使用符号边界框角点。
fn draw_symbol_property(
    painter: &egui::Painter,
    rect: egui::Rect,
    viewport: CanvasViewport,
    symbol: &osl_kicad::KicadCanvasSymbol,
    text: &str,
    at: Option<&KicadAt>,
    effects: Option<&osl_kicad::KicadTextEffects>,
    should_draw: bool,
    fallback_align: Align2,
    color: eframe::egui::Color32,
    default_font_size: f32,
) {
    if !should_draw || text.is_empty() {
        return;
    }
    let fs = effects.and_then(|e| e.font_size).map(|s| s.width as f32).unwrap_or(default_font_size).max(6.0);
    if let Some(at) = at {
        let pp = transform_property_point(
            KicadPoint { x: at.x, y: at.y }, symbol.at, symbol.mirror.as_deref(),
        );
        let sp = viewport.world_to_screen(rect, pp);
        painter.text(sp, fallback_align, text, FontId::proportional(fs), color);
    }
}

// ── Layer 5: 导线 ────────────────────────────────────────────────────

fn draw_wires(
    painter: &egui::Painter,
    rect: egui::Rect,
    scene: &KicadCanvasScene,
    viewport: CanvasViewport,
    visible_bounds: KicadBoundingBox,
    colors: SchematicColors,
) {
    for wire in &scene.wires {
        if !item_visible(wire.bounds, visible_bounds) {
            continue;
        }
        for seg in wire.points.windows(2) {
            primitives::draw_line(painter, rect, viewport, seg[0], seg[1], colors.wire, 1.5);
        }
    }
}

// ── Layer 6: 总线 ────────────────────────────────────────────────────

fn draw_buses(
    painter: &egui::Painter,
    rect: egui::Rect,
    scene: &KicadCanvasScene,
    viewport: CanvasViewport,
    visible_bounds: KicadBoundingBox,
    colors: SchematicColors,
) {
    for bus in &scene.buses {
        if !item_visible(bus.bounds, visible_bounds) {
            continue;
        }
        for seg in bus.points.windows(2) {
            primitives::draw_line(painter, rect, viewport, seg[0], seg[1], colors.bus, 2.0);
        }
    }
    for entry in &scene.bus_entries {
        if !item_visible(entry.bounds, visible_bounds) {
            continue;
        }
        primitives::draw_bus_entry(painter, rect, viewport, entry, colors);
    }
}

// ── Layer 7: 仿真指令标签 ────────────────────────────────────────────

fn draw_directive_labels(
    painter: &egui::Painter,
    rect: egui::Rect,
    scene: &KicadCanvasScene,
    viewport: CanvasViewport,
    visible_bounds: KicadBoundingBox,
    colors: SchematicColors,
) {
    for label in &scene.directive_labels {
        if !item_visible(label.bounds, visible_bounds) {
            continue;
        }
        if let Some(bounds) = label.bounds {
            primitives::draw_bounds(painter, rect, viewport, bounds, colors.label_directive_bounds, 1.0);
        }
        if let Some(at) = label.at {
            primitives::draw_rotated_text(painter, rect, viewport, at, &label.text, 12.0, colors.label_directive);
        }
    }
}

// ── Layer 8: 网络标签 ────────────────────────────────────────────────

fn draw_net_labels(
    painter: &egui::Painter,
    rect: egui::Rect,
    scene: &KicadCanvasScene,
    viewport: CanvasViewport,
    visible_bounds: KicadBoundingBox,
    colors: SchematicColors,
) {
    for label in &scene.labels {
        if !item_visible(label.bounds, visible_bounds) {
            continue;
        }
        if let Some(at) = label.at {
            let label_color = match label.kind {
                osl_kicad::KicadLabelKind::Local => colors.label_local,
                osl_kicad::KicadLabelKind::Global => colors.label_global,
                osl_kicad::KicadLabelKind::Hierarchical => colors.label_hierarchical,
            };
            let fs = label.effects.as_ref().and_then(|e| e.font_size).map(|s| s.width as f32).unwrap_or(12.0).max(6.0);
            primitives::draw_rotated_text(painter, rect, viewport, at, &label.text, fs, label_color);
        }
    }
}

// ── Layer 9: 自由文本 ────────────────────────────────────────────────

fn draw_text_items(
    painter: &egui::Painter,
    rect: egui::Rect,
    scene: &KicadCanvasScene,
    viewport: CanvasViewport,
    visible_bounds: KicadBoundingBox,
    colors: SchematicColors,
) {
    for text in &scene.text_items {
        if !item_visible(text.bounds, visible_bounds) {
            continue;
        }
        if let Some(at) = text.at {
            let color = if text.is_spice_directive { colors.text_spice_directive } else { colors.text };
            let fs = text.effects.as_ref().and_then(|e| e.font_size).map(|s| s.width as f32).unwrap_or(12.0).max(6.0);
            primitives::draw_rotated_text(painter, rect, viewport, at, &text.text, fs, color);
        }
    }
}

// ── Layer 10: 文本框 ─────────────────────────────────────────────────

fn draw_text_boxes(
    painter: &egui::Painter,
    rect: egui::Rect,
    scene: &KicadCanvasScene,
    viewport: CanvasViewport,
    visible_bounds: KicadBoundingBox,
    colors: SchematicColors,
) {
    for text_box in &scene.text_boxes {
        if !item_visible(text_box.bounds, visible_bounds) {
            continue;
        }
        if let Some(bounds) = text_box.bounds {
            primitives::draw_bounds(painter, rect, viewport, bounds, colors.text_box_border, 1.0);
        }
    }
}

// ── Layer 11: 连接点 ─────────────────────────────────────────────────

/// KiCad 默认连接点直径 36mil = 0.9144mm。
fn draw_junction_dots(
    painter: &egui::Painter,
    rect: egui::Rect,
    scene: &KicadCanvasScene,
    viewport: CanvasViewport,
    visible_bounds: KicadBoundingBox,
    colors: SchematicColors,
) {
    const DIAM_MM: f64 = 0.9144;
    for j in &scene.junctions {
        if !j.bounds.intersects(visible_bounds) {
            continue;
        }
        let center = viewport.world_to_screen(rect, j.at);
        let radius = (DIAM_MM as f32 * viewport.zoom * 0.5).max(2.0);
        painter.circle_filled(center, radius, colors.junction);
    }
}

// ── Layer 12: 无连接标记 ─────────────────────────────────────────────

/// KiCad 默认无连接标记尺寸 48mil = 1.2192mm。
fn draw_no_connects(
    painter: &egui::Painter,
    rect: egui::Rect,
    scene: &KicadCanvasScene,
    viewport: CanvasViewport,
    visible_bounds: KicadBoundingBox,
    colors: SchematicColors,
) {
    const SIZE_MM: f64 = 1.2192;
    for m in &scene.no_connects {
        if !m.bounds.intersects(visible_bounds) {
            continue;
        }
        let c = viewport.world_to_screen(rect, m.at);
        let s = (SIZE_MM as f32 * viewport.zoom * 0.5).max(3.0);
        painter.line_segment([Pos2::new(c.x - s, c.y - s), Pos2::new(c.x + s, c.y + s)], Stroke::new(1.5, colors.no_connect));
        painter.line_segment([Pos2::new(c.x - s, c.y + s), Pos2::new(c.x + s, c.y - s)], Stroke::new(1.5, colors.no_connect));
    }
}
