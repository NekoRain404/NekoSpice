use super::theme::StudioTheme;
use super::theme::StudioThemeMode;
use eframe::egui::{self, RichText};

pub(super) fn metric_row(ui: &mut egui::Ui, mode: StudioThemeMode, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(StudioTheme::muted_for(mode, label));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(value).color(StudioTheme::palette(mode).text));
        });
    });
}
