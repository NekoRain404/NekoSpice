use super::NekoSpiceApp;
use super::localization::UiText;
use super::simulation_workspace_widgets::solver_metric_card;
use super::theme::StudioTheme;
use eframe::egui;
use osl_core::RunStatus;

impl NekoSpiceApp {
    pub(super) fn draw_simulation_center_workspace(&mut self, ui: &mut egui::Ui) {
        self.poll_simulation_task();
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            self.draw_simulation_workspace_header(ui);
            ui.add_space(8.0);
            self.draw_simulation_solver_metrics(ui);
            ui.add_space(8.0);
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_width((ui.available_width() * 0.48).max(300.0));
                    self.draw_simulation_analysis_setup(ui);
                    ui.add_space(8.0);
                    self.draw_simulation_netlist_preview(ui);
                });
                ui.add_space(10.0);
                ui.vertical(|ui| {
                    self.draw_simulation_run_output(ui);
                    ui.add_space(8.0);
                    self.draw_document_diagnostics_panel(ui, 170.0);
                });
            });
        });
    }

    fn draw_simulation_workspace_header(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.heading(self.text(UiText::SimulationSolver));
                ui.label(StudioTheme::muted_for(
                    mode,
                    self.text(UiText::SimulationSolverCaption),
                ));
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                let running = self.simulation_panel.active_task.is_some();
                if ui
                    .add_enabled(
                        self.document.is_some() && !running,
                        egui::Button::new(self.text(UiText::RunSimulation)),
                    )
                    .clicked()
                {
                    self.run_simulation_from_panel();
                }
            });
        });
    }

    fn draw_simulation_solver_metrics(&self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let status = self.simulation_status_label();
        ui.columns(4, |columns| {
            solver_metric_card(
                &mut columns[0],
                mode,
                self.text(UiText::SolverEngine),
                "ngspice",
                "CLI",
            );
            solver_metric_card(
                &mut columns[1],
                mode,
                self.text(UiText::StatusConsole),
                status,
                self.text(UiText::Backend),
            );
            solver_metric_card(
                &mut columns[2],
                mode,
                self.text(UiText::Netlist),
                self.document
                    .as_ref()
                    .map(|document| document.simulation_directives().len().to_string())
                    .unwrap_or_else(|| "0".to_string())
                    .as_str(),
                "directives",
            );
            solver_metric_card(
                &mut columns[3],
                mode,
                self.text(UiText::LastRun),
                self.last_run_duration_label().as_str(),
                self.text(UiText::RunOutput),
            );
        });
    }

    fn simulation_status_label(&self) -> &'static str {
        if self.simulation_panel.active_task.is_some() {
            return self.text(UiText::Running);
        }
        match self
            .simulation_panel
            .last_run
            .as_ref()
            .map(|run| run.metadata.status)
        {
            Some(RunStatus::Passed) => self.text(UiText::Ready),
            Some(RunStatus::Failed) => self.text(UiText::WaveformError),
            None => self.text(UiText::Queued),
        }
    }

    fn last_run_duration_label(&self) -> String {
        self.simulation_panel
            .last_run
            .as_ref()
            .map(|run| format!("{} ms", run.metadata.duration_ms))
            .unwrap_or_else(|| self.text(UiText::NoRecentRun).to_string())
    }
}
