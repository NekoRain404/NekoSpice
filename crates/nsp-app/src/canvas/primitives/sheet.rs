//! 图纸边界渲染图元。绘制原理图边框和标题栏。
//!
use super::super::colors::SchematicColors;
use crate::viewport::CanvasViewport;
use eframe::egui::{self, Align2, FontId, Rect, Stroke, StrokeKind, Vec2};
use nsp_schema::{NspCanvasSheet, NspPoint};

/// draw sheet。
pub(crate) fn draw_sheet(
    painter: &egui::Painter,
    rect: Rect,
    viewport: CanvasViewport,
    sheet: &NspCanvasSheet,
    sc: SchematicColors,
) {
    let Some(at) = sheet.at else {
        return;
    };
    let Some(size) = sheet.size else {
        return;
    };
    let start = viewport.world_to_screen(rect, NspPoint { x: at.x, y: at.y });
    let end = viewport.world_to_screen(
        rect,
        NspPoint {
            x: at.x + size.width,
            y: at.y + size.height,
        },
    );
    let sheet_rect = Rect::from_two_pos(start, end);
    painter.rect_filled(sheet_rect, 0.0, sc.sheet_fill);
    painter.rect_stroke(
        sheet_rect,
        0.0,
        Stroke::new(1.5, sc.sheet_border),
        StrokeKind::Inside,
    );

    // Sheet name at top-left
    painter.text(
        sheet_rect.left_top() + Vec2::new(4.0, 4.0),
        Align2::LEFT_TOP,
        &sheet.name,
        FontId::monospace(12.0),
        sc.sheet_name,
    );

    // Draw sheet pins (small lines + label on the sheet border)
    for pin in &sheet.pins {
        if let Some(pin_at) = pin.at {
            let pin_screen = viewport.world_to_screen(
                rect,
                NspPoint {
                    x: pin_at.x,
                    y: pin_at.y,
                },
            );
            // Determine pin direction based on position relative to sheet rect
            let on_left = (pin_screen.x - sheet_rect.left()).abs() < 5.0;
            let on_right = (pin_screen.x - sheet_rect.right()).abs() < 5.0;
            let pin_length = 6.0;

            let (line_start, line_end, text_align) = if on_left {
                (
                    pin_screen,
                    pin_screen + Vec2::new(-pin_length, 0.0),
                    Align2::RIGHT_CENTER,
                )
            } else if on_right {
                (
                    pin_screen,
                    pin_screen + Vec2::new(pin_length, 0.0),
                    Align2::LEFT_CENTER,
                )
            } else {
                // Top or bottom pin
                let on_top = (pin_screen.y - sheet_rect.top()).abs() < 5.0;
                if on_top {
                    (
                        pin_screen,
                        pin_screen + Vec2::new(0.0, -pin_length),
                        Align2::CENTER_BOTTOM,
                    )
                } else {
                    (
                        pin_screen,
                        pin_screen + Vec2::new(0.0, pin_length),
                        Align2::CENTER_TOP,
                    )
                }
            };

            // Draw pin stub line
            painter.line_segment([line_start, line_end], Stroke::new(1.5, sc.sheet_pin));

            // Draw pin label
            let text_offset = match text_align {
                Align2::RIGHT_CENTER => Vec2::new(-4.0, 0.0),
                Align2::LEFT_CENTER => Vec2::new(4.0, 0.0),
                Align2::CENTER_BOTTOM => Vec2::new(0.0, -4.0),
                Align2::CENTER_TOP => Vec2::new(0.0, 4.0),
                _ => Vec2::ZERO,
            };
            let font_size = pin
                .effects
                .as_ref()
                .and_then(|e| e.font_size)
                .map(|s| s.width as f32)
                .unwrap_or(10.0)
                .max(6.0);
            painter.text(
                pin_screen + text_offset,
                text_align,
                &pin.name,
                FontId::proportional(font_size),
                sc.sheet_pin,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Graphic element drawing
// ---------------------------------------------------------------------------
