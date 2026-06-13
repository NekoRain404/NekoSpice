//! 原理图场景渲染器。按层（Layer 1-13）顺序渲染完整原理图到画布。
//!
//! 本文件包含编排函数和结构层（sheets、rule areas、graphics、symbols）。
//! 导线/总线层见 [`super::scene_renderer_wires`]，标注/标记层见
//! [`super::scene_renderer_annotations`]。

use super::colors::SchematicColors;
use super::primitives;
use super::transforms::{pin_text_offsets, transform_property_point};
use crate::viewport::{CanvasViewport, item_visible};
use eframe::egui::{self, Align2, FontId};
use nsp_schema::{NspAt, NspBoundingBox, NspCanvasScene, NspPoint};

/// 渲染完整原理图场景到画布区域。
///
/// 按 12 个图层顺序绘制所有原理图元素，仅渲染可见范围内的内容。
pub(crate) fn draw_scene(
    painter: &egui::Painter,
    rect: egui::Rect,
    scene: &NspCanvasScene,
    viewport: CanvasViewport,
    visible_bounds: NspBoundingBox,
    colors: SchematicColors,
) {
    // ── 结构层（本模块） ──
    draw_sheets(painter, rect, scene, viewport, visible_bounds, colors);
    draw_rule_areas(painter, rect, scene, viewport, visible_bounds, colors);
    draw_graphics(painter, rect, scene, viewport, visible_bounds, colors);
    draw_symbols(painter, rect, scene, viewport, visible_bounds, colors);

    // ── 连接层（scene_renderer_wires） ──
    super::scene_renderer_wires::draw_wires(painter, rect, scene, viewport, visible_bounds, colors);
    super::scene_renderer_wires::draw_buses(painter, rect, scene, viewport, visible_bounds, colors);

    // ── 标注层（scene_renderer_annotations） ──
    super::scene_renderer_annotations::draw_directive_labels(
        painter,
        rect,
        scene,
        viewport,
        visible_bounds,
        colors,
    );
    super::scene_renderer_annotations::draw_net_labels(
        painter,
        rect,
        scene,
        viewport,
        visible_bounds,
        colors,
    );
    super::scene_renderer_annotations::draw_text_items(
        painter,
        rect,
        scene,
        viewport,
        visible_bounds,
        colors,
    );
    super::scene_renderer_annotations::draw_text_boxes(
        painter,
        rect,
        scene,
        viewport,
        visible_bounds,
        colors,
    );
    super::scene_renderer_annotations::draw_junction_dots(
        painter,
        rect,
        scene,
        viewport,
        visible_bounds,
        colors,
    );
    super::scene_renderer_annotations::draw_no_connects(
        painter,
        rect,
        scene,
        viewport,
        visible_bounds,
        colors,
    );
}

// ── Layer 1: 层次化图纸 ──────────────────────────────────────────────

fn draw_sheets(
    painter: &egui::Painter,
    rect: egui::Rect,
    scene: &NspCanvasScene,
    viewport: CanvasViewport,
    visible_bounds: NspBoundingBox,
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
    scene: &NspCanvasScene,
    viewport: CanvasViewport,
    visible_bounds: NspBoundingBox,
    colors: SchematicColors,
) {
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
            colors.rule_area,
            1.5,
        );
    }
}

// ── Layer 3: 顶层图形 ────────────────────────────────────────────────

fn draw_graphics(
    painter: &egui::Painter,
    rect: egui::Rect,
    scene: &NspCanvasScene,
    viewport: CanvasViewport,
    visible_bounds: NspBoundingBox,
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
    scene: &NspCanvasScene,
    viewport: CanvasViewport,
    visible_bounds: NspBoundingBox,
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
                    NspPoint {
                        x: pin.end.x + ox,
                        y: pin.end.y + oy,
                    },
                );
                let fs = pin
                    .name_effects
                    .as_ref()
                    .and_then(|e| e.font_size)
                    .map(|s| s.width as f32)
                    .unwrap_or(8.0)
                    .max(5.0);
                painter.text(
                    pos,
                    align,
                    &pin.name,
                    FontId::proportional(fs),
                    colors.symbol_pin_name,
                );
            }

            // 引脚编号（靠近外部端）
            if !pin.number.is_empty() {
                let (ox, oy, align) = pin_text_offsets(&pin.start, &pin.end, false);
                let pos = viewport.world_to_screen(
                    rect,
                    NspPoint {
                        x: pin.start.x + ox,
                        y: pin.start.y + oy,
                    },
                );
                let fs = pin
                    .number_effects
                    .as_ref()
                    .and_then(|e| e.font_size)
                    .map(|s| s.width as f32)
                    .unwrap_or(7.0)
                    .max(5.0);
                painter.text(
                    pos,
                    align,
                    &pin.number,
                    FontId::monospace(fs),
                    colors.symbol_pin_number,
                );
            }
        }

        // 符号参考标识（R1、C2 等）
        draw_symbol_property(
            painter,
            rect,
            viewport,
            symbol,
            &symbol.reference,
            symbol.reference_at.as_ref(),
            symbol.reference_effects.as_ref(),
            !symbol.reference.is_empty(),
            Align2::LEFT_TOP,
            colors.symbol_reference,
            12.0,
        );

        // 符号值（100nF、10k 等）
        if !symbol.value.is_empty() {
            draw_symbol_property(
                painter,
                rect,
                viewport,
                symbol,
                &symbol.value,
                symbol.value_at.as_ref(),
                symbol.value_effects.as_ref(),
                true,
                Align2::LEFT_TOP,
                colors.symbol_value,
                12.0,
            );
        }
    }
}

/// 绘制符号的单个属性（reference 或 value）。
///
/// 若提供了显式 `at` 坐标，则使用坐标变换定位；否则使用符号边界框角点。
#[allow(clippy::too_many_arguments)]
fn draw_symbol_property(
    painter: &egui::Painter,
    rect: egui::Rect,
    viewport: CanvasViewport,
    symbol: &nsp_schema::NspCanvasSymbol,
    text: &str,
    at: Option<&NspAt>,
    effects: Option<&nsp_schema::NspTextEffects>,
    should_draw: bool,
    fallback_align: Align2,
    color: eframe::egui::Color32,
    default_font_size: f32,
) {
    if !should_draw || text.is_empty() {
        return;
    }
    let fs = effects
        .and_then(|e| e.font_size)
        .map(|s| s.width as f32)
        .unwrap_or(default_font_size)
        .max(6.0);
    if let Some(at) = at {
        let pp = transform_property_point(
            NspPoint { x: at.x, y: at.y },
            symbol.at,
            symbol.mirror.as_deref(),
        );
        let sp = viewport.world_to_screen(rect, pp);
        painter.text(sp, fallback_align, text, FontId::proportional(fs), color);
    }
}
