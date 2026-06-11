/// Toolbar widget helpers: icon-style buttons, document tabs, and signal rows.
///
/// Provides compact reusable widgets for the schematic toolbar, document tab bar,
/// and bottom-dock console output. Each widget respects the current theme mode.
use super::theme::{StudioTheme, StudioThemeMode};
use eframe::egui::{self, Color32, CornerRadius, Response, RichText, Stroke};

/// Draw a toolbar button with icon prefix.
///
/// Uses a compact monospace icon + label style for toolbar actions.
/// Disabled buttons are visually muted.
pub(super) fn canvas_toolbar_button(
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
    label: &str,
    enabled: bool,
) -> Response {
    let palette = StudioTheme::palette(mode);
    let text = RichText::new(label)
        .size(12.0)
        .color(if enabled { palette.text } else { palette.text_muted });
    let btn = egui::Button::new(text)
        .fill(palette.panel_soft)
        .stroke(Stroke::new(1.0, palette.border))
        .corner_radius(CornerRadius::same(4));
    let response = ui.add_enabled(enabled, btn);
    // Apply hover effect by painting over
    if response.hovered() && enabled {
        let painter = ui.painter();
        painter.rect_filled(
            response.rect,
            CornerRadius::same(4),
            palette.panel_hover,
        );
        // Re-draw the text on top
        painter.text(
            response.rect.center(),
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::proportional(12.0),
            palette.text,
        );
    }
    response
}

/// Draw a compact icon-only button for toolbar actions.
///
/// The button shows a Unicode symbol at the given size.
/// Hover state highlights the button border for visual feedback.
pub(super) fn toolbar_icon_button(
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
    icon: &str,
    tooltip: &str,
    enabled: bool,
) -> Response {
    let palette = StudioTheme::palette(mode);
    let text = RichText::new(icon)
        .size(14.0)
        .color(if enabled { palette.text } else { palette.text_muted });
    let btn = egui::Button::new(text)
        .fill(palette.panel_soft)
        .stroke(Stroke::new(1.0, palette.border))
        .corner_radius(CornerRadius::same(4));
    let response = ui.add_enabled(enabled, btn).on_hover_text(tooltip);
    // Hover feedback
    if response.hovered() && enabled {
        let painter = ui.painter();
        painter.rect_filled(
            response.rect,
            CornerRadius::same(4),
            palette.panel_hover,
        );
        painter.text(
            response.rect.center(),
            egui::Align2::CENTER_CENTER,
            icon,
            egui::FontId::proportional(14.0),
            palette.text,
        );
    }
    response
}

/// Draw a document tab in the tab bar.
///
/// Active tab uses accent fill; inactive tabs use panel background.
pub(super) fn document_tab(ui: &mut egui::Ui, mode: StudioThemeMode, text: &str, active: bool) -> Response {
    let palette = StudioTheme::palette(mode);
    let fill = if active {
        palette.accent_soft
    } else {
        palette.panel_soft
    };
    let stroke = if active {
        Stroke::new(1.0, palette.accent)
    } else {
        Stroke::new(1.0, palette.border)
    };
    let label = RichText::new(text)
        .size(12.0)
        .color(if active { palette.accent } else { palette.text_muted });
    ui.add(
        egui::Button::new(label)
            .fill(fill)
            .stroke(stroke)
            .corner_radius(CornerRadius::same(4)),
    )
}

/// Draw a signal row in the waveform/signal list panel.
///
/// Shows a colored dot, signal name, and scale label.
pub(super) fn signal_row(
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
    signal: &str,
    scale: &str,
    color: Color32,
) {
    ui.horizontal(|ui| {
        ui.colored_label(color, "\u{25CF}"); // solid circle
        ui.label(RichText::new(signal).size(12.0).color(StudioTheme::palette(mode).text));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(StudioTheme::muted_for(mode, scale));
        });
    });
}

/// Draw a console output line with the given color.
///
/// Used in the bottom dock console tab for status messages, errors, and info.
pub(super) fn bottom_console_line(
    ui: &mut egui::Ui,
    _mode: StudioThemeMode,
    text: &str,
    color: Color32,
) {
    ui.label(RichText::new(text).monospace().size(12.0).color(color));
}

/// Separator dot used between toolbar sections.
#[allow(dead_code)]
pub(super) fn toolbar_separator_dot(ui: &mut egui::Ui, mode: StudioThemeMode) {
    let palette = StudioTheme::palette(mode);
    ui.label(
        RichText::new("\u{2022}") // bullet
            .size(8.0)
            .color(palette.border),
    );
}
