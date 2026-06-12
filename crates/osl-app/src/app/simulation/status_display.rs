//! Simulation status display — shows run results, error logs, artifacts,
//! report links, and waveform previews after a simulation completes.
//!
//! Includes a "View Waveforms" button that navigates to the Waveforms workspace
//! for detailed analysis of the simulation results.

use crate::app::NekoSpiceApp;
use crate::app::navigation::StudioWorkspace;
use eframe::egui;
use crate::app::localization::UiText;
use crate::app::status_strip::severity_color;
use crate::app::theme::StudioTheme;
use crate::app::simulation::artifacts_panel::draw_simulation_artifacts_panel;
use crate::app::simulation::report_panel::draw_simulation_report_panel;
use crate::app::simulation::waveform_panel::draw_simulation_waveform_panel;
use crate::waveform_summary::GuiWaveformSummaryState;
use osl_core::RunStatus;
use osl_kicad::KicadDiagnosticSeverity;

impl NekoSpiceApp {
    /// Draw the run status section: netlist warnings, current task state,
    /// errors, ngspice log, last run info, artifacts, report, and waveform summary.
    pub(in crate::app) fn draw_simulation_run_status(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();

        // Show netlist validation warnings (persistent until next run)
        if !self.simulation_panel.netlist_warnings.is_empty() {
            for warning in &self.simulation_panel.netlist_warnings {
                ui.colored_label(
                    severity_color(mode, KicadDiagnosticSeverity::Warning),
                    format!("Warning: {}", warning),
                );
            }
            ui.add_space(4.0);
        }

        if self.simulation_panel.active_task.is_some() {
            ui.label(self.text(UiText::Running));
        }

        // Show ngspice/xyce log if available
        if let Some(run) = &self.simulation_panel.last_run {
            let log_path = run.output_dir.join("ngspice.log");
            let fallback = run.output_dir.join("xyce.log");
            let actual = if log_path.is_file() { log_path } else { fallback };
            if actual.is_file() {
                if let Ok(content) = std::fs::read_to_string(&actual) {
                    ui.separator();
                    ui.label(StudioTheme::muted_for(mode, "Simulation Log"));
                    egui::ScrollArea::vertical()
                        .id_salt("ngspice_log_viewer")
                        .max_height(100.0)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.monospace(&content);
                        });
                }
            }
        }
        if let Some(error) = &self.simulation_panel.last_error {
            ui.colored_label(
                severity_color(mode, KicadDiagnosticSeverity::Error),
                error,
            );
        }
        if let Some(run) = &self.simulation_panel.last_run {
            let color = match run.metadata.status {
                RunStatus::Passed => self.theme_palette().success,
                RunStatus::Failed => {
                    severity_color(mode, KicadDiagnosticSeverity::Error)
                }
            };
            ui.colored_label(
                color,
                format!(
                    "{}: {} ms, exit {:?}",
                    run.metadata.status.as_str(),
                    run.metadata.duration_ms,
                    run.metadata.exit_code
                ),
            );
            ui.monospace(run.output_dir.display().to_string());

            // Quick action: view waveforms in dedicated workspace
            if ui
                .button(self.text(UiText::WaveformAnalysis))
                .on_hover_text("Open Waveforms workspace for detailed analysis")
                .clicked()
            {
                self.active_workspace = StudioWorkspace::Waveforms;
            }

            draw_simulation_report_panel(ui, &run.report);
            draw_simulation_artifacts_panel(ui, run);
            draw_simulation_waveform_panel(
                ui,
                mode,
                &run.waveform,
                &mut self.simulation_panel.selected_waveform_signal,
            );
        }
    }

    /// Sync the selected waveform signal with the latest run's available signals.
    pub(in crate::app) fn sync_selected_waveform_signal(
        &mut self,
        waveform: &GuiWaveformSummaryState,
    ) {
        let GuiWaveformSummaryState::Ready(summary) = waveform else {
            self.simulation_panel.selected_waveform_signal = None;
            return;
        };
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
}
