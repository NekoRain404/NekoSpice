//! Waveform workspace UI widgets — tabs, trace chips, measurement tables,
//! cursor rows, and summary cards. Pure rendering helpers with no state.
//!
//! `trace_chip` supports both single-select and multi-select modes.

use super::preview::format_compact_f64;
use crate::app::theme::{StudioTheme, StudioThemeMode};
use crate::waveform_summary::GuiWaveformVariableSummary;
use eframe::egui::{self, RichText};

/// Tab button for switching waveform analysis modes.
pub(crate) fn waveform_mode_tab(
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
    label: &str,
    selected: bool,
) -> bool {
    let palette = StudioTheme::palette(mode);
    ui.add_sized(
        [104.0, 30.0],
        egui::Button::new(label)
            .fill(if selected {
                palette.accent_soft
            } else {
                palette.panel_soft
            })
            .stroke(egui::Stroke::new(
                1.0,
                if selected {
                    palette.accent
                } else {
                    palette.border
                },
            ))
            .corner_radius(5),
    )
    .clicked()
}

/// Single-select trace chip — highlights when active.
pub(crate) fn trace_chip(
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
    signal: &str,
    unit: &str,
    selected: bool,
) -> bool {
    let palette = StudioTheme::palette(mode);
    let caption = if unit.is_empty() {
        signal.to_string()
    } else {
        format!("{signal}  {unit}")
    };
    ui.add(
        egui::Button::new(RichText::new(caption).monospace())
            .fill(if selected {
                palette.accent_soft
            } else {
                palette.panel_soft
            })
            .stroke(egui::Stroke::new(
                1.0,
                if selected {
                    palette.accent
                } else {
                    palette.border
                },
            ))
            .corner_radius(5),
    )
    .clicked()
}

/// Multi-select trace chip — toggles visibility in overlay mode.
/// Returns whether the button was clicked.
pub(crate) fn trace_chip_toggle(
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
    signal: &str,
    unit: &str,
    visible: bool,
) -> bool {
    let palette = StudioTheme::palette(mode);
    let caption = if unit.is_empty() {
        signal.to_string()
    } else {
        format!("{signal}  {unit}")
    };
    ui.add(
        egui::Button::new(RichText::new(caption).monospace())
            .fill(if visible {
                palette.accent_soft
            } else {
                palette.panel_soft
            })
            .stroke(egui::Stroke::new(
                1.0,
                if visible {
                    palette.accent
                } else {
                    palette.border
                },
            ))
            .corner_radius(5),
    )
    .clicked()
}

/// Empty state placeholder for waveform workspace.
pub(crate) fn waveform_empty_state(
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
    title: &str,
    caption: &str,
) {
    let palette = StudioTheme::palette(mode);
    ui.set_min_height(330.0);
    ui.vertical_centered(|ui| {
        ui.add_space(118.0);
        ui.label(RichText::new(title).strong().size(18.0).color(palette.text));
        ui.label(RichText::new(caption).color(palette.text_muted));
    });
}

/// Measurement table showing signal statistics.
pub(crate) fn measurement_table(
    ui: &mut egui::Ui,
    labels: &MeasurementTableLabels<'_>,
    variables: &[GuiWaveformVariableSummary],
    limit: usize,
) {
    egui::Grid::new("waveform_workspace_measurements")
        .num_columns(6)
        .spacing(egui::Vec2::new(12.0, 5.0))
        .striped(true)
        .show(ui, |ui| {
            ui.strong(labels.signal);
            ui.strong(labels.last);
            ui.strong(labels.average);
            ui.strong(labels.rms);
            ui.strong(labels.peak_to_peak);
            ui.strong(labels.samples);
            ui.end_row();
            for variable in variables.iter().take(limit) {
                ui.label(&variable.name);
                ui.monospace(format_compact_f64(variable.last));
                ui.monospace(format_compact_f64(variable.avg));
                ui.monospace(format_compact_f64(variable.rms));
                ui.monospace(format_compact_f64(variable.peak_to_peak));
                ui.monospace(variable.samples.to_string());
                ui.end_row();
            }
        });
    if variables.len() > limit {
        ui.label(format!(
            "{} {}",
            variables.len() - limit,
            labels.more_variables
        ));
    }
}

/// Labels for measurement table columns (supports localization).
pub(crate) struct MeasurementTableLabels<'a> {
    pub(crate) signal: &'a str,
    pub(crate) last: &'a str,
    pub(crate) average: &'a str,
    pub(crate) rms: &'a str,
    pub(crate) peak_to_peak: &'a str,
    pub(crate) samples: &'a str,
    pub(crate) more_variables: &'a str,
}

/// Single row in run statistics display.
pub(crate) fn run_stat_row(ui: &mut egui::Ui, mode: StudioThemeMode, label: &str, value: &str) {
    let palette = StudioTheme::palette(mode);
    ui.horizontal(|ui| {
        ui.label(StudioTheme::muted_for(mode, label));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(value).color(palette.text));
        });
    });
    ui.separator();
}

/// Cursor readout row — shows signal name and interpolated value.
pub(crate) fn cursor_row(
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
    label: &str,
    signal: &str,
    value: &str,
) {
    let palette = StudioTheme::palette(mode);
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).strong().color(palette.accent));
        ui.label(RichText::new(signal).monospace().color(palette.text));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(value).monospace().color(palette.text_muted));
        });
    });
}

/// Summary card for a completed simulation run.
pub(crate) fn waveform_summary_card(
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
    title: &str,
    status: &str,
    path: &str,
) {
    let palette = StudioTheme::palette(mode);
    egui::Frame::new()
        .fill(palette.panel_soft)
        .stroke(egui::Stroke::new(1.0, palette.border))
        .corner_radius(5)
        .inner_margin(egui::Margin::same(8))
        .show(ui, |ui| {
            ui.label(RichText::new(title).strong().color(palette.text));
            ui.label(StudioTheme::accent_for(mode, status));
            ui.label(RichText::new(path).size(11.0).color(palette.text_muted));
        });
}
