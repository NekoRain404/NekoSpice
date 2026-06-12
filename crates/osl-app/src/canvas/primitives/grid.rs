//! 网格渲染图元。绘制参考网格线。
//!
use crate::viewport::CanvasViewport;
use super::super::colors::SchematicColors;
use eframe::egui::{self, Pos2, Rect, Stroke};

/// draw grid。
pub(crate) fn draw_grid(
    painter: &egui::Painter,
    rect: Rect,
    viewport: CanvasViewport,
    sc: SchematicColors,
) {
    let minor_step = (2.54 * viewport.zoom).max(4.0);
    let major_step = minor_step * 5.0;

    let origin = rect.center() + viewport.pan;
    let minor_stroke = Stroke::new(0.5, sc.grid_minor);
    let major_stroke = Stroke::new(1.0, sc.grid_major);

    let start_x = origin.x % minor_step;
    let mut x = start_x;
    while x < rect.width() {
        let screen_x = rect.left() + x;
        let is_major = (x % major_step).abs() < 0.5
            || ((major_step - (x % major_step)).abs() < 0.5);
        let stroke = if is_major { major_stroke } else { minor_stroke };
        painter.line_segment(
            [Pos2::new(screen_x, rect.top()), Pos2::new(screen_x, rect.bottom())],
            stroke,
        );
        x += minor_step;
    }

    let start_y = origin.y % minor_step;
    let mut y = start_y;
    while y < rect.height() {
        let screen_y = rect.top() + y;
        let is_major = (y % major_step).abs() < 0.5
            || ((major_step - (y % major_step)).abs() < 0.5);
        let stroke = if is_major { major_stroke } else { minor_stroke };
        painter.line_segment(
            [Pos2::new(rect.left(), screen_y), Pos2::new(rect.right(), screen_y)],
            stroke,
        );
        y += minor_step;
    }
}
