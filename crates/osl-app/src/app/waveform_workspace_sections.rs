use super::NekoSpiceApp;
use super::localization::UiText;
use super::theme::StudioTheme;
use super::waveform_preview::{draw_stacked_waveform_preview, format_compact_f64};
use super::waveform_workspace::WaveformAnalysisTab;
use super::waveform_workspace_widgets::{
    MeasurementTableLabels, cursor_row, measurement_table, run_stat_row, trace_chip,
    waveform_empty_state, waveform_mode_tab, waveform_summary_card,
};
use crate::waveform_summary::{GuiWaveformSummary, GuiWaveformSummaryState};
use eframe::egui;

const MEASUREMENT_LIMIT: usize = 8;

impl NekoSpiceApp {
    pub(super) fn draw_waveform_workspace_toolbar(&mut self, ui: &mut egui::Ui) {
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
            ui.checkbox(&mut self.waveform_workspace.cursor_overlay, cursors);
            let _ = ui.button(self.text(UiText::AutoScale));
        });
    }

    pub(super) fn draw_waveform_plot_section(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            match self.current_waveform_summary().cloned() {
                Some(summary) => {
                    self.sync_selected_waveform_for_summary(&summary);
                    self.draw_waveform_trace_chips(ui, &summary);
                    ui.add_space(6.0);
                    draw_stacked_waveform_preview(
                        ui,
                        mode,
                        &summary,
                        self.simulation_panel.selected_waveform_signal.as_deref(),
                        330.0,
                    );
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

    pub(super) fn draw_waveform_detail_sections(&self, ui: &mut egui::Ui) {
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

    pub(super) fn draw_waveform_workspace_panel(&mut self, ui: &mut egui::Ui) {
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

    fn current_waveform_summary(&self) -> Option<&GuiWaveformSummary> {
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
        ui.horizontal_wrapped(|ui| {
            for preview in summary.previews.iter().take(6) {
                let selected = self
                    .simulation_panel
                    .selected_waveform_signal
                    .as_deref()
                    .is_some_and(|signal| signal.eq_ignore_ascii_case(&preview.signal));
                if trace_chip(ui, mode, &preview.signal, &preview.unit, selected) {
                    self.simulation_panel.selected_waveform_signal = Some(preview.signal.clone());
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

    fn draw_waveform_cursor_panel(&self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::Cursors),
            ));
            let Some(summary) = self.current_waveform_summary() else {
                ui.label(StudioTheme::muted_for(mode, self.text(UiText::NoWaveform)));
                return;
            };
            let selected = self.simulation_panel.selected_waveform_signal.as_deref();
            let variable = selected.and_then(|signal| summary.variable_summary_for_signal(signal));
            if let Some(variable) = variable {
                cursor_row(
                    ui,
                    mode,
                    "A",
                    &variable.name,
                    &format_compact_f64(variable.first),
                );
                cursor_row(
                    ui,
                    mode,
                    "B",
                    &variable.name,
                    &format_compact_f64(variable.last),
                );
                cursor_row(
                    ui,
                    mode,
                    "Delta",
                    &variable.name,
                    &format_compact_f64(variable.peak_to_peak),
                );
            } else {
                ui.label(StudioTheme::muted_for(mode, self.text(UiText::NoSelection)));
            }
        });
    }

    fn draw_waveform_compare_panel(&self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::ComparisonManager),
            ));
            let Some(run) = &self.simulation_panel.last_run else {
                ui.label(StudioTheme::muted_for(mode, self.text(UiText::NoRecentRun)));
                return;
            };
            waveform_summary_card(
                ui,
                mode,
                self.text(UiText::BaselineRun),
                run.metadata.status.as_str(),
                &run.output_dir.display().to_string(),
            );
        });
    }

    fn draw_waveform_export_panel(&self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::ExportShare),
            ));
            ui.vertical(|ui| {
                let _ = ui.button(self.text(UiText::ExportWaveforms));
                let _ = ui.button(self.text(UiText::ExportCsv));
                let _ = ui.button(self.text(UiText::ExportReport));
            });
        });
    }
}
