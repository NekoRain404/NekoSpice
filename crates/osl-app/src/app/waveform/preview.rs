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

/// Draw an interactive waveform plot with zoom/pan/cursor support.
///
/// Allocates a response area with click-drag + scroll sensitivity,
/// handles viewport mutations, and draws cursor overlay when enabled.
pub(crate) fn draw_interactive_waveform_plot(
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
    summary: &GuiWaveformSummary,
    selected_signal: Option<&str>,
    viewport: &mut super::workspace::WaveformViewport,
    cursor_overlay: bool,
    cursor_x: &mut Option<f64>,
    cursor_y: &mut Option<f64>,
    is_panning: &mut bool,
    pan_start: &mut Option<egui::Pos2>,
    height: f32,
) {
    let desired_size = Vec2::new(ui.available_width().max(320.0), height);
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click_and_drag());
    let palette = StudioTheme::palette(mode);
    let painter = ui.painter_at(rect);
    let plot_rect = rect.shrink2(Vec2::new(14.0, 12.0));

    // Background
    painter.rect_filled(rect, 4.0, plot_fill(mode));
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0, palette.border), StrokeKind::Inside);
    draw_plot_grid(&painter, plot_rect, palette.border.linear_multiply(0.45));

    let traces = ordered_previews(summary, selected_signal);
    if traces.is_empty() {
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            "No plottable signals",
            FontId::proportional(13.0),
            palette.text_muted,
        );
        return;
    }

    // Handle scroll zoom
    let scroll_delta = ui.input(|i| i.smooth_scroll_delta.y);
    if scroll_delta != 0.0 {
        let zoom_factor = if scroll_delta > 0.0 { 1.15 } else { 1.0 / 1.15 };
        let mouse_x = ui.input(|i| i.pointer.hover_pos().map(|p| p.x).unwrap_or(rect.center().x));
        // Map screen x to data x
        let data_x = viewport.x_min + ((mouse_x - rect.left()) as f64 / rect.width() as f64) * (viewport.x_max - viewport.x_min);
        viewport.zoom(zoom_factor, data_x);
    }

    // Handle drag panning
    if response.drag_started() {
        *is_panning = true;
        *pan_start = response.interact_pointer_pos();
    }
    if *is_panning {
        if let Some(current_pos) = response.interact_pointer_pos() {
            if let Some(start) = pan_start {
                let dx = (start.x - current_pos.x) / rect.width();
                let dy = (current_pos.y - start.y) / rect.height();
                viewport.pan(dx as f64, dy as f64);
                *pan_start = Some(current_pos);
            }
        }
        if response.drag_stopped() {
            *is_panning = false;
            *pan_start = None;
        }
    }

    // Auto-fit on first render
    if !viewport.user_modified && !traces.is_empty() {
        let mut global_min = f64::INFINITY;
        let mut global_max = f64::NEG_INFINITY;
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;
        for t in &traces {
            global_min = global_min.min(t.time_min);
            global_max = global_max.max(t.time_max);
            y_min = y_min.min(t.value_min);
            y_max = y_max.max(t.value_max);
        }
        viewport.fit_to_data(global_min, global_max, y_min, y_max);
    }

    // Draw traces using viewport mapping
    let x_range = viewport.x_max - viewport.x_min;
    let y_range = (viewport.y_max - viewport.y_min).max(1e-30);
    let lane_height = plot_rect.height() / traces.len().max(1) as f32;

    for (index, preview) in traces.iter().enumerate() {
        let lane_rect = Rect::from_min_max(
            Pos2::new(plot_rect.left(), plot_rect.top() + lane_height * index as f32),
            Pos2::new(plot_rect.right(), plot_rect.top() + lane_height * (index + 1) as f32),
        )
        .shrink2(Vec2::new(2.0, 6.0));

        let color = trace_color(mode, index);
        let lane_h = lane_rect.height();
        let lane_w = lane_rect.width();

        // Draw zero axis relative to viewport
        if viewport.y_min <= 0.0 && viewport.y_max >= 0.0 {
            let zero_y = lane_rect.bottom() - ((0.0 - viewport.y_min) / y_range) as f32 * lane_h;
            painter.line_segment(
                [Pos2::new(lane_rect.left(), zero_y), Pos2::new(lane_rect.right(), zero_y)],
                Stroke::new(0.9, palette.border),
            );
        }

        // Draw trace with viewport mapping
        let stroke = Stroke::new(1.35, color);
        let mut prev_point: Option<Pos2> = None;
        for bucket in &preview.buckets {
            let t_mid = (bucket.start_time + bucket.end_time) * 0.5;
            if t_mid < viewport.x_min || t_mid > viewport.x_max {
                prev_point = None;
                continue;
            }
            let x = lane_rect.left() + ((t_mid - viewport.x_min) / x_range) as f32 * lane_w;
            let y_min_s = lane_rect.bottom() - ((bucket.min - viewport.y_min) / y_range) as f32 * lane_h;
            let y_max_s = lane_rect.bottom() - ((bucket.max - viewport.y_min) / y_range) as f32 * lane_h;
            let y_mid = (y_min_s + y_max_s) * 0.5;
            let point = Pos2::new(x, y_mid);
            if bucket.samples > 1 {
                painter.line_segment([Pos2::new(x, y_min_s), Pos2::new(x, y_max_s)], stroke);
            }
            if let Some(prev) = prev_point {
                painter.line_segment([prev, point], stroke);
            }
            prev_point = Some(point);
        }

        draw_trace_label(&painter, lane_rect, preview, color, palette.text_muted);
    }

    // Draw cursor overlay
    if cursor_overlay {
        if let Some(hover_pos) = response.hover_pos() {
            let cx = hover_pos.x.clamp(plot_rect.left(), plot_rect.right());
            // Vertical cursor line
            painter.line_segment(
                [Pos2::new(cx, plot_rect.top()), Pos2::new(cx, plot_rect.bottom())],
                Stroke::new(1.0, Color32::from_rgba_premultiplied(255, 255, 255, 80)),
            );
            // Map screen x back to data x
            *cursor_x = Some(viewport.x_min + ((cx - plot_rect.left()) as f64 / plot_rect.width() as f64) * x_range);
            *cursor_y = None; // Will be set per-trace below

            // Show value tooltip for each trace
            for (index, preview) in traces.iter().enumerate() {
                let lane_height_each = plot_rect.height() / traces.len().max(1) as f32;
                let lane_top = plot_rect.top() + lane_height_each * index as f32;
                let lane_bot = lane_top + lane_height_each;
                if hover_pos.y >= lane_top && hover_pos.y <= lane_bot {
                    // Find nearest bucket by time
                    let data_x = cursor_x.unwrap_or(0.0);
                    if let Some(bucket) = preview.buckets.iter().find(|b| {
                        let mid = (b.start_time + b.end_time) * 0.5;
                        (mid - data_x).abs() < x_range * 0.02
                    }) {
                        let value = (bucket.min + bucket.max) * 0.5;
                        let label = format!("{}: {}", preview.signal, format_compact_f64(value));
                        painter.text(
                            Pos2::new(cx + 8.0, hover_pos.y - 6.0),
                            Align2::LEFT_BOTTOM,
                            &label,
                            FontId::monospace(11.0),
                            trace_color(mode, index),
                        );
                    }
                    break;
                }
            }
        }
    }

    // Viewport info overlay
    let info = format!(
        "View: {} to {} | Scroll=zoom, Drag=pan",
        format_compact_f64(viewport.x_min),
        format_compact_f64(viewport.x_max),
    );
    painter.text(
        Pos2::new(plot_rect.right(), plot_rect.top() - 4.0),
        Align2::RIGHT_BOTTOM,
        &info,
        FontId::monospace(9.0),
        palette.text_muted,
    );

    // Fit button
    let fit_btn = ui.allocate_ui_with_layout(
        egui::Vec2::new(60.0, 18.0),
        egui::Layout::right_to_left(egui::Align::Center),
        |ui| {
            ui.small_button("Fit")
                .on_hover_text("Reset viewport to fit all data")
        },
    );
    if fit_btn.inner.clicked() {
        let mut global_min = f64::INFINITY;
        let mut global_max = f64::NEG_INFINITY;
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;
        for t in &traces {
            global_min = global_min.min(t.time_min);
            global_max = global_max.max(t.time_max);
            y_min = y_min.min(t.value_min);
            y_max = y_max.max(t.value_max);
        }
        viewport.fit_to_data(global_min, global_max, y_min, y_max);
    }
}
