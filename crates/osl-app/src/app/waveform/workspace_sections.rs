use crate::app::NekoSpiceApp;
use crate::app::localization::UiText;
use crate::app::theme::StudioTheme;
use super::freq_domain_preview::{draw_fft_magnitude_plot, draw_bode_plot, draw_noise_plot};
use super::interactive::draw_interactive_waveform_plot;
use super::workspace::WaveformAnalysisTab;
use super::workspace_widgets::{
    MeasurementTableLabels, measurement_table, run_stat_row, trace_chip, trace_chip_toggle,
    waveform_empty_state, waveform_mode_tab,
};
use crate::waveform_summary::{GuiWaveformSummary, GuiWaveformSummaryState};
use eframe::egui;

const MEASUREMENT_LIMIT: usize = 8;

impl NekoSpiceApp {
    /// draw waveform workspace toolbar。
    pub(crate) fn draw_waveform_workspace_toolbar(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        ui.horizontal_wrapped(|ui| {
            for tab in WaveformAnalysisTab::ALL {
                let selected = self.waveform_workspace.analysis_tab == tab;
                if waveform_mode_tab(ui, mode, self.text(tab.text_key()), selected) {
                    self.waveform_workspace.analysis_tab = tab;
                }
            }
            ui.separator();
            let cursors = self.text(UiText::Cursors);
            let prev = self.waveform_workspace.cursor_overlay;
            ui.checkbox(&mut self.waveform_workspace.cursor_overlay, cursors);
            if self.waveform_workspace.cursor_overlay && !prev {
                self.status_message = Some("Cursor overlay enabled".to_string());
            } else if !self.waveform_workspace.cursor_overlay && prev {
                self.status_message = Some("Cursor overlay disabled".to_string());
            }
            if ui.button(self.text(UiText::AutoScale)).clicked() {
                // AutoScale: reset viewport to fit all visible data
                if let Some(run) = &self.simulation_panel.last_run {
                    if let crate::waveform_summary::GuiWaveformSummaryState::Ready(summary) = &run.waveform {
                        self.status_message = Some(format!("Auto-scaled {} signals", summary.variable_count));
                    }
                }
            }
            ui.separator();
            let overlay_label = if self.waveform_workspace.overlay_mode { "Overlay: ON" } else { "Overlay: OFF" };
            if ui.selectable_label(self.waveform_workspace.overlay_mode, overlay_label).on_hover_text("Toggle multi-signal overlay mode").clicked() {
                self.waveform_workspace.overlay_mode = !self.waveform_workspace.overlay_mode;
                self.waveform_workspace.visible_signals.clear();
                if self.waveform_workspace.overlay_mode {
                    self.status_message = Some("Overlay mode: all signals shown on single lane".to_string());
                } else {
                    self.status_message = Some("Overlay mode: single signal view".to_string());
                }
            }
            ui.separator();
            if ui.button("Export CSV").on_hover_text("Export all waveform data as CSV").clicked() {
                self.export_measurements_csv();
            }
            if ui.button("Export Report").on_hover_text("Export simulation report as HTML").clicked() {
                self.export_report_html();
            }
        });
    }

    /// Draw waveform plot section, dispatching to the correct visualization
    /// based on the currently selected analysis tab.
    pub(crate) fn draw_waveform_plot_section(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let tab = self.waveform_workspace.analysis_tab;
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            match self.current_waveform_summary().cloned() {
                Some(summary) => {
                    self.sync_selected_waveform_for_summary(&summary);
                    self.draw_waveform_trace_chips(ui, &summary);
                    ui.add_space(6.0);
                    match tab {
                        super::workspace::WaveformAnalysisTab::TimeDomain => {
                            draw_interactive_waveform_plot(
                                ui,
                                mode,
                                &summary,
                                self.simulation_panel.selected_waveform_signal.as_deref(),
                                &mut self.waveform_workspace.viewport,
                                self.waveform_workspace.cursor_overlay,
                                &mut self.waveform_workspace.cursor_x,
                                &mut self.waveform_workspace.cursor_y,
                                &mut self.waveform_workspace.is_panning,
                                &mut self.waveform_workspace.pan_start,
                                330.0,
                            );
                        }
                        super::workspace::WaveformAnalysisTab::Fft => {
                            draw_fft_magnitude_plot(
                                ui,
                                mode,
                                &summary,
                                self.simulation_panel.selected_waveform_signal.as_deref(),
                            );
                        }
                        super::workspace::WaveformAnalysisTab::Bode => {
                            draw_bode_plot(
                                ui,
                                mode,
                                &summary,
                                self.simulation_panel.selected_waveform_signal.as_deref(),
                            );
                        }
                        super::workspace::WaveformAnalysisTab::Noise => {
                            draw_noise_plot(
                                ui,
                                mode,
                                &summary,
                                self.simulation_panel.selected_waveform_signal.as_deref(),
                            );
                        }
                        super::workspace::WaveformAnalysisTab::Eye => {
                            // Eye diagram: show all signals overlaid with period folding hint
                            draw_interactive_waveform_plot(
                                ui,
                                mode,
                                &summary,
                                self.simulation_panel.selected_waveform_signal.as_deref(),
                                &mut self.waveform_workspace.viewport,
                                self.waveform_workspace.cursor_overlay,
                                &mut self.waveform_workspace.cursor_x,
                                &mut self.waveform_workspace.cursor_y,
                                &mut self.waveform_workspace.is_panning,
                                &mut self.waveform_workspace.pan_start,
                                330.0,
                            );
                            ui.add_space(4.0);
                            ui.horizontal_wrapped(|ui| {
                                ui.label(StudioTheme::muted_for(mode,
                                    "Tip: Eye diagram works best with periodic digital signals. Use .step to overlay multiple periods."));
                            });
                        }
                    }
                }
                None => waveform_empty_state(
                    ui,
                    mode,
                    self.text(UiText::NoWaveform),
                    self.text(UiText::WaveformEmptyHint),
                ),
            }
        });
    }

    /// draw waveform detail sections。
    pub(crate) fn draw_waveform_detail_sections(&mut self, ui: &mut egui::Ui) {
        let width = ui.available_width();
        if width < 720.0 {
            self.draw_waveform_measurements_section(ui);
            ui.add_space(8.0);
            self.draw_waveform_run_statistics_section(ui);
            return;
        }

        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width((width * 0.58).max(360.0));
                self.draw_waveform_measurements_section(ui);
            });
            ui.add_space(10.0);
            ui.vertical(|ui| {
                ui.set_width(ui.available_width().max(260.0));
                self.draw_waveform_run_statistics_section(ui);
            });
        });
    }

    /// draw waveform workspace panel。
    pub(crate) fn draw_waveform_workspace_panel(&mut self, ui: &mut egui::Ui) {
        self.poll_simulation_task();
        let mode = self.theme_mode();
        ui.heading(self.text(UiText::WaveformAnalysis));
        ui.label(StudioTheme::muted_for(
            mode,
            self.text(UiText::WaveformAnalysisCaption),
        ));
        ui.add_space(8.0);
        self.draw_waveform_cursor_panel(ui);
        ui.add_space(8.0);
        self.draw_waveform_compare_panel(ui);
        ui.add_space(8.0);
        self.draw_waveform_export_panel(ui);
    }

    pub(crate) fn current_waveform_summary(&self) -> Option<&GuiWaveformSummary> {
        let run = self.simulation_panel.last_run.as_ref()?;
        let GuiWaveformSummaryState::Ready(summary) = &run.waveform else {
            return None;
        };
        Some(summary)
    }

    fn sync_selected_waveform_for_summary(&mut self, summary: &GuiWaveformSummary) {
        let keep_current = self
            .simulation_panel
            .selected_waveform_signal
            .as_deref()
            .is_some_and(|signal| summary.has_preview_signal(signal));
        if !keep_current {
            self.simulation_panel.selected_waveform_signal =
                summary.default_signal_name().map(ToOwned::to_owned);
        }
    }


    fn draw_waveform_trace_chips(&mut self, ui: &mut egui::Ui, summary: &GuiWaveformSummary) {
        let mode = self.theme_mode();
        let overlay = self.waveform_workspace.overlay_mode;
        ui.horizontal_wrapped(|ui| {
            for preview in summary.previews.iter().take(6) {
                if overlay {
                    // Overlay mode: toggle visibility for multi-signal display
                    let visible = self.waveform_workspace.is_signal_visible(&preview.signal);
                    if trace_chip_toggle(ui, mode, &preview.signal, &preview.unit, visible) {
                        self.waveform_workspace.toggle_signal(&preview.signal);
                    }
                } else {
                    // Single-select mode: click to select one signal
                    let selected = self
                        .simulation_panel
                        .selected_waveform_signal
                        .as_deref()
                        .is_some_and(|signal| signal.eq_ignore_ascii_case(&preview.signal));
                    if trace_chip(ui, mode, &preview.signal, &preview.unit, selected) {
                        self.simulation_panel.selected_waveform_signal = Some(preview.signal.clone());
                    }
                }
            }
            if summary.omitted_preview_count > 0 {
                ui.label(StudioTheme::muted_for(
                    mode,
                    format!(
                        "{} {}",
                        summary.omitted_preview_count,
                        self.text(UiText::MoreSignals)
                    ),
                ));
            }
        });
    }

    fn draw_waveform_measurements_section(&self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::Measurements),
            ));
            let Some(summary) = self.current_waveform_summary() else {
                ui.label(StudioTheme::muted_for(mode, self.text(UiText::NoWaveform)));
                return;
            };
            let labels = MeasurementTableLabels {
                signal: self.text(UiText::Signal),
                last: self.text(UiText::LastValue),
                average: self.text(UiText::Average),
                rms: self.text(UiText::Rms),
                peak_to_peak: self.text(UiText::PeakToPeak),
                samples: self.text(UiText::Samples),
                more_variables: self.text(UiText::MoreVariables),
            };
            measurement_table(ui, &labels, &summary.variables, MEASUREMENT_LIMIT);
        });
    }

    fn draw_waveform_run_statistics_section(&self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::RunStatistics),
            ));
            let Some(run) = &self.simulation_panel.last_run else {
                ui.label(StudioTheme::muted_for(mode, self.text(UiText::NoRecentRun)));
                return;
            };
            let point_count = self
                .current_waveform_summary()
                .map(|summary| summary.point_count.to_string())
                .unwrap_or_else(|| "0".to_string());
            let variable_count = self
                .current_waveform_summary()
                .map(|summary| summary.variable_count.to_string())
                .unwrap_or_else(|| "0".to_string());
            run_stat_row(
                ui,
                mode,
                self.text(UiText::StatusConsole),
                run.metadata.status.as_str(),
            );
            run_stat_row(ui, mode, self.text(UiText::Backend), &run.metadata.backend);
            run_stat_row(
                ui,
                mode,
                self.text(UiText::LastRun),
                &format!("{} ms", run.metadata.duration_ms),
            );
            run_stat_row(ui, mode, self.text(UiText::Points), &point_count);
            run_stat_row(ui, mode, self.text(UiText::Signals), &variable_count);
            run_stat_row(
                ui,
                mode,
                self.text(UiText::Artifacts),
                &run.metadata.artifacts.len().to_string(),
            );
        });
    }
}
