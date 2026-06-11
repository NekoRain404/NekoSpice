use super::theme::{StudioTheme, StudioThemeMode};
use eframe::egui::{self, Color32, Response, RichText};

pub(super) fn canvas_toolbar_button(
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
    text: &str,
    enabled: bool,
) -> Response {
    let palette = StudioTheme::palette(mode);
    ui.add_enabled(
        enabled,
        egui::Button::new(text)
            .fill(palette.panel_soft)
            .stroke(egui::Stroke::new(1.0, palette.border))
            .corner_radius(4),
    )
}

pub(super) fn document_tab(ui: &mut egui::Ui, mode: StudioThemeMode, text: &str, active: bool) {
    let palette = StudioTheme::palette(mode);
    let fill = if active {
        palette.accent_soft
    } else {
        palette.panel_soft
    };
    let stroke = if active {
        palette.accent
    } else {
        palette.border
    };
    let label = RichText::new(text).color(palette.text);
    let _ = ui.add(
        egui::Button::new(label)
            .fill(fill)
            .stroke(egui::Stroke::new(1.0, stroke))
            .corner_radius(4),
    );
}

pub(super) fn signal_row(
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
    signal: &str,
    scale: &str,
    color: Color32,
) {
    ui.horizontal(|ui| {
        ui.colored_label(color, "*");
        ui.label(RichText::new(signal).color(StudioTheme::palette(mode).text));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(StudioTheme::muted_for(mode, scale));
        });
    });
}

pub(super) fn bottom_console_line(
    ui: &mut egui::Ui,
    _mode: StudioThemeMode,
    text: &str,
    color: Color32,
) {
    ui.label(RichText::new(text).monospace().color(color));
}
