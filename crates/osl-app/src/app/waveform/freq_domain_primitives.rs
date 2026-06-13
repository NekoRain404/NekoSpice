//! Frequency-domain drawing primitives.
//!
//! Shared rendering functions for FFT/Bode/Noise plots:
//! grid lines, magnitude/phase traces, log-frequency axis mapping,
//! and compact frequency formatting.

use osl_waveform::fft::FftBin;
use eframe::egui::{self, Color32, FontId, Pos2, Rect, Stroke};

/// Draw frequency grid lines (horizontal + vertical decade markers).
pub(crate) fn draw_freq_grid(painter: &egui::Painter, rect: Rect, bins: &[FftBin], color: Color32) {
    // Horizontal grid lines
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
pub(crate) fn draw_magnitude_trace(painter: &egui::Painter, rect: Rect, bins: &[FftBin], color: Color32, width: f32) {
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

/// Draw the phase trace as a connected line.
pub(crate) fn draw_phase_trace(painter: &egui::Painter, rect: Rect, bins: &[FftBin], color: Color32, width: f32) {
    if bins.len() < 2 {
        return;
    }
    let stroke = Stroke::new(width, color);
    let mut prev: Option<Pos2> = None;
    for bin in bins {
        let x = log_freq_to_screen_x(rect, bin.frequency, bins);
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
pub(crate) fn log_freq_to_screen_x(rect: Rect, freq: f64, bins: &[FftBin]) -> f32 {
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

/// Draw frequency axis labels at the edges and center.
pub(crate) fn draw_freq_axis_labels(painter: &egui::Painter, rect: Rect, bins: &[FftBin], color: Color32) {
    if bins.is_empty() {
        return;
    }
    if let Some(min_freq) = bins.first().map(|b| b.frequency) {
        if min_freq > 0.0 {
            painter.text(
                Pos2::new(rect.left(), rect.bottom() + 4.0),
                eframe::egui::Align2::LEFT_TOP,
                format_compact_freq(min_freq),
                FontId::monospace(9.0),
                color,
            );
        }
    }
    if let Some(max_freq) = bins.last().map(|b| b.frequency) {
        painter.text(
            Pos2::new(rect.right(), rect.bottom() + 4.0),
            eframe::egui::Align2::RIGHT_TOP,
            format_compact_freq(max_freq),
            FontId::monospace(9.0),
            color,
        );
    }
    painter.text(
        Pos2::new(rect.center().x, rect.bottom() + 4.0),
        eframe::egui::Align2::CENTER_TOP,
        "frequency (Hz)",
        FontId::monospace(9.0),
        color,
    );
}

/// Format frequency in compact human-readable form.
pub(crate) fn format_compact_freq(freq: f64) -> String {
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

/// Reconstruct approximate time-domain samples from bucket data.
pub(crate) fn reconstruct_samples_from_buckets(preview: &crate::waveform_summary::GuiWaveformPreview) -> Vec<f64> {
    preview.buckets.iter()
        .flat_map(|b| std::iter::repeat((b.min + b.max) * 0.5).take(b.samples))
        .collect()
}
