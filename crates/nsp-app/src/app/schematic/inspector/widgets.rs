//! Schematic inspector widgets — reusable UI components.

use crate::app::theme::{StudioTheme, StudioThemeMode};
use eframe::egui::{self, Color32, RichText};

/// inspector tab。
pub(crate) fn inspector_tab(
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
    label: &str,
    active: bool,
) -> bool {
    let palette = StudioTheme::palette(mode);
    let text = if active {
        RichText::new(label).strong().color(palette.text)
    } else {
        StudioTheme::muted_for(mode, label)
    };
    ui.add(
        egui::Button::new(text)
            .fill(if active {
                palette.accent_soft
            } else {
                palette.panel_soft
            })
            .stroke(egui::Stroke::new(
                1.0,
                if active {
                    palette.accent
                } else {
                    palette.border
                },
            ))
            .corner_radius(4),
    )
    .clicked()
}

/// property row。
pub(crate) fn property_row(ui: &mut egui::Ui, mode: StudioThemeMode, label: &str, value: &str) {
    let palette = StudioTheme::palette(mode);
    ui.horizontal(|ui| {
        ui.label(StudioTheme::muted_for(mode, label));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(value).color(palette.text));
        });
    });
}

/// status pill。
pub(crate) fn status_pill(ui: &mut egui::Ui, mode: StudioThemeMode, label: &str, color: Color32) {
    let palette = StudioTheme::palette(mode);
    egui::Frame::new()
        .fill(palette.panel_soft)
        .stroke(egui::Stroke::new(1.0, palette.border))
        .corner_radius(4)
        .inner_margin(egui::Margin::symmetric(7, 3))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(StudioTheme::status_dot(color));
                ui.label(RichText::new(label).color(palette.text));
            });
        });
}

/// compact action。
pub(crate) fn compact_action(ui: &mut egui::Ui, mode: StudioThemeMode, label: &str) -> bool {
    let palette = StudioTheme::palette(mode);
    ui.add(
        egui::Button::new(label)
            .fill(palette.panel_soft)
            .stroke(egui::Stroke::new(1.0, palette.border))
            .corner_radius(4),
    )
    .clicked()
}

/// section caption。
pub(crate) fn section_caption(ui: &mut egui::Ui, mode: StudioThemeMode, text: &str) {
    ui.label(StudioTheme::muted_for(mode, text));
}
