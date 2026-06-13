//! Simulation waveform panel — compact waveform preview in the simulation sidebar.

use crate::app::theme::StudioThemeMode;
use crate::app::waveform::preview::{draw_single_waveform_preview, format_compact_f64};
use crate::waveform_summary::{
    GuiWaveformSummary, GuiWaveformSummaryState, GuiWaveformVariableSummary,
};
use eframe::egui::{self, Color32};

/// draw simulation waveform panel。
pub(crate) fn draw_simulation_waveform_panel(
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
    waveform: &GuiWaveformSummaryState,
    selected_signal: &mut Option<String>,
) {
    match waveform {
        GuiWaveformSummaryState::Ready(summary) => {
            draw_ready_waveform_summary(ui, mode, summary, selected_signal);
        }
        GuiWaveformSummaryState::Missing { raw_path } => {
            ui.label("Waveform: no waveform.raw");
            ui.monospace(raw_path.display().to_string());
        }
        GuiWaveformSummaryState::Error { raw_path, message } => {
            ui.colored_label(
                Color32::from_rgb(180, 120, 20),
                format!("Waveform parse failed: {message}"),
            );
            ui.monospace(raw_path.display().to_string());
        }
    }
}

fn draw_ready_waveform_summary(
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
    summary: &GuiWaveformSummary,
    selected_signal: &mut Option<String>,
) {
    let palette = crate::app::theme::StudioTheme::palette(mode);

    // Waveform metadata header
    ui.label(crate::app::theme::StudioTheme::section_title_for(mode, "Waveform Data"));
    ui.add_space(2.0);
    ui.horizontal_wrapped(|ui| {
        ui.label(
            egui::RichText::new(format!("{} points", summary.point_count))
                .monospace().size(11.0).color(palette.text_muted),
        );
        ui.separator();
        ui.label(
            egui::RichText::new(format!("{} variables", summary.variable_count))
                .monospace().size(11.0).color(palette.text_muted),
        );
        if summary.omitted_variable_count > 0 {
            ui.separator();
            ui.label(
                egui::RichText::new(format!("+{} omitted", summary.omitted_variable_count))
                    .monospace().size(11.0).color(palette.warning),
            );
        }
    });
    if !summary.title.is_empty() {
        ui.label(egui::RichText::new(&summary.title).size(11.0).color(palette.text));
    }

    draw_waveform_preview_selector(ui, summary, selected_signal);
    if let Some(signal) = selected_signal.as_deref()
        && let Some(preview) = summary.preview_for_signal(signal)
    {
        draw_single_waveform_preview(ui, mode, preview, 120.0);
        if let Some(variable) = summary.variable_summary_for_signal(signal) {
            draw_selected_measurements(ui, variable);
        }
    }

    draw_waveform_variable_table(ui, summary);
}

fn draw_waveform_preview_selector(
    ui: &mut egui::Ui,
    summary: &GuiWaveformSummary,
    selected_signal: &mut Option<String>,
) {
    if summary.previews.is_empty() {
        ui.label("Waveform preview: no plottable signals");
        return;
    }

    if selected_signal
        .as_deref()
        .is_none_or(|signal| !summary.has_preview_signal(signal))
    {
        *selected_signal = summary.default_signal_name().map(ToOwned::to_owned);
    }

    let mut selected = selected_signal
        .clone()
        .unwrap_or_else(|| summary.previews[0].signal.clone());
    ui.horizontal(|ui| {
        ui.label("Signal");
        egui::ComboBox::from_id_salt("simulation_waveform_signal")
            .selected_text(selected.clone())
            .show_ui(ui, |ui| {
                for preview in &summary.previews {
                    ui.selectable_value(&mut selected, preview.signal.clone(), &preview.signal);
                }
            });
        if summary.omitted_preview_count > 0 {
            ui.label(format!("{} more", summary.omitted_preview_count));
        }
    });
    *selected_signal = Some(selected);
}

fn draw_selected_measurements(ui: &mut egui::Ui, variable: &GuiWaveformVariableSummary) {
    egui::Grid::new("simulation_selected_waveform_measurements")
        .num_columns(4)
        .spacing(egui::Vec2::new(10.0, 2.0))
        .show(ui, |ui| {
            measurement_cell(ui, "First", variable.first);
            measurement_cell(ui, "Last", variable.last);
            measurement_cell(ui, "Avg", variable.avg);
            measurement_cell(ui, "RMS", variable.rms);
            ui.end_row();
            measurement_cell(ui, "Min", variable.min);
            measurement_cell(ui, "Max", variable.max);
            measurement_cell(ui, "P-P", variable.peak_to_peak);
            ui.label(format!("{} samples", variable.samples));
            ui.end_row();
        });
}

fn measurement_cell(ui: &mut egui::Ui, label: &str, value: f64) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.monospace(format_compact_f64(value));
    });
}

fn draw_waveform_variable_table(ui: &mut egui::Ui, summary: &GuiWaveformSummary) {
    egui::Grid::new("simulation_waveform_summary")
        .num_columns(5)
        .spacing(egui::Vec2::new(8.0, 2.0))
        .striped(true)
        .show(ui, |ui| {
            ui.strong("Signal");
            ui.strong("Last");
            ui.strong("Min");
            ui.strong("Max");
            ui.strong("P-P");
            ui.end_row();
            for variable in &summary.variables {
                ui.label(variable_label(
                    &variable.name,
                    &variable.unit,
                    variable.samples,
                ))
                .on_hover_text(variable_hover_text(
                    variable.first,
                    variable.avg,
                    variable.rms,
                ));
                ui.monospace(format_compact_f64(variable.last));
                ui.monospace(format_compact_f64(variable.min));
                ui.monospace(format_compact_f64(variable.max));
                ui.monospace(format_compact_f64(variable.peak_to_peak));
                ui.end_row();
            }
        });
    if summary.omitted_variable_count > 0 {
        ui.label(format!("{} more variables", summary.omitted_variable_count));
    }
}

fn variable_hover_text(first: f64, avg: f64, rms: f64) -> String {
    format!(
        "First: {}\nAvg: {}\nRMS: {}",
        format_compact_f64(first),
        format_compact_f64(avg),
        format_compact_f64(rms)
    )
}

fn variable_label(name: &str, unit: &str, samples: usize) -> String {
    if unit.is_empty() {
        format!("{name} ({samples})")
    } else {
        format!("{name} [{unit}] ({samples})")
    }
}
