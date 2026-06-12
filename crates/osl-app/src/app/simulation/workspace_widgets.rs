//! Shared widget helpers for the simulation workspace.
//!
//! Provides reusable UI components: analysis mode buttons, metric cards,
//! code preview lines, profile summary rows, and status indicators.

use crate::app::theme::{StudioTheme, StudioThemeMode};
use eframe::egui::{self, RichText};

/// Interactive analysis mode button card with active state highlighting.
///
/// Returns `true` when clicked. Uses accent color for the active state
/// and muted styling for inactive buttons.
pub(crate) fn analysis_mode_button(
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
    title: &str,
    caption: &str,
    active: bool,
) -> bool {
    let palette = StudioTheme::palette(mode);
    egui::Frame::new()
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
        .corner_radius(5)
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.set_min_width(132.0);
            ui.set_min_height(54.0);
            ui.label(RichText::new(title).strong().color(palette.text));
            ui.label(RichText::new(caption).size(11.0).color(palette.text_muted));
        })
        .response
        .interact(egui::Sense::click())
        .clicked()
}

/// Solver metric card displaying a label, large value, and caption.
///
/// Used in the overview's 4-column metrics row to show solver engine,
/// status, netlist directives, and last run duration at a glance.
pub(crate) fn solver_metric_card(
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
    label: &str,
    value: &str,
    caption: &str,
) {
    let palette = StudioTheme::palette(mode);
    StudioTheme::panel_frame_for(mode).show(ui, |ui| {
        ui.set_min_height(72.0);
        ui.label(StudioTheme::muted_for(mode, label));
        ui.label(RichText::new(value).strong().size(18.0).color(palette.text));
        ui.label(RichText::new(caption).size(11.0).color(palette.text_muted));
    });
}

/// Status indicator card with a colored dot, label, and value.
///
/// Used for quick-glance status in the simulation workspace.
#[allow(dead_code)]
pub(crate) fn status_indicator_card(
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
    color: egui::Color32,
    label: &str,
    value: &str,
) {
    let palette = StudioTheme::palette(mode);
    StudioTheme::panel_frame_for(mode).show(ui, |ui| {
        ui.set_min_height(56.0);
        ui.horizontal(|ui| {
            ui.label(StudioTheme::status_dot(color));
            ui.vertical(|ui| {
                ui.label(StudioTheme::muted_for(mode, label));
                ui.label(RichText::new(value).strong().color(palette.text));
            });
        });
    });
}

/// Code preview line with line number gutter and monospace text.
///
/// Used in the netlist preview to show syntax with line numbers.
pub(crate) fn code_preview_line(ui: &mut egui::Ui, line_number: usize, text: &str) {
    ui.horizontal(|ui| {
        ui.monospace(format!("{line_number:>2}"));
        ui.monospace(text);
    });
}

/// Profile summary row showing a label, monospace value, and status tag.
///
/// Used in the configuration summary panel to display individual settings.
pub(crate) fn profile_row(
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
    label: &str,
    value: &str,
    status: &str,
) {
    let palette = StudioTheme::palette(mode);
    ui.horizontal(|ui| {
        ui.label(StudioTheme::muted_for(mode, label));
        ui.label(RichText::new(value).monospace().color(palette.text));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(StudioTheme::accent_for(mode, status));
        });
    });
}
