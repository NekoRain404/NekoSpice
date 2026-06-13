//! Bottom dock waveform-related tabs: Waveforms, FFT, and Bode plot.
//!
//! These tabs display simulation waveform results including signal lists,
//! stacked waveform previews, frequency-domain analysis, and Bode magnitude/phase.
//! FFT and Bode use the real FFT engine from `osl_waveform::fft`.

use crate::app::NekoSpiceApp;
use crate::app::theme::StudioTheme;
use crate::waveform_summary::GuiWaveformSummaryState;
use eframe::egui::{self, Vec2};

/// Helper: reconstruct time-domain samples from waveform bucket data.
fn samples_from_preview(preview: &crate::waveform_summary::GuiWaveformPreview) -> Vec<f64> {
    preview
        .buckets
        .iter()
        .flat_map(|b| std::iter::repeat_n((b.min + b.max) * 0.5, b.samples))
        .collect()
}

/// Helper: compute FFT bins from a waveform preview, returning None if data is insufficient.
fn compute_preview_fft(
    preview: &crate::waveform_summary::GuiWaveformPreview,
) -> Option<Vec<osl_waveform::fft::FftBin>> {
    let time_range = preview.time_max - preview.time_min;
    if time_range <= 0.0 || preview.source_points < 4 {
        return None;
    }
    let dt = time_range / (preview.source_points - 1) as f64;
    let samples = samples_from_preview(preview);
    if samples.len() < 4 {
        return None;
    }
    Some(osl_waveform::fft::compute_fft_bins(
        &samples,
        dt,
        osl_waveform::fft::WindowFunction::Hanning,
    ))
}

/// Helper: draw a trace line from FFT bins into a plot rect.
fn draw_fft_trace(
    painter: &egui::Painter,
    plot_rect: eframe::egui::Rect,
    bins: &[osl_waveform::fft::FftBin],
    get_value: impl Fn(&osl_waveform::fft::FftBin) -> f64,
    color: eframe::egui::Color32,
) {
    if bins.is_empty() {
        return;
    }
    let vals: Vec<f64> = bins.iter().map(get_value).collect();
    let max_val = vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let min_val = vals
        .iter()
        .cloned()
        .filter(|v| v.abs() < 299.0)
        .fold(f64::INFINITY, f64::min);
    let range = (max_val - min_val).max(1.0);
    let stroke = egui::Stroke::new(1.5, color);
    let mut prev: Option<eframe::egui::Pos2> = None;
    for (i, &val) in vals.iter().enumerate() {
        if val.abs() > 299.0 {
            prev = None;
            continue;
        }
        let x_norm = i as f32 / (vals.len() - 1).max(1) as f32;
        let x = plot_rect.left() + x_norm * plot_rect.width();
        let y_norm = ((val - min_val) / range).clamp(0.0, 1.0) as f32;
        let y = plot_rect.bottom() - y_norm * plot_rect.height();
        let point = eframe::egui::Pos2::new(x, y);
        if let Some(p) = prev {
            painter.line_segment([p, point], stroke);
        }
        prev = Some(point);
    }
}

impl NekoSpiceApp {
    /// Waveform tab: signal list + stacked waveform preview.
    pub(crate) fn draw_bottom_waveforms_tab(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let palette = self.theme_palette();
        let Some(run) = &self.simulation_panel.last_run else {
            ui.label(StudioTheme::muted_for(
                mode,
                "Run a simulation to view waveforms",
            ));
            return;
        };
        match &run.waveform {
            GuiWaveformSummaryState::Ready(summary) => {
                ui.label(StudioTheme::section_title_for(
                    mode,
                    format!("Signals ({})", summary.variable_count),
                ));
                ui.add_space(2.0);
                crate::app::waveform::preview::draw_stacked_waveform_preview(
                    ui, mode, summary, None, 100.0,
                );
                ui.add_space(4.0);
                for variable in &summary.variables {
                    super::super::workspace_widgets::signal_row(
                        ui,
                        mode,
                        &variable.name,
                        &format!(
                            "{}: min={:.3} max={:.3}",
                            variable.unit, variable.min, variable.max
                        ),
                        palette.accent,
                    );
                }
                if summary.omitted_variable_count > 0 {
                    ui.label(StudioTheme::muted_for(
                        mode,
                        format!("+{} more variables", summary.omitted_variable_count),
                    ));
                }
            }
            GuiWaveformSummaryState::Missing { .. } => {
                ui.label(StudioTheme::muted_for(mode, "No waveform data loaded"));
            }
            GuiWaveformSummaryState::Error { message, .. } => {
                super::super::workspace_widgets::bottom_console_line(
                    ui,
                    mode,
                    &format!("Waveform error: {message}"),
                    palette.danger,
                );
            }
        }
    }

    /// FFT tab: real frequency-domain analysis using the FFT engine.
    pub(crate) fn draw_bottom_fft_tab(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let palette = self.theme_palette();
        let Some(run) = &self.simulation_panel.last_run else {
            ui.label(StudioTheme::muted_for(
                mode,
                "Run a simulation to enable FFT",
            ));
            return;
        };
        let GuiWaveformSummaryState::Ready(summary) = &run.waveform else {
            ui.label(StudioTheme::muted_for(
                mode,
                "Run a transient simulation for FFT analysis",
            ));
            return;
        };
        ui.label(StudioTheme::section_title_for(
            mode,
            format!("FFT ({})", summary.plot_name),
        ));
        ui.add_space(2.0);
        // Signal list
        let freq_vars: Vec<_> = summary
            .variables
            .iter()
            .filter(|v| {
                let name = v.name.to_lowercase();
                name.starts_with("v(")
                    || name.starts_with("i(")
                    || name.contains("freq")
                    || v.unit.to_lowercase().contains("hz")
            })
            .collect();
        if freq_vars.is_empty() {
            ui.label(StudioTheme::muted_for(
                mode,
                "Select voltage/current signals for FFT analysis",
            ));
            return;
        }
        for variable in freq_vars.iter().take(4) {
            super::super::workspace_widgets::signal_row(
                ui,
                mode,
                &variable.name,
                &format!("{}: {} pts", variable.unit, summary.point_count),
                palette.warning,
            );
        }
        // Real FFT magnitude visualization
        ui.add_space(4.0);
        let desired_size = Vec2::new(ui.available_width().max(120.0), 80.0);
        let (rect, _) = ui.allocate_exact_size(desired_size, egui::Sense::hover());
        let painter = ui.painter_at(rect);
        painter.rect_filled(
            rect,
            2.0,
            crate::app::waveform::preview_primitives::plot_fill(mode),
        );
        painter.rect_stroke(
            rect,
            2.0,
            egui::Stroke::new(1.0, palette.border),
            egui::StrokeKind::Inside,
        );
        let plot_rect = rect.shrink(8.0);
        if let Some(first_preview) = summary.previews.first()
            && let Some(bins) = compute_preview_fft(first_preview)
        {
            draw_fft_trace(
                &painter,
                plot_rect,
                &bins,
                |b| b.magnitude_db,
                palette.accent,
            );
        }
        painter.text(
            plot_rect.left_bottom() + Vec2::new(2.0, 2.0),
            egui::Align2::LEFT_TOP,
            "0 Hz",
            egui::FontId::monospace(8.0),
            palette.text_muted,
        );
        painter.text(
            plot_rect.right_bottom() + Vec2::new(-2.0, 2.0),
            egui::Align2::RIGHT_TOP,
            "Fs/2",
            egui::FontId::monospace(8.0),
            palette.text_muted,
        );
        painter.text(
            plot_rect.left_center() + Vec2::new(-2.0, 0.0),
            egui::Align2::RIGHT_CENTER,
            "dB",
            egui::FontId::monospace(8.0),
            palette.text_muted,
        );
    }

    /// Bode tab: real magnitude/phase frequency-domain plots.
    pub(crate) fn draw_bottom_bode_tab(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let palette = self.theme_palette();
        let Some(run) = &self.simulation_panel.last_run else {
            ui.label(StudioTheme::muted_for(
                mode,
                "Run a simulation to enable Bode plot",
            ));
            return;
        };
        let GuiWaveformSummaryState::Ready(summary) = &run.waveform else {
            ui.label(StudioTheme::muted_for(
                mode,
                "Run an AC analysis for Bode plot",
            ));
            return;
        };
        ui.label(StudioTheme::section_title_for(
            mode,
            format!("Bode ({})", summary.plot_name),
        ));
        ui.add_space(2.0);
        let ac_vars: Vec<_> = summary
            .variables
            .iter()
            .filter(|v| v.name.starts_with("v(") || v.name.starts_with("i("))
            .collect();
        if ac_vars.is_empty() {
            ui.label(StudioTheme::muted_for(
                mode,
                "Run an AC analysis to generate Bode data",
            ));
            return;
        }
        for variable in ac_vars.iter().take(4) {
            super::super::workspace_widgets::signal_row(
                ui,
                mode,
                &variable.name,
                &format!("{}: {} pts", variable.unit, summary.point_count),
                palette.warning,
            );
        }
        // Compute FFT bins for real Bode visualization
        let fft_bins = summary.previews.first().and_then(compute_preview_fft);
        let bins_ref = fft_bins.as_deref();
        // Dual mini chart: magnitude (top) + phase (bottom)
        ui.add_space(4.0);
        let half = 40.0;
        // Magnitude panel
        {
            let desired_size = Vec2::new(ui.available_width().max(120.0), half);
            let (rect, _) = ui.allocate_exact_size(desired_size, egui::Sense::hover());
            let painter = ui.painter_at(rect);
            painter.rect_filled(
                rect,
                2.0,
                crate::app::waveform::preview_primitives::plot_fill(mode),
            );
            painter.rect_stroke(
                rect,
                2.0,
                egui::Stroke::new(1.0, palette.border),
                egui::StrokeKind::Inside,
            );
            let plot_rect = rect.shrink(4.0);
            if let Some(bins) = bins_ref {
                draw_fft_trace(
                    &painter,
                    plot_rect,
                    bins,
                    |b| b.magnitude_db,
                    palette.accent,
                );
            }
            painter.text(
                plot_rect.left_center() + Vec2::new(2.0, 0.0),
                egui::Align2::LEFT_CENTER,
                "0 dB",
                egui::FontId::monospace(8.0),
                palette.text_muted,
            );
            painter.text(
                plot_rect.center(),
                egui::Align2::CENTER_CENTER,
                "Magnitude (dB)",
                egui::FontId::proportional(9.0),
                palette.text_muted,
            );
        }
        // Phase panel
        {
            let desired_size = Vec2::new(ui.available_width().max(120.0), half);
            let (rect, _) = ui.allocate_exact_size(desired_size, egui::Sense::hover());
            let painter = ui.painter_at(rect);
            painter.rect_filled(
                rect,
                2.0,
                crate::app::waveform::preview_primitives::plot_fill(mode),
            );
            painter.rect_stroke(
                rect,
                2.0,
                egui::Stroke::new(1.0, palette.border),
                egui::StrokeKind::Inside,
            );
            let plot_rect = rect.shrink(4.0);
            if let Some(bins) = bins_ref {
                draw_fft_trace(&painter, plot_rect, bins, |b| b.phase_deg, palette.warning);
            }
            painter.text(
                plot_rect.left_center() + Vec2::new(2.0, 0.0),
                egui::Align2::LEFT_CENTER,
                "0 deg",
                egui::FontId::monospace(8.0),
                palette.text_muted,
            );
            painter.text(
                plot_rect.center(),
                egui::Align2::CENTER_CENTER,
                "Phase (deg)",
                egui::FontId::proportional(9.0),
                palette.text_muted,
            );
        }
    }
}
