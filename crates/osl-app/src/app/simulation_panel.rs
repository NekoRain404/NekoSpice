use super::NekoSpiceApp;
use super::localization::UiText;
use super::simulation_artifacts_panel::draw_simulation_artifacts_panel;
use super::simulation_report_panel::draw_simulation_report_panel;
use super::simulation_waveform_panel::draw_simulation_waveform_panel;
use super::status_strip::severity_color;
use crate::simulation::{GuiSimulationRun, GuiSimulationTask};
use crate::waveform_summary::GuiWaveformSummaryState;
use eframe::egui;
use osl_core::RunStatus;
use osl_kicad::{KicadDiagnosticSeverity, KicadSimulationDirective, KicadSimulationDirectiveKind};
use std::path::Path;
use std::time::Duration;

const NETLIST_PREVIEW_LINES: usize = 18;

#[derive(Debug)]
pub(crate) struct SimulationPanelState {
    pub(super) directive_kind: KicadSimulationDirectiveKind,
    pub(super) directive_body: String,
    pub(super) show_netlist: bool,
    pub(super) last_run: Option<GuiSimulationRun>,
    pub(super) last_error: Option<String>,
    pub(super) active_task: Option<GuiSimulationTask>,
    pub(super) selected_waveform_signal: Option<String>,
}

impl Default for SimulationPanelState {
    fn default() -> Self {
        Self {
            directive_kind: KicadSimulationDirectiveKind::Tran,
            directive_body: "1u 1m".to_string(),
            show_netlist: true,
            last_run: None,
            last_error: None,
            active_task: None,
            selected_waveform_signal: None,
        }
    }
}

impl NekoSpiceApp {
    pub(super) fn draw_simulation_panel(&mut self, ui: &mut egui::Ui) {
        self.poll_simulation_task();
        ui.heading(self.text(UiText::SimulationWorkspace));
        self.draw_simulation_directive_editor(ui);

        let is_running = self.simulation_panel.active_task.is_some();
        if is_running {
            ui.ctx().request_repaint_after(Duration::from_millis(100));
        }
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    self.document.is_some() && !is_running,
                    egui::Button::new(self.text(UiText::RunSimulation)),
                )
                .clicked()
            {
                self.run_simulation_from_panel();
            }
            if is_running {
                ui.label(self.text(UiText::Running));
            }
        });
        self.draw_simulation_run_status(ui);

        let Some(document) = &self.document else {
            ui.label(self.text(UiText::NoEditableSchematicLoaded));
            return;
        };

        ui.separator();
        draw_simulation_directives(ui, &document.simulation_directives());

        ui.separator();
        self.draw_document_diagnostics_panel(ui, 150.0);

        ui.separator();
        ui.checkbox(&mut self.simulation_panel.show_netlist, "Netlist preview");
        if self.simulation_panel.show_netlist {
            match document.spice_netlist_preview() {
                Ok(netlist) => {
                    egui::ScrollArea::vertical()
                        .id_salt("simulation_netlist_preview")
                        .max_height(220.0)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            for line in netlist.lines().take(NETLIST_PREVIEW_LINES) {
                                ui.monospace(line);
                            }
                            let hidden = netlist
                                .lines()
                                .count()
                                .saturating_sub(NETLIST_PREVIEW_LINES);
                            if hidden > 0 {
                                ui.label(format!("{hidden} more lines"));
                            }
                        });
                }
                Err(error) => {
                    ui.colored_label(
                        severity_color(self.theme_mode(), KicadDiagnosticSeverity::Error),
                        error,
                    );
                }
            }
        }
    }

    pub(in crate::app) fn draw_simulation_directive_editor(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            for (kind, label) in [
                (KicadSimulationDirectiveKind::Tran, ".tran"),
                (KicadSimulationDirectiveKind::Ac, ".ac"),
                (KicadSimulationDirectiveKind::Dc, ".dc"),
                (KicadSimulationDirectiveKind::Op, ".op"),
            ] {
                ui.selectable_value(&mut self.simulation_panel.directive_kind, kind, label);
            }
        });
        ui.horizontal(|ui| {
            ui.label(self.text(UiText::Body));
            ui.text_edit_singleline(&mut self.simulation_panel.directive_body);
        });
        if ui
            .add_enabled(
                self.document.is_some(),
                egui::Button::new(self.text(UiText::SetDirective)),
            )
            .clicked()
        {
            self.apply_simulation_directive_edit();
        }
    }

    pub(in crate::app) fn apply_simulation_directive_edit(&mut self) {
        let Some(document) = &mut self.document else {
            self.status_message = Some("No editable schematic loaded".to_string());
            return;
        };
        let kind = self.simulation_panel.directive_kind;
        let body = self.simulation_panel.directive_body.clone();
        match document.set_simulation_directive(kind, body, None) {
            Ok(summary) => {
                self.scene = Some(document.scene());
                self.load_error = None;
                self.status_message =
                    Some(format!("Edited {} {}", summary.operation, summary.target));
            }
            Err(error) => {
                self.status_message = Some(error);
            }
        }
    }

    pub(super) fn run_simulation_from_panel(&mut self) {
        let Some(document) = &self.document else {
            self.status_message = Some("No editable schematic loaded".to_string());
            return;
        };
        let runs_root = Path::new("runs").join("gui");
        match crate::simulation::GuiSimulationJob::from_document(document, &runs_root) {
            Ok(job) => {
                self.simulation_panel.last_run = None;
                self.simulation_panel.last_error = None;
                self.simulation_panel.active_task = Some(GuiSimulationTask::spawn_ngspice(job));
                self.status_message = Some("Simulation started".to_string());
            }
            Err(error) => {
                self.status_message = Some(error.clone());
                self.simulation_panel.last_error = Some(error);
            }
        }
    }

    pub(in crate::app) fn poll_simulation_task(&mut self) {
        let Some(task) = &self.simulation_panel.active_task else {
            return;
        };
        let Some(result) = task.try_finish() else {
            return;
        };
        self.simulation_panel.active_task = None;
        match result {
            Ok(run) => {
                self.status_message = Some(format!(
                    "Simulation {} in {} ms",
                    run.metadata.status.as_str(),
                    run.metadata.duration_ms
                ));
                self.simulation_panel.last_error = None;
                self.sync_selected_waveform_signal(&run.waveform);
                self.simulation_panel.last_run = Some(run);
            }
            Err(error) => {
                self.status_message = Some(error.clone());
                self.simulation_panel.last_run = None;
                self.simulation_panel.last_error = Some(error);
            }
        }
    }

    pub(in crate::app) fn draw_simulation_run_status(&mut self, ui: &mut egui::Ui) {
        if self.simulation_panel.active_task.is_some() {
            ui.label(self.text(UiText::Running));
        }
        if let Some(error) = &self.simulation_panel.last_error {
            ui.colored_label(
                severity_color(self.theme_mode(), KicadDiagnosticSeverity::Error),
                error,
            );
        }
        if let Some(run) = &self.simulation_panel.last_run {
            let color = match run.metadata.status {
                RunStatus::Passed => self.theme_palette().success,
                RunStatus::Failed => {
                    severity_color(self.theme_mode(), KicadDiagnosticSeverity::Error)
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
            draw_simulation_report_panel(ui, &run.report);
            draw_simulation_artifacts_panel(ui, run);
            draw_simulation_waveform_panel(
                ui,
                self.theme_mode(),
                &run.waveform,
                &mut self.simulation_panel.selected_waveform_signal,
            );
        }
    }

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

fn draw_simulation_directives(ui: &mut egui::Ui, directives: &[KicadSimulationDirective]) {
    ui.label(format!("{} directives", directives.len()));
    for directive in directives {
        ui.horizontal(|ui| {
            ui.monospace(directive.kind.to_string());
            ui.label(&directive.text);
        });
    }
}
