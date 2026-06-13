//! 原理图场景渲染 — 标注与标记层（Layer 7-12）。
//!
//! 包含仿真指令标签、网络标签、自由文本、文本框、连接点和无连接标记。
//! 这些图层提供了原理图的文本信息和电气连接状态可视化。

use super::colors::SchematicColors;
use super::primitives;
use crate::viewport::{CanvasViewport, item_visible};
use eframe::egui::{self, Pos2, Stroke};
use nsp_schema::{NspBoundingBox, NspCanvasScene};

/// Layer 7: 绘制仿真指令标签（.tran、.ac 等）。
///
/// 指令标签通常带背景框，用于标识原理图中的 SPICE 仿真配置。
pub(crate) fn draw_directive_labels(
    painter: &egui::Painter,
    rect: egui::Rect,
    scene: &NspCanvasScene,
    viewport: CanvasViewport,
    visible_bounds: NspBoundingBox,
    colors: SchematicColors,
) {
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
                colors.label_directive_bounds,
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
                colors.label_directive,
            );
        }
    }
}

/// Layer 8: 绘制网络标签（Local / Global / Hierarchical）。
///
/// 不同层级的标签使用不同颜色区分作用域。
pub(crate) fn draw_net_labels(
    painter: &egui::Painter,
    rect: egui::Rect,
    scene: &NspCanvasScene,
    viewport: CanvasViewport,
    visible_bounds: NspBoundingBox,
    colors: SchematicColors,
) {
    for label in &scene.labels {
        if !item_visible(label.bounds, visible_bounds) {
            continue;
        }
        if let Some(at) = label.at {
            let label_color = match label.kind {
                nsp_schema::NspLabelKind::Local => colors.label_local,
                nsp_schema::NspLabelKind::Global => colors.label_global,
                nsp_schema::NspLabelKind::Hierarchical => colors.label_hierarchical,
            };
            let fs = label
                .effects
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
                fs,
                label_color,
            );
        }
    }
}

/// Layer 9: 绘制自由文本项。
///
/// SPICE 指令文本使用专用颜色，普通文本使用默认文本颜色。
pub(crate) fn draw_text_items(
    painter: &egui::Painter,
    rect: egui::Rect,
    scene: &NspCanvasScene,
    viewport: CanvasViewport,
    visible_bounds: NspBoundingBox,
    colors: SchematicColors,
) {
    for text in &scene.text_items {
        if !item_visible(text.bounds, visible_bounds) {
            continue;
        }
        if let Some(at) = text.at {
            let color = if text.is_spice_directive {
                colors.text_spice_directive
            } else {
                colors.text
            };
            let fs = text
                .effects
                .as_ref()
                .and_then(|e| e.font_size)
                .map(|s| s.width as f32)
                .unwrap_or(12.0)
                .max(6.0);
            primitives::draw_rotated_text(painter, rect, viewport, at, &text.text, fs, color);
        }
    }
}

/// Layer 10: 绘制文本框边框。
pub(crate) fn draw_text_boxes(
    painter: &egui::Painter,
    rect: egui::Rect,
    scene: &NspCanvasScene,
    viewport: CanvasViewport,
    visible_bounds: NspBoundingBox,
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

/// Layer 11: 绘制连接点（Junction dots）。
///
/// schema 默认连接点直径 36mil ≈ 0.9144mm。
pub(crate) fn draw_junction_dots(
    painter: &egui::Painter,
    rect: egui::Rect,
    scene: &NspCanvasScene,
    viewport: CanvasViewport,
    visible_bounds: NspBoundingBox,
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

/// Layer 12: 绘制无连接标记（No-connect X markers）。
///
/// schema 默认无连接标记尺寸 48mil ≈ 1.2192mm。
pub(crate) fn draw_no_connects(
    painter: &egui::Painter,
    rect: egui::Rect,
    scene: &NspCanvasScene,
    viewport: CanvasViewport,
    visible_bounds: NspBoundingBox,
    colors: SchematicColors,
) {
    const SIZE_MM: f64 = 1.2192;
    for m in &scene.no_connects {
        if !m.bounds.intersects(visible_bounds) {
            continue;
        }
        let c = viewport.world_to_screen(rect, m.at);
        let s = (SIZE_MM as f32 * viewport.zoom * 0.5).max(3.0);
        let stroke = Stroke::new(1.5, colors.no_connect);
        painter.line_segment(
            [Pos2::new(c.x - s, c.y - s), Pos2::new(c.x + s, c.y + s)],
            stroke,
        );
        painter.line_segment(
            [Pos2::new(c.x - s, c.y + s), Pos2::new(c.x + s, c.y - s)],
            stroke,
        );
    }
}
