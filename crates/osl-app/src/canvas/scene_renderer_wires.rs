//! 原理图场景渲染 — 导线与总线层（Layer 5-6）。
//!
//! 包含导线（wires）和总线（buses）的绘制逻辑。
//! 连接关系的可视化是原理图的核心，这些图层决定了电气连接的清晰度。

use super::colors::SchematicColors;
use super::primitives;
use crate::viewport::{CanvasViewport, item_visible};
use eframe::egui;
use osl_kicad::{KicadBoundingBox, KicadCanvasScene};

/// Layer 5: 绘制所有导线。
///
/// 导线由折线段组成，每段用 1.5pt 宽度绘制。
pub(crate) fn draw_wires(
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

/// Layer 6: 绘制总线及其入口。
///
/// 总线宽度为 2.0pt，入口用专用图元绘制。
pub(crate) fn draw_buses(
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
