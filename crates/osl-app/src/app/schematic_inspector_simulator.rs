use super::NekoSpiceApp;
use super::localization::UiText;
use super::schematic_inspector_widgets::{compact_action, property_row, status_pill};
use super::status_strip::severity_color;
use super::theme::StudioTheme;
use eframe::egui;
use osl_kicad::KicadDiagnosticSeverity;

impl NekoSpiceApp {
    pub(super) fn draw_schematic_simulator_tab(&mut self, ui: &mut egui::Ui) {
        self.poll_simulation_task();
        let mode = self.theme_mode();
        let palette = self.theme_palette();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::Simulator),
            ));
            ui.horizontal_wrapped(|ui| {
                status_pill(ui, mode, "ngspice", palette.success);
                let status = if self.simulation_panel.active_task.is_some() {
                    self.text(UiText::Running)
                } else {
                    self.text(UiText::Ready)
                };
                status_pill(ui, mode, status, palette.accent);
            });
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
        self.active_workspace = super::navigation::StudioWorkspace::Simulation;
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
