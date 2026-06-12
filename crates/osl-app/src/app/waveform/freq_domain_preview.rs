//! Frequency-domain waveform preview — renders FFT magnitude/phase and Bode
//! plots using the pure-Rust FFT engine from `osl_waveform::fft`.
//!
//! The rendering uses the same bucket-based drawing approach as time-domain
//! previews but maps frequency on the x-axis (log scale for Bode plots).

use crate::app::theme::{StudioTheme, StudioThemeMode};
use crate::waveform_summary::{GuiWaveformPreview, GuiWaveformSummary};
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Stroke, StrokeKind, Vec2};
use osl_waveform::fft::{self, FftBin, WindowFunction};

const BODE_TRACE_HEIGHT: f32 = 140.0;

/// Draw an FFT magnitude plot from time-domain waveform data.
///
/// Computes the FFT of the selected signal and renders magnitude (dB) vs. frequency
/// on a log-scale x-axis. If no signal is selected, shows all available signals.
pub(crate) fn draw_fft_magnitude_plot(
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
    summary: &GuiWaveformSummary,
    selected_signal: Option<&str>,
) {
    let desired_size = Vec2::new(ui.available_width().max(320.0), 300.0);
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::hover());
    let palette = StudioTheme::palette(mode);
    let painter = ui.painter_at(rect);

    // Background
    painter.rect_filled(rect, 4.0, plot_fill(mode));
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0, palette.border), StrokeKind::Inside);

    let plot_rect = rect.shrink2(Vec2::new(40.0, 20.0));

    // Get the selected signal or first available
    let signal_name = selected_signal
        .or_else(|| summary.previews.first().map(|p| p.signal.as_str()));

    let Some(signal) = signal_name else {
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            "No signals available for FFT",
            FontId::proportional(13.0),
            palette.text_muted,
        );
        return;
    };

    // Compute dt from the preview data (time range / source points)
    let preview = match summary.preview_for_signal(signal) {
        Some(p) => p,
        None => return,
    };
    let time_range = preview.time_max - preview.time_min;
    if time_range <= 0.0 || preview.source_points < 2 {
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            "Insufficient data for FFT",
            FontId::proportional(13.0),
            palette.text_muted,
        );
        return;
    }
    let dt = time_range / (preview.source_points - 1) as f64;

    // Reconstruct approximate time-domain values from buckets for FFT
    let samples = reconstruct_samples_from_buckets(preview);

    if samples.len() < 4 {
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            "Not enough samples for FFT",
            FontId::proportional(13.0),
            palette.text_muted,
        );
        return;
    }

    // Compute FFT
    let bins = fft::compute_fft_bins(&samples, dt, WindowFunction::Hanning);
    if bins.is_empty() {
        return;
    }

    // Draw log-frequency grid and magnitude trace
    draw_freq_grid(&painter, plot_rect, &bins, palette.border.linear_multiply(0.55));
    draw_magnitude_trace(&painter, plot_rect, &bins, palette.accent, 1.5);
    draw_freq_axis_labels(&painter, plot_rect, &bins, palette.text_muted);

    // Y-axis label
    painter.text(
        Pos2::new(plot_rect.left() - 6.0, plot_rect.center().y),
        Align2::RIGHT_CENTER,
        "dB",
        FontId::monospace(10.0),
        palette.text_muted,
    );

    // Title
    painter.text(
        Pos2::new(plot_rect.left(), plot_rect.top() - 8.0),
        Align2::LEFT_BOTTOM,
        format!("FFT: {signal}"),
        FontId::proportional(12.0),
        palette.text,
    );

    // Hover tooltip
    response.on_hover_text(format!(
        "FFT: {} bins, {} Hz to {:.3e} Hz\nSignal: {}",
        bins.len(),
        bins.first().map(|b| b.frequency).unwrap_or(0.0),
        bins.last().map(|b| b.frequency).unwrap_or(0.0),
        signal,
    ));
}

/// Draw a dual-panel Bode plot (magnitude + phase) from time-domain data.
pub(crate) fn draw_bode_plot(
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
    summary: &GuiWaveformSummary,
    selected_signal: Option<&str>,
) {
    let palette = StudioTheme::palette(mode);
    let signal_name = selected_signal
        .or_else(|| summary.previews.first().map(|p| p.signal.as_str()));

    let Some(signal) = signal_name else {
        let desired_size = Vec2::new(ui.available_width().max(320.0), BODE_TRACE_HEIGHT * 2.0 + 10.0);
        let (rect, _) = ui.allocate_exact_size(desired_size, egui::Sense::hover());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 4.0, plot_fill(mode));
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            "No signals available for Bode plot",
            FontId::proportional(13.0),
            palette.text_muted,
        );
        return;
    };

    let preview = match summary.preview_for_signal(signal) {
        Some(p) => p,
        None => return,
    };
    let time_range = preview.time_max - preview.time_min;
    if time_range <= 0.0 || preview.source_points < 2 {
        return;
    }
    let dt = time_range / (preview.source_points - 1) as f64;
    let samples = reconstruct_samples_from_buckets(preview);

    if samples.len() < 4 {
        return;
    }

    let bins = fft::compute_fft_bins(&samples, dt, WindowFunction::Hanning);
    if bins.is_empty() {
        return;
    }

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
    mag_painter.text(
        Pos2::new(mag_plot.left(), mag_plot.top() - 4.0),
        Align2::LEFT_BOTTOM,
        "Magnitude (dB)",
        FontId::monospace(10.0),
        palette.text_muted,
    );

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
    phase_painter.text(
        Pos2::new(phase_plot.left(), phase_plot.top() - 4.0),
        Align2::LEFT_BOTTOM,
        "Phase (deg)",
        FontId::monospace(10.0),
        palette.text_muted,
    );

    // Y-axis labels
    mag_painter.text(
        Pos2::new(mag_plot.left() - 6.0, mag_plot.center().y),
        Align2::RIGHT_CENTER,
        "dB",
        FontId::monospace(9.0),
        palette.text_muted,
    );
    phase_painter.text(
        Pos2::new(phase_plot.left() - 6.0, phase_plot.center().y),
        Align2::RIGHT_CENTER,
        "deg",
        FontId::monospace(9.0),
        palette.text_muted,
    );
}

/// Draw a noise spectral density plot.
pub(crate) fn draw_noise_plot(
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
    summary: &GuiWaveformSummary,
    selected_signal: Option<&str>,
) {
    // Noise plot is similar to FFT but displays noise spectral density (V²/Hz)
    let desired_size = Vec2::new(ui.available_width().max(320.0), 300.0);
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::hover());
    let palette = StudioTheme::palette(mode);
    let painter = ui.painter_at(rect);

    painter.rect_filled(rect, 4.0, plot_fill(mode));
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0, palette.border), StrokeKind::Inside);

    let plot_rect = rect.shrink2(Vec2::new(40.0, 20.0));

    let signal_name = selected_signal
        .or_else(|| summary.previews.first().map(|p| p.signal.as_str()));

    let Some(signal) = signal_name else {
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            "No data for noise analysis",
            FontId::proportional(13.0),
            palette.text_muted,
        );
        return;
    };

    let preview = match summary.preview_for_signal(signal) {
        Some(p) => p,
        None => return,
    };
    let time_range = preview.time_max - preview.time_min;
    if time_range <= 0.0 || preview.source_points < 2 {
        return;
    }
    let dt = time_range / (preview.source_points - 1) as f64;
    let samples = reconstruct_samples_from_buckets(preview);
    if samples.len() < 4 {
        return;
    }

    let bins = fft::compute_fft_bins(&samples, dt, WindowFunction::Hanning);
    if bins.is_empty() {
        return;
    }

    // Noise spectral density: |FFT|² / (fs * N)
    let fs = 1.0 / dt;
    let n = samples.len() as f64;
    let noise_density: Vec<f64> = bins.iter().map(|b| {
        let psd = b.magnitude * b.magnitude / (fs * n);
        10.0 * psd.log10().max(-300.0)
    }).collect();

    let max_nd = noise_density.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let min_nd = noise_density.iter().cloned().filter(|v| *v > -300.0).fold(f64::INFINITY, f64::min);
    let nd_range = (max_nd - min_nd).max(1.0);

    // Draw grid
    draw_freq_grid(&painter, plot_rect, &bins, palette.border.linear_multiply(0.5));

    // Draw noise density trace
    let mut prev_point: Option<Pos2> = None;
    for (i, &nd) in noise_density.iter().enumerate() {
        if nd < -299.0 {
            prev_point = None;
            continue;
        }
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
    painter.text(
        Pos2::new(plot_rect.left() - 6.0, plot_rect.center().y),
        Align2::RIGHT_CENTER,
        "dBV/Hz",
        FontId::monospace(9.0),
        palette.text_muted,
    );
    painter.text(
        Pos2::new(plot_rect.left(), plot_rect.top() - 8.0),
        Align2::LEFT_BOTTOM,
        format!("Noise: {signal}"),
        FontId::proportional(12.0),
        palette.text,
    );

    response.on_hover_text("Noise spectral density plot");
}

// ── Internal helpers ────────────────────────────────────────────────────

/// Reconstruct approximate time-domain samples from bucket data.
/// Each bucket contributes `samples` evenly-spaced points.
fn reconstruct_samples_from_buckets(preview: &GuiWaveformPreview) -> Vec<f64> {
    let mut result = Vec::new();
    for bucket in &preview.buckets {
        if bucket.samples == 0 {
            continue;
        }
        let mid = (bucket.min + bucket.max) * 0.5;
        for _ in 0..bucket.samples {
            result.push(mid);
        }
    }
    result
}

/// Draw frequency grid lines (vertical).
fn draw_freq_grid(painter: &egui::Painter, rect: Rect, bins: &[FftBin], color: Color32) {
    if bins.is_empty() {
        return;
    }
    // Horizontal grid lines (magnitude dB levels)
    for i in 1..5 {
        let y = rect.top() + rect.height() * i as f32 / 5.0;
        painter.line_segment(
            [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
            Stroke::new(0.5, color),
        );
    }
    // Vertical grid lines at decade boundaries
    if let Some(max_freq) = bins.last().map(|b| b.frequency) {
        let mut decade = 1.0;
        while decade < max_freq {
            let x = log_freq_to_screen_x(rect, decade, bins);
            if x >= rect.left() && x <= rect.right() {
                painter.line_segment(
                    [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
                    Stroke::new(0.5, color),
                );
            }
            decade *= 10.0;
        }
    }
}

/// Draw the magnitude trace as a connected line.
fn draw_magnitude_trace(painter: &egui::Painter, rect: Rect, bins: &[FftBin], color: Color32, width: f32) {
    if bins.len() < 2 {
        return;
    }
    let max_db = bins.iter().map(|b| b.magnitude_db).fold(f64::NEG_INFINITY, f64::max);
    let min_db = bins
        .iter()
        .map(|b| b.magnitude_db)
        .filter(|v| *v > -299.0)
        .fold(f64::INFINITY, f64::min);
    let db_range = (max_db - min_db).max(1.0);

    let stroke = Stroke::new(width, color);
    let mut prev: Option<Pos2> = None;
    for bin in bins {
        if bin.magnitude_db < -299.0 {
            prev = None;
            continue;
        }
        let x = log_freq_to_screen_x(rect, bin.frequency, bins);
        let y_norm = ((bin.magnitude_db - min_db) / db_range).clamp(0.0, 1.0) as f32;
        let y = rect.bottom() - y_norm * rect.height();
        let point = Pos2::new(x, y);
        if let Some(p) = prev {
            painter.line_segment([p, point], stroke);
        }
        prev = Some(point);
    }
}

/// Draw the phase trace.
fn draw_phase_trace(painter: &egui::Painter, rect: Rect, bins: &[FftBin], color: Color32, width: f32) {
    if bins.len() < 2 {
        return;
    }
    let stroke = Stroke::new(width, color);
    let mut prev: Option<Pos2> = None;
    for bin in bins {
        let x = log_freq_to_screen_x(rect, bin.frequency, bins);
        // Phase normalized to [-180, 180] -> [0, 1]
        let y_norm = ((bin.phase_deg + 180.0) / 360.0).clamp(0.0, 1.0) as f32;
        let y = rect.bottom() - y_norm * rect.height();
        let point = Pos2::new(x, y);
        if let Some(p) = prev {
            painter.line_segment([p, point], stroke);
        }
        prev = Some(point);
    }
}

/// Map a frequency to screen x-coordinate using log scale.
fn log_freq_to_screen_x(rect: Rect, freq: f64, bins: &[FftBin]) -> f32 {
    let min_freq = bins.first().map(|b| b.frequency.max(1e-10)).unwrap_or(1e-10);
    let max_freq = bins.last().map(|b| b.frequency.max(1.0)).unwrap_or(1.0);
    if min_freq <= 0.0 || max_freq <= min_freq || freq <= 0.0 {
        return rect.left();
    }
    let log_min = min_freq.log10();
    let log_max = max_freq.log10();
    let log_range = log_max - log_min;
    if log_range <= 0.0 {
        return rect.left();
    }
    let normalized = ((freq.log10() - log_min) / log_range).clamp(0.0, 1.0) as f32;
    rect.left() + normalized * rect.width()
}

/// Draw frequency axis labels at decade boundaries.
fn draw_freq_axis_labels(painter: &egui::Painter, rect: Rect, bins: &[FftBin], color: Color32) {
    if bins.is_empty() {
        return;
    }
    let max_freq = bins.last().map(|b| b.frequency).unwrap_or(1.0);

    // Show start and end frequencies
    if let Some(min_freq) = bins.first().map(|b| b.frequency) {
        if min_freq > 0.0 {
            painter.text(
                Pos2::new(rect.left(), rect.bottom() + 4.0),
                Align2::LEFT_TOP,
                format_compact_freq(min_freq),
                FontId::monospace(9.0),
                color,
            );
        }
    }
    painter.text(
        Pos2::new(rect.right(), rect.bottom() + 4.0),
        Align2::RIGHT_TOP,
        format_compact_freq(max_freq),
        FontId::monospace(9.0),
        color,
    );
    // Center label
    let _center_freq = (bins.first().map(|b| b.frequency).unwrap_or(1.0)
        * max_freq).sqrt();
    painter.text(
        Pos2::new(rect.center().x, rect.bottom() + 4.0),
        Align2::CENTER_TOP,
        "frequency (Hz)",
        FontId::monospace(9.0),
        color,
    );
}

/// Format frequency in compact form.
fn format_compact_freq(freq: f64) -> String {
    if freq >= 1e9 {
        format!("{:.1} GHz", freq / 1e9)
    } else if freq >= 1e6 {
        format!("{:.1} MHz", freq / 1e6)
    } else if freq >= 1e3 {
        format!("{:.1} kHz", freq / 1e3)
    } else if freq >= 1.0 {
        format!("{:.0} Hz", freq)
    } else {
        format!("{:.2} Hz", freq)
    }
}

/// Plot background fill color matching the theme.
fn plot_fill(mode: StudioThemeMode) -> Color32 {
    let palette = StudioTheme::palette(mode);
    if matches!(mode, StudioThemeMode::Light) {
        palette.canvas
    } else {
        palette.panel_soft
    }
}
