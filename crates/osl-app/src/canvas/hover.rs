//! 悬停高亮绘制。为鼠标悬停的元素绘制半透明高亮边框。

use crate::viewport::CanvasViewport;
use super::colors::SchematicColors;
use super::primitives;
use eframe::egui;

/// 在悬停元素的边界框周围绘制半透明高亮。
///
/// 相比选中高亮使用更浅的颜色，为用户提供点击前的视觉反馈。
pub(crate) fn draw_hover_highlight(
    painter: &egui::Painter,
    rect: egui::Rect,
    viewport: CanvasViewport,
    bounds: osl_kicad::KicadBoundingBox,
    colors: SchematicColors,
) {
    primitives::draw_bounds(painter, rect, viewport, bounds, colors.hover_highlight, 1.5);
}
