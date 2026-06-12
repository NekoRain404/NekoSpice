/// Rotated text rendering for KiCad schematics.
///
/// Egui lacks native rotated text support, so we use character-by-character
/// placement through a 2D rotation transform. Cardinal angles are optimized.
use crate::viewport::CanvasViewport;
use eframe::egui::{self, Align2, Color32, FontId, Rect, Vec2};

pub(crate) fn draw_rotated_text(
    painter: &egui::Painter,
    rect: Rect,
    viewport: CanvasViewport,
    at: osl_kicad::KicadAt,
    text: &str,
    font_size: f32,
    color: Color32,
) {
    let rotation = ((at.rotation % 360.0) + 360.0) % 360.0;
    let screen_pos = viewport.world_to_screen(
        rect,
        osl_kicad::KicadPoint { x: at.x, y: at.y },
    );

    // Near-zero rotation: direct native rendering
    if rotation.abs() < 0.1 || (rotation - 360.0).abs() < 0.1 {
        painter.text(
            screen_pos,
            Align2::LEFT_TOP,
            text,
            FontId::proportional(font_size),
            color,
        );
        return;
    }

    // Cardinal rotation 180°: flip alignment
    if (rotation - 180.0).abs() < 0.1 {
        painter.text(
            screen_pos,
            Align2::RIGHT_BOTTOM,
            text,
            FontId::proportional(font_size),
            color,
        );
        return;
    }

    // Character-by-character rotated rendering for 90°, 270° and arbitrary angles
    let char_spacing = font_size * 0.65;
    let radians = -rotation.to_radians();
    let cos = radians.cos() as f32;
    let sin = radians.sin() as f32;

    for (i, ch) in text.chars().enumerate() {
        let local_x = i as f32 * char_spacing;
        let rotated_x = local_x * cos;
        let rotated_y = local_x * sin;
        let char_pos = screen_pos + Vec2::new(rotated_x, rotated_y);
        painter.text(
            char_pos,
            Align2::LEFT_TOP,
            ch.to_string(),
            FontId::proportional(font_size),
            color,
        );
    }
}

// ---------------------------------------------------------------------------
// Selection bounds
// ---------------------------------------------------------------------------

