//! 共享 UI 组件库。提供跨工作区复用的通用 UI 组件。
//!
use super::theme::StudioTheme;
use super::theme::StudioThemeMode;
use eframe::egui::{self, RichText};

/// metric row。
pub(super) fn metric_row(ui: &mut egui::Ui, mode: StudioThemeMode, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(StudioTheme::muted_for(mode, label));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(value).color(StudioTheme::palette(mode).text));
        });
    });
}
