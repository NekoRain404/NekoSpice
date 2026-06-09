use super::theme::{StudioTheme, StudioThemeMode};
use eframe::egui::{self, RichText};

pub(super) fn report_metric_card(
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

pub(super) fn report_row(ui: &mut egui::Ui, mode: StudioThemeMode, label: &str, value: &str) {
    let palette = StudioTheme::palette(mode);
    ui.horizontal(|ui| {
        ui.label(StudioTheme::muted_for(mode, label));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(value).color(palette.text));
        });
    });
}

pub(super) fn artifact_row(ui: &mut egui::Ui, kind: &str, file: &str, selected: bool) {
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
