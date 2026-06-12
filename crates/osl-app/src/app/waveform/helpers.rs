//! Shared helpers for waveform preview rendering.
//!
//! Contains functions used by both static previews and interactive plots
//! to avoid code duplication across modules.

use crate::app::theme::{StudioThemeMode};
use crate::waveform_summary::{GuiWaveformPreview, GuiWaveformSummary};
use eframe::egui::{self, Align2, FontId, Pos2, Rect, Stroke, Vec2};

/// Maximum number of traces shown in stacked preview.
pub(crate) const STACKED_TRACE_LIMIT: usize = 4;

/// Height of the navigator bar at the bottom of stacked previews.
pub(crate) const STACKED_NAVIGATOR_HEIGHT: f32 = 30.0;

/// Select which previews to display, prioritizing the selected signal.
pub(crate) fn ordered_previews<'a>(
    summary: &'a GuiWaveformSummary,
    selected_signal: Option<&str>,
) -> Vec<&'a GuiWaveformPreview> {
    let mut previews = Vec::new();
    if let Some(signal) = selected_signal
        && let Some(selected) = summary.preview_for_signal(signal)
    {
        previews.push(selected);
    }
    for preview in &summary.previews {
        if previews
            .iter()
            .all(|candidate| !same_signal(&candidate.signal, &preview.signal))
        {
            previews.push(preview);
        }
        if previews.len() == STACKED_TRACE_LIMIT {
            break;
        }
    }
    previews
}

/// Draw a trace label in the top-left corner of a lane.
pub(crate) fn draw_trace_label(
    painter: &egui::Painter,
    rect: Rect,
    preview: &GuiWaveformPreview,
    color: egui::Color32,
    muted: egui::Color32,
) {
    painter.text(
        Pos2::new(rect.left() + 4.0, rect.top() + 2.0),
        Align2::LEFT_TOP,
        &preview.signal,
        FontId::monospace(11.0),
        color,
    );
    let unit = if preview.unit.is_empty() {
        format!("{} samples", preview.source_points)
    } else {
        format!("{} / {}", preview.unit, preview.source_points)
    };
    painter.text(
        Pos2::new(rect.right() - 4.0, rect.top() + 2.0),
        Align2::RIGHT_TOP,
        unit,
        FontId::monospace(10.0),
        muted,
    );
}

/// Draw time-axis labels below the plot.
pub(crate) fn draw_axis_labels(
    painter: &egui::Painter,
    rect: Rect,
    preview: &GuiWaveformPreview,
    muted: egui::Color32,
) {
    use super::preview::format_compact_f64;
    painter.text(
        Pos2::new(rect.left(), rect.bottom() + 4.0),
        Align2::LEFT_TOP,
        format_compact_f64(preview.time_min),
        FontId::monospace(10.0),
        muted,
    );
    painter.text(
        Pos2::new(rect.center().x, rect.bottom() + 4.0),
        Align2::CENTER_TOP,
        "time",
        FontId::monospace(10.0),
        muted,
    );
    painter.text(
        Pos2::new(rect.right(), rect.bottom() + 4.0),
        Align2::RIGHT_TOP,
        format_compact_f64(preview.time_max),
        FontId::monospace(10.0),
        muted,
    );
}

/// Draw the navigator overview bar at the bottom of stacked previews.
pub(crate) fn draw_navigator(
    painter: &egui::Painter,
    mode: StudioThemeMode,
    rect: Rect,
    preview: &GuiWaveformPreview,
) {
    use super::preview_primitives::draw_waveform_buckets;
    use eframe::egui::StrokeKind;
    let palette = crate::app::theme::StudioTheme::palette(mode);
    painter.rect_filled(rect, 2.0, palette.panel);
    painter.rect_stroke(
        rect,
        2.0,
        Stroke::new(1.0, palette.border),
        StrokeKind::Inside,
    );
    let trace_rect = rect.shrink2(Vec2::new(6.0, 5.0));
    draw_waveform_buckets(
        painter,
        trace_rect,
        preview,
        palette.accent.linear_multiply(0.75),
        1.0,
    );
    let window = Rect::from_min_max(
        Pos2::new(
            trace_rect.left() + trace_rect.width() * 0.30,
            trace_rect.top() - 2.0,
        ),
        Pos2::new(
            trace_rect.left() + trace_rect.width() * 0.72,
            trace_rect.bottom() + 2.0,
        ),
    );
    painter.rect_stroke(
        window,
        1.0,
        Stroke::new(1.0, palette.accent),
        StrokeKind::Inside,
    );
}

/// Case-insensitive signal name comparison.
pub(crate) fn same_signal(left: &str, right: &str) -> bool {
    left.trim().eq_ignore_ascii_case(right.trim())
}
