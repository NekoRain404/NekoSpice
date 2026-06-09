use super::theme::StudioTheme;
use eframe::egui::{self, RichText};

pub(super) fn metric_row(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(StudioTheme::muted(label));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(value).color(StudioTheme::TEXT));
        });
    });
}
