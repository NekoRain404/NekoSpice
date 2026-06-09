use crate::waveform_summary::{
    GuiWaveformPreview, GuiWaveformSummary, GuiWaveformSummaryState, GuiWaveformVariableSummary,
};
use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};

pub(super) fn draw_simulation_waveform_panel(
    ui: &mut egui::Ui,
    waveform: &GuiWaveformSummaryState,
    selected_signal: &mut Option<String>,
) {
    match waveform {
        GuiWaveformSummaryState::Ready(summary) => {
            draw_ready_waveform_summary(ui, summary, selected_signal);
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
    summary: &GuiWaveformSummary,
    selected_signal: &mut Option<String>,
) {
    ui.separator();
    ui.label(format!(
        "Waveform: {} points, {} variables",
        summary.point_count, summary.variable_count
    ));
    if !summary.plot_name.is_empty() {
        ui.label(format!("Plot: {}", summary.plot_name));
    }
    if !summary.title.is_empty() {
        ui.label(format!("Title: {}", summary.title));
    }
    ui.monospace(summary.raw_path.display().to_string());

    draw_waveform_preview_selector(ui, summary, selected_signal);
    if let Some(signal) = selected_signal.as_deref()
        && let Some(preview) = summary.preview_for_signal(signal)
    {
        draw_waveform_preview(ui, preview);
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

fn draw_waveform_preview(ui: &mut egui::Ui, preview: &GuiWaveformPreview) {
    let desired_size = Vec2::new(ui.available_width().max(160.0), 120.0);
    let (rect, response) = ui.allocate_exact_size(desired_size, Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 2.0, Color32::from_rgb(248, 250, 252));
    painter.rect_stroke(
        rect,
        2.0,
        Stroke::new(1.0, Color32::from_rgb(190, 198, 208)),
        StrokeKind::Inside,
    );
    let plot_rect = rect.shrink2(Vec2::new(8.0, 8.0));
    draw_waveform_zero_axis(&painter, plot_rect, preview);
    draw_waveform_buckets(&painter, plot_rect, preview);
    response.on_hover_text(format!(
        "{} [{}]\n{} source points\n{} to {}\n{} to {}",
        preview.signal,
        preview.unit,
        preview.source_points,
        format_compact_f64(preview.time_min),
        format_compact_f64(preview.time_max),
        format_compact_f64(preview.value_min),
        format_compact_f64(preview.value_max)
    ));
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

fn draw_waveform_zero_axis(painter: &egui::Painter, rect: Rect, preview: &GuiWaveformPreview) {
    if preview.value_min > 0.0 || preview.value_max < 0.0 {
        return;
    }
    let y = value_to_screen_y(rect, 0.0, preview);
    painter.line_segment(
        [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
        Stroke::new(1.0, Color32::from_rgb(220, 224, 230)),
    );
}

fn draw_waveform_buckets(painter: &egui::Painter, rect: Rect, preview: &GuiWaveformPreview) {
    let stroke = Stroke::new(1.5, Color32::from_rgb(35, 105, 180));
    let mut previous_midpoint = None;
    for bucket in &preview.buckets {
        let x_start = time_to_screen_x(rect, bucket.start_time, preview);
        let x_end = time_to_screen_x(rect, bucket.end_time, preview);
        let x_mid = (x_start + x_end) * 0.5;
        let y_min = value_to_screen_y(rect, bucket.min, preview);
        let y_max = value_to_screen_y(rect, bucket.max, preview);
        let midpoint = Pos2::new(x_mid, (y_min + y_max) * 0.5);
        if bucket.samples > 1 {
            painter.line_segment([Pos2::new(x_mid, y_min), Pos2::new(x_mid, y_max)], stroke);
        }
        if let Some(previous) = previous_midpoint {
            painter.line_segment([previous, midpoint], stroke);
        }
        previous_midpoint = Some(midpoint);
    }
}

fn time_to_screen_x(rect: Rect, time: f64, preview: &GuiWaveformPreview) -> f32 {
    let span = (preview.time_max - preview.time_min).abs();
    if span <= f64::EPSILON {
        return rect.center().x;
    }
    let normalized = ((time - preview.time_min) / span).clamp(0.0, 1.0) as f32;
    rect.left() + normalized * rect.width()
}

fn value_to_screen_y(rect: Rect, value: f64, preview: &GuiWaveformPreview) -> f32 {
    let span = (preview.value_max - preview.value_min).abs();
    if span <= f64::EPSILON {
        return rect.center().y;
    }
    let normalized = ((value - preview.value_min) / span).clamp(0.0, 1.0) as f32;
    rect.bottom() - normalized * rect.height()
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

fn format_compact_f64(value: f64) -> String {
    if !value.is_finite() {
        return value.to_string();
    }
    let absolute = value.abs();
    if value == 0.0 {
        "0".to_string()
    } else if !(1.0e-3..1.0e4).contains(&absolute) {
        format!("{value:.3e}")
    } else {
        format!("{value:.4}")
    }
}
