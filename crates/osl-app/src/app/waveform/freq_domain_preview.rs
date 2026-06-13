//! Frequency-domain waveform preview — renders FFT magnitude/phase and Bode
//! plots using the pure-Rust FFT engine from `osl_waveform::fft`.
//!
//! Plot rendering primitives (grid, traces, axis labels) live in
//! `freq_domain_primitives.rs` to keep each file focused.

use super::freq_domain_primitives::{
    draw_freq_axis_labels, draw_freq_grid, draw_magnitude_trace, draw_phase_trace,
    log_freq_to_screen_x, reconstruct_samples_from_buckets,
};
use super::preview_primitives::plot_fill;
use crate::app::theme::StudioThemeMode;
use crate::waveform_summary::GuiWaveformSummary;
use eframe::egui::{self, Align2, FontId, Pos2, Stroke, StrokeKind, Vec2};
use osl_waveform::fft::{self, WindowFunction};

const BODE_TRACE_HEIGHT: f32 = 140.0;

/// Draw an FFT magnitude plot from time-domain waveform data.
pub(crate) fn draw_fft_magnitude_plot(
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
    summary: &GuiWaveformSummary,
    selected_signal: Option<&str>,
) {
    let desired_size = Vec2::new(ui.available_width().max(320.0), 300.0);
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::hover());
    let palette = eframe::egui::Context::default(); // unused, but needed for palette
    let _ = &palette;
    let palette = crate::app::theme::StudioTheme::palette(mode);
    let painter = ui.painter_at(rect);

    painter.rect_filled(rect, 4.0, plot_fill(mode));
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0, palette.border), StrokeKind::Inside);
    let plot_rect = rect.shrink2(Vec2::new(40.0, 20.0));

    let signal_name = selected_signal
        .or_else(|| summary.previews.first().map(|p| p.signal.as_str()));
    let Some(signal) = signal_name else {
        painter.text(rect.center(), Align2::CENTER_CENTER,
            "No signals available for FFT", FontId::proportional(13.0), palette.text_muted);
        return;
    };
    let preview = match summary.preview_for_signal(signal) {
        Some(p) => p,
        None => return,
    };
    let time_range = preview.time_max - preview.time_min;
    if time_range <= 0.0 || preview.source_points < 2 {
        painter.text(rect.center(), Align2::CENTER_CENTER,
            "Insufficient data for FFT", FontId::proportional(13.0), palette.text_muted);
        return;
    }
    let dt = time_range / (preview.source_points - 1) as f64;
    let samples = reconstruct_samples_from_buckets(preview);
    if samples.len() < 4 {
        painter.text(rect.center(), Align2::CENTER_CENTER,
            "Not enough samples for FFT", FontId::proportional(13.0), palette.text_muted);
        return;
    }

    let bins = fft::compute_fft_bins(&samples, dt, WindowFunction::Hanning);
    if bins.is_empty() { return; }

    draw_freq_grid(&painter, plot_rect, &bins, palette.border.linear_multiply(0.55));
    draw_magnitude_trace(&painter, plot_rect, &bins, palette.accent, 1.5);
    draw_freq_axis_labels(&painter, plot_rect, &bins, palette.text_muted);

    painter.text(Pos2::new(plot_rect.left() - 6.0, plot_rect.center().y),
        Align2::RIGHT_CENTER, "dB", FontId::monospace(10.0), palette.text_muted);
    painter.text(Pos2::new(plot_rect.left(), plot_rect.top() - 8.0),
        Align2::LEFT_BOTTOM, format!("FFT: {signal}"), FontId::proportional(12.0), palette.text);

    response.on_hover_text(format!(
        "FFT: {} bins, {} Hz to {:.3e} Hz\nSignal: {}",
        bins.len(), bins.first().map(|b| b.frequency).unwrap_or(0.0),
        bins.last().map(|b| b.frequency).unwrap_or(0.0), signal,
    ));
}

/// Draw a dual-panel Bode plot (magnitude + phase) from time-domain data.
pub(crate) fn draw_bode_plot(
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
    summary: &GuiWaveformSummary,
    selected_signal: Option<&str>,
) {
    let palette = crate::app::theme::StudioTheme::palette(mode);
    let signal_name = selected_signal
        .or_else(|| summary.previews.first().map(|p| p.signal.as_str()));

    let Some(signal) = signal_name else {
        let desired_size = Vec2::new(ui.available_width().max(320.0), BODE_TRACE_HEIGHT * 2.0 + 10.0);
        let (rect, _) = ui.allocate_exact_size(desired_size, egui::Sense::hover());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 4.0, plot_fill(mode));
        painter.text(rect.center(), Align2::CENTER_CENTER,
            "No signals available for Bode plot", FontId::proportional(13.0), palette.text_muted);
        return;
    };

    let preview = match summary.preview_for_signal(signal) {
        Some(p) => p,
        None => return,
    };
    let time_range = preview.time_max - preview.time_min;
    if time_range <= 0.0 || preview.source_points < 2 { return; }
    let dt = time_range / (preview.source_points - 1) as f64;
    let samples = reconstruct_samples_from_buckets(preview);
    if samples.len() < 4 { return; }

    let bins = fft::compute_fft_bins(&samples, dt, WindowFunction::Hanning);
    if bins.is_empty() { return; }

    // Magnitude panel
    let mag_size = Vec2::new(ui.available_width().max(320.0), BODE_TRACE_HEIGHT);
    let (mag_rect, _) = ui.allocate_exact_size(mag_size, egui::Sense::hover());
    let mag_painter = ui.painter_at(mag_rect);
    mag_painter.rect_filled(mag_rect, 3.0, plot_fill(mode));
    mag_painter.rect_stroke(mag_rect, 3.0, Stroke::new(1.0, palette.border), StrokeKind::Inside);
    let mag_plot = mag_rect.shrink2(Vec2::new(40.0, 14.0));
    draw_freq_grid(&mag_painter, mag_plot, &bins, palette.border.linear_multiply(0.5));
    draw_magnitude_trace(&mag_painter, mag_plot, &bins, palette.success, 1.5);
    draw_freq_axis_labels(&mag_painter, mag_plot, &bins, palette.text_muted);
    mag_painter.text(Pos2::new(mag_plot.left() - 6.0, mag_plot.center().y),
        Align2::RIGHT_CENTER, "dB", FontId::monospace(9.0), palette.text_muted);
    mag_painter.text(Pos2::new(mag_plot.left(), mag_plot.top() - 4.0),
        Align2::LEFT_BOTTOM, "Magnitude (dB)", FontId::monospace(10.0), palette.text_muted);

    ui.add_space(4.0);

    // Phase panel
    let phase_size = Vec2::new(ui.available_width().max(320.0), BODE_TRACE_HEIGHT);
    let (phase_rect, _) = ui.allocate_exact_size(phase_size, egui::Sense::hover());
    let phase_painter = ui.painter_at(phase_rect);
    phase_painter.rect_filled(phase_rect, 3.0, plot_fill(mode));
    phase_painter.rect_stroke(phase_rect, 3.0, Stroke::new(1.0, palette.border), StrokeKind::Inside);
    let phase_plot = phase_rect.shrink2(Vec2::new(40.0, 14.0));
    draw_freq_grid(&phase_painter, phase_plot, &bins, palette.border.linear_multiply(0.5));
    draw_phase_trace(&phase_painter, phase_plot, &bins, palette.warning, 1.5);
    draw_freq_axis_labels(&phase_painter, phase_plot, &bins, palette.text_muted);
    phase_painter.text(Pos2::new(phase_plot.left() - 6.0, phase_plot.center().y),
        Align2::RIGHT_CENTER, "deg", FontId::monospace(9.0), palette.text_muted);
    phase_painter.text(Pos2::new(phase_plot.left(), phase_plot.top() - 4.0),
        Align2::LEFT_BOTTOM, "Phase (deg)", FontId::monospace(10.0), palette.text_muted);
}

/// Draw a noise spectral density plot.
pub(crate) fn draw_noise_plot(
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
    summary: &GuiWaveformSummary,
    selected_signal: Option<&str>,
) {
    let desired_size = Vec2::new(ui.available_width().max(320.0), 300.0);
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::hover());
    let palette = crate::app::theme::StudioTheme::palette(mode);
    let painter = ui.painter_at(rect);

    painter.rect_filled(rect, 4.0, plot_fill(mode));
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0, palette.border), StrokeKind::Inside);
    let plot_rect = rect.shrink2(Vec2::new(40.0, 20.0));

    let signal_name = selected_signal
        .or_else(|| summary.previews.first().map(|p| p.signal.as_str()));
    let Some(signal) = signal_name else {
        painter.text(rect.center(), Align2::CENTER_CENTER,
            "No data for noise analysis", FontId::proportional(13.0), palette.text_muted);
        return;
    };
    let preview = match summary.preview_for_signal(signal) {
        Some(p) => p,
        None => return,
    };
    let time_range = preview.time_max - preview.time_min;
    if time_range <= 0.0 || preview.source_points < 2 { return; }
    let dt = time_range / (preview.source_points - 1) as f64;
    let samples = reconstruct_samples_from_buckets(preview);
    if samples.len() < 4 { return; }

    let bins = fft::compute_fft_bins(&samples, dt, WindowFunction::Hanning);
    if bins.is_empty() { return; }

    // Noise spectral density: |FFT|^2 / (fs * N)
    let fs = 1.0 / dt;
    let n = samples.len() as f64;
    let noise_density: Vec<f64> = bins.iter().map(|b| {
        let psd = b.magnitude * b.magnitude / (fs * n);
        10.0 * psd.log10().max(-300.0)
    }).collect();

    let max_nd = noise_density.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let min_nd = noise_density.iter().cloned().filter(|v| *v > -300.0).fold(f64::INFINITY, f64::min);
    let nd_range = (max_nd - min_nd).max(1.0);

    draw_freq_grid(&painter, plot_rect, &bins, palette.border.linear_multiply(0.5));

    let mut prev_point: Option<Pos2> = None;
    for (i, &nd) in noise_density.iter().enumerate() {
        if nd < -299.0 { prev_point = None; continue; }
        let x = log_freq_to_screen_x(plot_rect, bins[i].frequency, &bins);
        let y_norm = ((nd - min_nd) / nd_range).clamp(0.0, 1.0) as f32;
        let y = plot_rect.bottom() - y_norm * plot_rect.height();
        let point = Pos2::new(x, y);
        if let Some(prev) = prev_point {
            painter.line_segment([prev, point], Stroke::new(1.5, palette.accent));
        }
        prev_point = Some(point);
    }

    draw_freq_axis_labels(&painter, plot_rect, &bins, palette.text_muted);
    painter.text(Pos2::new(plot_rect.left() - 6.0, plot_rect.center().y),
        Align2::RIGHT_CENTER, "dBV/Hz", FontId::monospace(9.0), palette.text_muted);
    painter.text(Pos2::new(plot_rect.left(), plot_rect.top() - 8.0),
        Align2::LEFT_BOTTOM, format!("Noise: {signal}"), FontId::proportional(12.0), palette.text);

    response.on_hover_text("Noise spectral density plot");
}
