use crate::app::theme::{StudioTheme, StudioThemeMode};
pub(crate) use super::preview_primitives::format_compact_f64;
use super::preview_primitives::{
    draw_plot_grid, draw_waveform_buckets, draw_waveform_zero_axis, plot_fill, trace_color,
};
use crate::waveform_summary::{GuiWaveformPreview, GuiWaveformSummary};
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};

const STACKED_TRACE_LIMIT: usize = 4;
const STACKED_NAVIGATOR_HEIGHT: f32 = 30.0;

/// draw single waveform preview。
pub(crate) fn draw_single_waveform_preview(
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
    preview: &GuiWaveformPreview,
    height: f32,
) {
    let desired_size = Vec2::new(ui.available_width().max(160.0), height);
    let (rect, response) = ui.allocate_exact_size(desired_size, Sense::hover());
    let palette = StudioTheme::palette(mode);
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 3.0, plot_fill(mode));
    painter.rect_stroke(
        rect,
        3.0,
        Stroke::new(1.0, palette.border),
        StrokeKind::Inside,
    );

    let plot_rect = rect.shrink2(Vec2::new(9.0, 9.0));
    draw_plot_grid(&painter, plot_rect, palette.border.linear_multiply(0.55));
    draw_waveform_zero_axis(&painter, plot_rect, preview, palette.border);
    draw_waveform_buckets(&painter, plot_rect, preview, palette.accent, 1.5);

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

/// draw stacked waveform preview。
pub(crate) fn draw_stacked_waveform_preview(
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
    summary: &GuiWaveformSummary,
    selected_signal: Option<&str>,
    height: f32,
) {
    let desired_size = Vec2::new(ui.available_width().max(320.0), height);
    let (rect, response) = ui.allocate_exact_size(desired_size, Sense::hover());
    let palette = StudioTheme::palette(mode);
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, plot_fill(mode));
    painter.rect_stroke(
        rect,
        4.0,
        Stroke::new(1.0, palette.border),
        StrokeKind::Inside,
    );

    let Some(first_preview) = summary.previews.first() else {
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            "No plottable signals",
            FontId::proportional(13.0),
            palette.text_muted,
        );
        return;
    };

    let navigator_rect = Rect::from_min_max(
        Pos2::new(
            rect.left() + 12.0,
            rect.bottom() - STACKED_NAVIGATOR_HEIGHT - 10.0,
        ),
        Pos2::new(rect.right() - 12.0, rect.bottom() - 10.0),
    );
    let plot_rect = Rect::from_min_max(
        Pos2::new(rect.left() + 14.0, rect.top() + 12.0),
        Pos2::new(rect.right() - 14.0, navigator_rect.top() - 12.0),
    );
    draw_plot_grid(&painter, plot_rect, palette.border.linear_multiply(0.45));

    let traces = ordered_previews(summary, selected_signal);
    let lane_height = plot_rect.height() / traces.len().max(1) as f32;
    for (index, preview) in traces.iter().enumerate() {
        let lane_rect = Rect::from_min_max(
            Pos2::new(
                plot_rect.left(),
                plot_rect.top() + lane_height * index as f32,
            ),
            Pos2::new(
                plot_rect.right(),
                plot_rect.top() + lane_height * (index + 1) as f32,
            ),
        )
        .shrink2(Vec2::new(2.0, 6.0));
        let color = trace_color(mode, index);
        draw_waveform_zero_axis(&painter, lane_rect, preview, palette.border);
        draw_waveform_buckets(&painter, lane_rect, preview, color, 1.35);
        draw_trace_label(&painter, lane_rect, preview, color, palette.text_muted);
    }

    draw_axis_labels(&painter, plot_rect, first_preview, palette.text_muted);
    draw_navigator(&painter, mode, navigator_rect, first_preview);
    response.on_hover_text(format!(
        "{} points, {} variables\n{}",
        summary.point_count,
        summary.variable_count,
        summary.raw_path.display()
    ));
}

fn ordered_previews<'a>(
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

fn draw_trace_label(
    painter: &egui::Painter,
    rect: Rect,
    preview: &GuiWaveformPreview,
    color: Color32,
    muted: Color32,
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

fn draw_axis_labels(
    painter: &egui::Painter,
    rect: Rect,
    preview: &GuiWaveformPreview,
    muted: Color32,
) {
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

fn draw_navigator(
    painter: &egui::Painter,
    mode: StudioThemeMode,
    rect: Rect,
    preview: &GuiWaveformPreview,
) {
    let palette = StudioTheme::palette(mode);
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

fn same_signal(left: &str, right: &str) -> bool {
    left.trim().eq_ignore_ascii_case(right.trim())
}
