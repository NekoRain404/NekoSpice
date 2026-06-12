use crate::app::NekoSpiceApp;
use crate::app::localization::UiText;
use super::widgets::{compact_action, property_row, status_pill};
use crate::app::status_strip::severity_color;
use crate::app::theme::StudioTheme;
use crate::app::simulation::state::StepSweep;
use eframe::egui;
use osl_kicad::KicadDiagnosticSeverity;

impl NekoSpiceApp {
    /// draw schematic simulator tab。
    pub(crate) fn draw_schematic_simulator_tab(&mut self, ui: &mut egui::Ui) {
        self.poll_simulation_task();
        let mode = self.theme_mode();
        let palette = self.theme_palette();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::Simulator),
            ));
            ui.horizontal_wrapped(|ui| {
                status_pill(ui, mode, self.simulation_panel.backend.label(), palette.success);
                let status = if self.simulation_panel.active_task.is_some() {
                    self.text(UiText::Running)
                } else {
                    self.text(UiText::Ready)
                };
                status_pill(ui, mode, status, palette.accent);
            });
            ui.add_space(6.0);

            // Current analysis directive
            let directive = format!(
                "{} {}",
                self.simulation_panel.directive_kind,
                self.simulation_panel.analysis_params.to_body().trim()
            ).trim().to_string();
            property_row(ui, mode, "Directive", &directive);

            // Step sweep summary (if active)
            if let StepSweep::Parametric { param_name, sweep_mode, start, stop, step } = &self.simulation_panel.step_sweep {
                let sweep_desc = match sweep_mode.as_str() {
                    "list" => format!("{} list {}", param_name, start),
                    "lin" => format!("{} {} to {} step {}", param_name, start, stop, step),
                    "dec" => format!("{} dec {} pts/dec {} to {}", param_name, step, start, stop),
                    "oct" => format!("{} oct {} pts/oct {} to {}", param_name, step, start, stop),
                    _ => format!("{} {} {} {}", param_name, sweep_mode, start, stop),
                };
                property_row(ui, mode, "Step", &sweep_desc);
            }

            // Measurement count
            let measure_count = self.simulation_measurements.len();
            if measure_count > 0 {
                property_row(ui, mode, "Measures", &format!("{} defined", measure_count));
            }

            ui.add_space(6.0);
            if compact_action(ui, mode, self.text(UiText::RunSimulation)) {
                self.run_simulation_from_panel();
            }
            if compact_action(ui, mode, self.text(UiText::SimulationWorkspace)) {
                self.open_simulation_workspace();
            }
        });

        ui.add_space(8.0);
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::LiveMeasurements),
            ));
            draw_measurement_rows(self, ui);
            if let Some(error) = &self.simulation_panel.last_error {
                ui.colored_label(severity_color(mode, KicadDiagnosticSeverity::Error), error);
            }
            if let Some(run) = &self.simulation_panel.last_run {
                property_row(
                    ui,
                    mode,
                    self.text(UiText::LastRun),
                    &format!("{} ms", run.metadata.duration_ms),
                );
                let history_count = self.simulation_history.len();
                if history_count > 0 {
                    property_row(ui, mode, "History", &format!("{} runs", history_count));
                }
            }
        });

        ui.add_space(8.0);
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::SchematicTools),
            ));
            self.draw_schematic_tool_controls(ui);
        });
    }

    fn open_simulation_workspace(&mut self) {
        self.active_workspace = crate::app::navigation::StudioWorkspace::Simulation;
    }
}

fn draw_measurement_rows(app: &NekoSpiceApp, ui: &mut egui::Ui) {
    let mode = app.theme_mode();
    if let Some(run) = &app.simulation_panel.last_run {
        if let crate::waveform_summary::GuiWaveformSummaryState::Ready(summary) = &run.waveform {
            for var in summary.variables.iter().take(6) {
                property_row(
                    ui,
                    mode,
                    &var.name,
                    &format!("{}: {} ({:.3})", var.unit, var.name, var.max),
                );
            }
            if summary.variables.is_empty() {
                property_row(ui, mode, app.text(UiText::LiveMeasurements), "No signals");
            }
        } else {
            property_row(ui, mode, app.text(UiText::LiveMeasurements), "Processing...");
        }
    } else {
        property_row(ui, mode, "V(IN)", "--");
        property_row(ui, mode, "V(OUT)", "--");
        property_row(ui, mode, "I(R1)", "--");
        property_row(ui, mode, app.text(UiText::TemperatureSweep), "27 C");
    }
}
