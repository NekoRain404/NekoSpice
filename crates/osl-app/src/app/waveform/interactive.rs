//! Interactive waveform plot with zoom/pan/cursor support.
//!
//! Allocates a response area with click-drag + scroll sensitivity,
//! handles viewport mutations, and draws cursor overlay when enabled.
//! Separated from static previews to keep file sizes manageable.

use crate::app::theme::{StudioTheme, StudioThemeMode};
use super::preview::format_compact_f64;
use super::preview_primitives::{draw_plot_grid, plot_fill, trace_color};
use super::workspace::WaveformViewport;
use super::helpers::{draw_trace_label, ordered_previews};
use crate::waveform_summary::GuiWaveformSummary;
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Stroke, StrokeKind, Vec2};

/// Draw an interactive waveform plot with zoom/pan/cursor support.
pub(crate) fn draw_interactive_waveform_plot(
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
    summary: &GuiWaveformSummary,
    selected_signal: Option<&str>,
    viewport: &mut WaveformViewport,
    cursor_overlay: bool,
    cursor_x: &mut Option<f64>,
    _cursor_y: &mut Option<f64>,
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
            rect.center(), Align2::CENTER_CENTER,
            "No plottable signals", FontId::proportional(13.0), palette.text_muted,
        );
        return;
    }

    // ── Scroll zoom ────────────────────────────────────────────────────
    let scroll_delta = ui.input(|i| i.smooth_scroll_delta.y);
    if scroll_delta != 0.0 {
        let zoom_factor = if scroll_delta > 0.0 { 1.15 } else { 1.0 / 1.15 };
        let mouse_x = ui.input(|i| i.pointer.hover_pos().map(|p| p.x).unwrap_or(rect.center().x));
        let data_x = viewport.x_min + ((mouse_x - rect.left()) as f64 / rect.width() as f64) * (viewport.x_max - viewport.x_min);
        viewport.zoom(zoom_factor, data_x);
    }

    // ── Drag panning ───────────────────────────────────────────────────
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

    // ── Auto-fit on first render ────────────────────────────────────────
    if !viewport.user_modified && !traces.is_empty() {
        let (gmin, gmax, ymin, ymax) = data_bounds(&traces);
        viewport.fit_to_data(gmin, gmax, ymin, ymax);
    }

    let x_range = viewport.x_max - viewport.x_min;
    let y_range = (viewport.y_max - viewport.y_min).max(1e-30);
    let lane_height = plot_rect.height() / traces.len().max(1) as f32;

    // ── Draw traces ─────────────────────────────────────────────────────
    for (index, preview) in traces.iter().enumerate() {
        let lane_rect = Rect::from_min_max(
            Pos2::new(plot_rect.left(), plot_rect.top() + lane_height * index as f32),
            Pos2::new(plot_rect.right(), plot_rect.top() + lane_height * (index + 1) as f32),
        ).shrink2(Vec2::new(2.0, 6.0));

        let color = trace_color(mode, index);
        let lane_h = lane_rect.height();
        let lane_w = lane_rect.width();

        // Zero axis relative to viewport
        if viewport.y_min <= 0.0 && viewport.y_max >= 0.0 {
            let zero_y = lane_rect.bottom() - ((0.0 - viewport.y_min) / y_range) as f32 * lane_h;
            painter.line_segment(
                [Pos2::new(lane_rect.left(), zero_y), Pos2::new(lane_rect.right(), zero_y)],
                Stroke::new(0.9, palette.border),
            );
        }

        // Trace with viewport mapping
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

    // ── Cursor overlay ──────────────────────────────────────────────────
    if cursor_overlay {
        if let Some(hover_pos) = response.hover_pos() {
            let cx = hover_pos.x.clamp(plot_rect.left(), plot_rect.right());
            painter.line_segment(
                [Pos2::new(cx, plot_rect.top()), Pos2::new(cx, plot_rect.bottom())],
                Stroke::new(1.0, Color32::from_rgba_premultiplied(255, 255, 255, 80)),
            );
            *cursor_x = Some(viewport.x_min + ((cx - plot_rect.left()) as f64 / plot_rect.width() as f64) * x_range);

            // Value tooltip per trace
            for (index, preview) in traces.iter().enumerate() {
                let lane_top = plot_rect.top() + lane_height * index as f32;
                let lane_bot = lane_top + lane_height;
                if hover_pos.y >= lane_top && hover_pos.y <= lane_bot {
                    let data_x = cursor_x.unwrap_or(0.0);
                    if let Some(bucket) = preview.buckets.iter().find(|b| {
                        let mid = (b.start_time + b.end_time) * 0.5;
                        (mid - data_x).abs() < x_range * 0.02
                    }) {
                        let value = (bucket.min + bucket.max) * 0.5;
                        painter.text(
                            Pos2::new(cx + 8.0, hover_pos.y - 6.0),
                            Align2::LEFT_BOTTOM,
                            format!("{}: {}", preview.signal, format_compact_f64(value)),
                            FontId::monospace(11.0),
                            trace_color(mode, index),
                        );
                    }
                    break;
                }
            }
        }
    }

    // ── Viewport info ───────────────────────────────────────────────────
    painter.text(
        Pos2::new(plot_rect.right(), plot_rect.top() - 4.0),
        Align2::RIGHT_BOTTOM,
        format!("{} to {} | Scroll=zoom, Drag=pan",
            format_compact_f64(viewport.x_min), format_compact_f64(viewport.x_max)),
        FontId::monospace(9.0), palette.text_muted,
    );

    // ── Fit button ──────────────────────────────────────────────────────
    let fit_btn = ui.allocate_ui_with_layout(
        egui::Vec2::new(60.0, 18.0),
        egui::Layout::right_to_left(egui::Align::Center),
        |ui| ui.small_button("Fit").on_hover_text("Reset viewport to fit all data"),
    );
    if fit_btn.inner.clicked() {
        let (gmin, gmax, ymin, ymax) = data_bounds(&traces);
        viewport.fit_to_data(gmin, gmax, ymin, ymax);
    }
}

/// Compute bounding box across all trace previews.
fn data_bounds(traces: &[&crate::waveform_summary::GuiWaveformPreview]) -> (f64, f64, f64, f64) {
    let mut gmin = f64::INFINITY;
    let mut gmax = f64::NEG_INFINITY;
    let mut ymin = f64::INFINITY;
    let mut ymax = f64::NEG_INFINITY;
    for t in traces {
        gmin = gmin.min(t.time_min);
        gmax = gmax.max(t.time_max);
        ymin = ymin.min(t.value_min);
        ymax = ymax.max(t.value_max);
    }
    (gmin, gmax, ymin, ymax)
}
