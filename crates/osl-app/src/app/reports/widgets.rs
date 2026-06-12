use crate::app::theme::{StudioTheme, StudioThemeMode};
use eframe::egui::{self, RichText};

/// report metric card。
pub(crate) fn report_metric_card(
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
    label: &str,
    value: &str,
    caption: &str,
) {
    let palette = StudioTheme::palette(mode);
    StudioTheme::panel_frame_for(mode).show(ui, |ui| {
        ui.set_min_height(78.0);
        ui.set_min_width(150.0);
        ui.label(StudioTheme::muted_for(mode, label));
        ui.label(RichText::new(value).strong().size(20.0).color(palette.text));
        ui.label(RichText::new(caption).size(11.0).color(palette.text_muted));
    });
}

/// report row。
pub(crate) fn report_row(ui: &mut egui::Ui, mode: StudioThemeMode, label: &str, value: &str) {
    let palette = StudioTheme::palette(mode);
    ui.horizontal(|ui| {
        ui.label(StudioTheme::muted_for(mode, label));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(value).color(palette.text));
        });
    });
}

/// artifact row。
pub(crate) fn artifact_row(ui: &mut egui::Ui, kind: &str, file: &str, selected: bool) {
    ui.horizontal(|ui| {
        if selected {
            ui.strong(kind);
            ui.strong(file);
        } else {
            ui.label(kind);
            ui.monospace(file);
        }
    });
}

/// report status card。
pub(crate) fn report_status_card(
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
    label: &str,
    value: &str,
    caption: &str,
    emphasized: bool,
) {
    let palette = StudioTheme::palette(mode);
    StudioTheme::panel_frame_for(mode).show(ui, |ui| {
        ui.set_min_height(86.0);
        ui.label(StudioTheme::muted_for(mode, label));
        let value_color = if emphasized {
            palette.success
        } else {
            palette.text
        };
        ui.label(RichText::new(value).strong().size(19.0).color(value_color));
        ui.label(RichText::new(caption).size(11.0).color(palette.text_muted));
    });
}

/// formula token。
pub(crate) fn formula_token(ui: &mut egui::Ui, mode: StudioThemeMode, token: &str) {
    let palette = StudioTheme::palette(mode);
    egui::Frame::new()
        .fill(palette.panel_soft)
        .stroke(egui::Stroke::new(1.0, palette.border))
        .corner_radius(egui::CornerRadius::same(4))
        .inner_margin(egui::Margin::symmetric(8, 4))
        .show(ui, |ui| {
            ui.monospace(token);
        });
}

/// export toggle。
pub(crate) fn export_toggle(ui: &mut egui::Ui, mode: StudioThemeMode, label: &str, enabled: bool) {
    let palette = StudioTheme::palette(mode);
    ui.horizontal(|ui| {
        let marker = if enabled { "[x]" } else { "[ ]" };
        ui.label(RichText::new(marker).monospace().color(palette.accent));
        ui.label(label);
    });
}
