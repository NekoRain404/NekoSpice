use crate::app::theme::{StudioTheme, StudioThemeMode};
use crate::waveform_summary::GuiWaveformPreview;
use eframe::egui::{self, Color32, Pos2, Rect, Stroke};

/// draw plot grid。
pub(crate) fn draw_plot_grid(painter: &egui::Painter, rect: Rect, color: Color32) {
    for index in 1..5 {
        let x = rect.left() + rect.width() * index as f32 / 5.0;
        painter.line_segment(
            [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
            Stroke::new(0.7, color),
        );
    }
    for index in 1..4 {
        let y = rect.top() + rect.height() * index as f32 / 4.0;
        painter.line_segment(
            [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
            Stroke::new(0.7, color),
        );
    }
}

/// draw waveform zero axis。
pub(crate) fn draw_waveform_zero_axis(
    painter: &egui::Painter,
    rect: Rect,
    preview: &GuiWaveformPreview,
    color: Color32,
) {
    if preview.value_min > 0.0 || preview.value_max < 0.0 {
        return;
    }
    let y = value_to_screen_y(rect, 0.0, preview);
    painter.line_segment(
        [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
        Stroke::new(0.9, color),
    );
}

/// draw waveform buckets。
pub(crate) fn draw_waveform_buckets(
    painter: &egui::Painter,
    rect: Rect,
    preview: &GuiWaveformPreview,
    color: Color32,
    width: f32,
) {
    let stroke = Stroke::new(width, color);
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

/// trace color。
pub(crate) fn trace_color(mode: StudioThemeMode, index: usize) -> Color32 {
    let palette = StudioTheme::palette(mode);
    match index {
        0 => palette.success,
        1 => palette.warning,
        2 => Color32::from_rgb(35, 206, 229),
        3 => Color32::from_rgb(211, 71, 225),
        _ => palette.accent,
    }
}

/// plot fill。
pub(crate) fn plot_fill(mode: StudioThemeMode) -> Color32 {
    let palette = StudioTheme::palette(mode);
    if matches!(mode, StudioThemeMode::Light) {
        palette.canvas
    } else {
        palette.panel_soft
    }
}

/// format compact f64。
pub(crate) fn format_compact_f64(value: f64) -> String {
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
