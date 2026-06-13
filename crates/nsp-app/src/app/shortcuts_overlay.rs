//! 快捷键帮助叠加层。按 ? 弹出的全屏快捷键说明面板。
//!
use super::NekoSpiceApp;
use super::theme::StudioTheme;
use crate::canvas::colors::SchematicColors;
use eframe::egui::{self, Align2, CornerRadius, FontId, Rect, Stroke, Vec2};

/// Shortcut entry: (key combination, description)
const SHORTCUTS: &[(&str, &str)] = &[
    // Tools
    ("V", "Select tool"),
    ("W", "Wire tool"),
    ("L", "Label tool"),
    ("B", "Bus tool"),
    ("S", "Sheet tool"),
    ("J", "Junction tool"),
    ("Q", "No-connect tool"),
    // Edit
    ("R", "Rotate selected 90\u{00B0}"),
    ("Del", "Delete selected"),
    ("\u{2190}\u{2191}\u{2192}\u{2193}", "Nudge selected item"),
    // Navigation
    ("F", "Fit view to schematic"),
    ("Scroll", "Zoom in/out"),
    ("Middle drag", "Pan canvas"),
    // History
    ("Ctrl+Z", "Undo"),
    ("Ctrl+Shift+Z / Ctrl+Y", "Redo"),
    // File
    ("Ctrl+O", "Open schematic"),
    ("Ctrl+S", "Save"),
    ("Ctrl+Shift+S", "Save As"),
    // Simulation
    ("F5", "Run simulation"),
    ("Ctrl+Shift+E", "Export netlist"),
    // Help
    ("?", "Toggle this help"),
    ("Esc", "Cancel / Switch to Select"),
];

impl NekoSpiceApp {
    /// Draw a semi-transparent overlay listing keyboard shortcuts.
    pub(super) fn draw_shortcuts_overlay(
        &self,
        painter: &eframe::egui::Painter,
        canvas_rect: Rect,
        _colors: SchematicColors,
    ) {
        let mode = self.theme_mode();
        let palette = StudioTheme::palette(mode);

        let panel_width = 300.0;
        let line_height = 20.0;
        let padding = 12.0;
        let header_height = 28.0;
        let panel_height = header_height + (SHORTCUTS.len() as f32 * line_height) + padding * 2.0;

        let panel_rect = Rect::from_min_size(
            canvas_rect.right_top() + Vec2::new(-panel_width - 10.0, 10.0),
            Vec2::new(panel_width, panel_height),
        );

        painter.rect_filled(
            panel_rect,
            CornerRadius::same(8),
            egui::Color32::from_rgba_premultiplied(10, 18, 30, 220),
        );
        painter.rect_stroke(
            panel_rect,
            CornerRadius::same(8),
            Stroke::new(1.0, palette.border),
            egui::StrokeKind::Inside,
        );

        painter.text(
            panel_rect.left_top() + Vec2::new(padding, padding),
            Align2::LEFT_TOP,
            "Keyboard Shortcuts",
            FontId::proportional(13.0),
            palette.accent,
        );

        let mut y = panel_rect.top() + header_height + padding;
        for (key, desc) in SHORTCUTS {
            painter.text(
                panel_rect.left_top() + Vec2::new(padding, y),
                Align2::LEFT_TOP,
                key,
                FontId::monospace(11.0),
                palette.text,
            );
            painter.text(
                panel_rect.left_top() + Vec2::new(padding + 140.0, y),
                Align2::LEFT_TOP,
                desc,
                FontId::proportional(11.0),
                palette.text_muted,
            );
            y += line_height;
        }
    }
}
