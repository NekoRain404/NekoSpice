//! Shared widget helpers for the library workspace (symbol chips, filter rows).

use crate::app::theme::{StudioTheme, StudioThemeMode};
use eframe::egui::{self, RichText};

/// library filter tab。
pub(crate) fn library_filter_tab(
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
    label: &str,
    count: usize,
    active: bool,
) -> bool {
    let palette = StudioTheme::palette(mode);
    let text = if active {
        RichText::new(format!("{label}  {count}"))
            .strong()
            .color(palette.text)
    } else {
        StudioTheme::muted_for(mode, format!("{label}  {count}"))
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

/// symbol list row。
pub(crate) fn symbol_list_row(
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
    id: &str,
    detail: &str,
    stats: &str,
    active: bool,
) -> bool {
    let palette = StudioTheme::palette(mode);
    let fill = if active {
        palette.accent_soft
    } else {
        palette.panel
    };
    egui::Frame::new()
        .fill(fill)
        .stroke(egui::Stroke::new(
            1.0,
            if active {
                palette.accent
            } else {
                palette.border
            },
        ))
        .corner_radius(5)
        .inner_margin(egui::Margin::same(8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new(id).strong().color(palette.text));
                    ui.label(StudioTheme::muted_for(mode, detail));
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(StudioTheme::accent_for(mode, stats));
                });
            });
        })
        .response
        .interact(egui::Sense::click())
        .clicked()
}

/// code line。
pub(crate) fn code_line(ui: &mut egui::Ui, line_number: usize, text: &str) {
    ui.horizontal(|ui| {
        ui.monospace(format!("{line_number:>2}"));
        ui.monospace(text);
    });
}

/// metadata row。
pub(crate) fn metadata_row(ui: &mut egui::Ui, mode: StudioThemeMode, label: &str, value: &str) {
    let palette = StudioTheme::palette(mode);
    ui.horizontal(|ui| {
        ui.label(StudioTheme::muted_for(mode, label));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(value).color(palette.text));
        });
    });
}

/// library metric card。
pub(crate) fn library_metric_card(
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
    label: &str,
    value: &str,
    caption: &str,
) {
    let palette = StudioTheme::palette(mode);
    StudioTheme::panel_frame_for(mode).show(ui, |ui| {
        ui.set_min_height(74.0);
        ui.label(StudioTheme::muted_for(mode, label));
        ui.label(RichText::new(value).strong().size(18.0).color(palette.text));
        ui.label(RichText::new(caption).size(11.0).color(palette.text_muted));
    });
}

/// pin mapping row。
pub(crate) fn pin_mapping_row(
    ui: &mut egui::Ui,
    number: &str,
    name: &str,
    kind: &str,
    target: &str,
) {
    ui.monospace(number);
    ui.label(name);
    ui.label(kind);
    ui.label(target);
    ui.end_row();
}

/// validation row。
pub(crate) fn validation_row(
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
    label: &str,
    value: &str,
    passed: bool,
) {
    let palette = StudioTheme::palette(mode);
    let color = if passed {
        palette.success
    } else {
        palette.warning
    };
    ui.horizontal(|ui| {
        ui.label(StudioTheme::muted_for(mode, label));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(value).strong().color(color));
        });
    });
}
