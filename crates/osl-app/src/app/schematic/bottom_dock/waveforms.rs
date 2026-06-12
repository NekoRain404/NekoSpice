//! Bottom dock waveform-related tabs: Waveforms, FFT, and Bode plot.
//!
//! These tabs display simulation waveform results including signal lists,
//! stacked waveform previews, frequency-domain analysis, and Bode magnitude/phase.

use crate::app::NekoSpiceApp;
use crate::app::theme::StudioTheme;
use crate::waveform_summary::GuiWaveformSummaryState;
use eframe::egui::{self, Vec2};

impl NekoSpiceApp {
    /// 波形标签页：信号列表 + 叠加波形预览。
    pub(crate) fn draw_bottom_waveforms_tab(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let palette = self.theme_palette();
        let Some(run) = &self.simulation_panel.last_run else {
            ui.label(StudioTheme::muted_for(mode, "Run a simulation to view waveforms"));
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
                        ui, mode, &variable.name,
                        &format!("{}: min={:.3} max={:.3}", variable.unit, variable.min, variable.max),
                        palette.accent,
                    );
                }
                if summary.omitted_variable_count > 0 {
                    ui.label(StudioTheme::muted_for(
                        mode, format!("+{} more variables", summary.omitted_variable_count),
                    ));
                }
            }
            GuiWaveformSummaryState::Missing { .. } => {
                ui.label(StudioTheme::muted_for(mode, "No waveform data loaded"));
            }
            GuiWaveformSummaryState::Error { message, .. } => {
                super::super::workspace_widgets::bottom_console_line(
                    ui, mode, &format!("Waveform error: {message}"), palette.danger,
                );
            }
        }
    }

    /// FFT 标签页：频域分析预览。
    pub(crate) fn draw_bottom_fft_tab(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let palette = self.theme_palette();
        let Some(run) = &self.simulation_panel.last_run else {
            ui.label(StudioTheme::muted_for(mode, "Run a simulation to enable FFT"));
            return;
        };
        let GuiWaveformSummaryState::Ready(summary) = &run.waveform else {
            ui.label(StudioTheme::muted_for(mode, "Run a transient simulation for FFT analysis"));
            return;
        };
        ui.label(StudioTheme::section_title_for(mode, format!("FFT ({})", summary.plot_name)));
        ui.add_space(2.0);
        let freq_vars: Vec<_> = summary.variables.iter()
            .filter(|v| {
                let name = v.name.to_lowercase();
                name.starts_with("v(") || name.starts_with("i(")
                    || name.contains("freq") || v.unit.to_lowercase().contains("hz")
            })
            .collect();
        if freq_vars.is_empty() {
            ui.label(StudioTheme::muted_for(mode, "Select voltage/current signals for FFT analysis"));
        } else {
            for variable in freq_vars.iter().take(4) {
                super::super::workspace_widgets::signal_row(
                    ui, mode, &variable.name,
                    &format!("{}: {} pts", variable.unit, summary.point_count),
                    palette.warning,
                );
            }
        }
        // Mini frequency-domain chart placeholder
        ui.add_space(4.0);
        let desired_size = Vec2::new(ui.available_width().max(120.0), 50.0);
        let (rect, _) = ui.allocate_exact_size(desired_size, egui::Sense::hover());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 2.0, crate::app::waveform::preview_primitives::plot_fill(mode));
        painter.rect_stroke(rect, 2.0, egui::Stroke::new(1.0, palette.border), egui::StrokeKind::Inside);
        let plot_rect = rect.shrink(6.0);
        painter.text(plot_rect.left_bottom() + Vec2::new(4.0, -2.0), egui::Align2::LEFT_BOTTOM, "0 Hz", egui::FontId::monospace(9.0), palette.text_muted);
        painter.text(plot_rect.right_bottom() + Vec2::new(-4.0, -2.0), egui::Align2::RIGHT_BOTTOM, "Fs/2", egui::FontId::monospace(9.0), palette.text_muted);
        painter.text(plot_rect.center(), egui::Align2::CENTER_CENTER, "FFT visualization (run transient sim)", egui::FontId::proportional(10.0), palette.text_muted);
    }

    /// Bode 标签页：AC 分析的幅频/相频显示。
    pub(crate) fn draw_bottom_bode_tab(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let palette = self.theme_palette();
        let Some(run) = &self.simulation_panel.last_run else {
            ui.label(StudioTheme::muted_for(mode, "Run a simulation to enable Bode plot"));
            return;
        };
        let GuiWaveformSummaryState::Ready(summary) = &run.waveform else {
            ui.label(StudioTheme::muted_for(mode, "Run an AC analysis for Bode plot"));
            return;
        };
        ui.label(StudioTheme::section_title_for(mode, format!("Bode ({})", summary.plot_name)));
        ui.add_space(2.0);
        let ac_vars: Vec<_> = summary.variables.iter()
            .filter(|v| v.name.starts_with("v(") || v.name.starts_with("i("))
            .collect();
        if ac_vars.is_empty() {
            ui.label(StudioTheme::muted_for(mode, "Run an AC analysis to generate Bode data"));
        } else {
            for variable in ac_vars.iter().take(4) {
                super::super::workspace_widgets::signal_row(
                    ui, mode, &variable.name,
                    &format!("{}: {} pts", variable.unit, summary.point_count),
                    palette.warning,
                );
            }
        }
        // Dual mini chart: magnitude (top) + phase (bottom)
        ui.add_space(4.0);
        let half = 40.0;
        for (label, y_label) in [("Magnitude (dB)", "0 dB"), ("Phase (deg)", "0°")] {
            let desired_size = Vec2::new(ui.available_width().max(120.0), half);
            let (rect, _) = ui.allocate_exact_size(desired_size, egui::Sense::hover());
            let painter = ui.painter_at(rect);
            painter.rect_filled(rect, 2.0, crate::app::waveform::preview_primitives::plot_fill(mode));
            painter.rect_stroke(rect, 2.0, egui::Stroke::new(1.0, palette.border), egui::StrokeKind::Inside);
            let plot_rect = rect.shrink(4.0);
            painter.text(plot_rect.left_center() + Vec2::new(2.0, 0.0), egui::Align2::LEFT_CENTER, y_label, egui::FontId::monospace(8.0), palette.text_muted);
            painter.text(plot_rect.center(), egui::Align2::CENTER_CENTER, label, egui::FontId::proportional(9.0), palette.text_muted);
        }
    }
}
