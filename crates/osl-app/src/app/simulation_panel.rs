use super::NekoSpiceApp;
use crate::simulation::{GuiSimulationRun, GuiSimulationTask};
use crate::waveform_summary::{GuiWaveformSummary, GuiWaveformSummaryState};
use eframe::egui::{self, Color32};
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
        }
    }
}

impl NekoSpiceApp {
    pub(super) fn draw_simulation_panel(&mut self, ui: &mut egui::Ui) {
        self.poll_simulation_task();
        ui.heading("Simulation");
        self.draw_simulation_directive_editor(ui);

        let is_running = self.simulation_panel.active_task.is_some();
        if is_running {
            ui.ctx().request_repaint_after(Duration::from_millis(100));
        }
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    self.document.is_some() && !is_running,
                    egui::Button::new("Run ngspice"),
                )
                .clicked()
            {
                self.run_simulation_from_panel();
            }
            if is_running {
                ui.label("Running");
            }
        });
        self.draw_simulation_run_status(ui);

        let Some(document) = &self.document else {
            ui.label("No editable schematic loaded");
            return;
        };

        ui.separator();
        draw_simulation_directives(ui, &document.simulation_directives());

        ui.separator();
        let report = document.check_report();
        ui.horizontal(|ui| {
            ui.label(format!(
                "{} diagnostics",
                report.error_count() + report.warning_count() + report.info_count()
            ));
            ui.colored_label(
                severity_color(KicadDiagnosticSeverity::Error),
                format!("{} errors", report.error_count()),
            );
            ui.colored_label(
                severity_color(KicadDiagnosticSeverity::Warning),
                format!("{} warnings", report.warning_count()),
            );
        });
        egui::ScrollArea::vertical()
            .id_salt("simulation_diagnostics")
            .max_height(150.0)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if report.diagnostics.is_empty() {
                    ui.label("No diagnostics");
                }
                for diagnostic in &report.diagnostics {
                    ui.colored_label(
                        severity_color(diagnostic.severity),
                        format!("{}: {}", diagnostic.code, diagnostic.message),
                    );
                }
            });

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
                    ui.colored_label(severity_color(KicadDiagnosticSeverity::Error), error);
                }
            }
        }
    }

    fn draw_simulation_directive_editor(&mut self, ui: &mut egui::Ui) {
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
            ui.label("Body");
            ui.text_edit_singleline(&mut self.simulation_panel.directive_body);
        });
        if ui
            .add_enabled(self.document.is_some(), egui::Button::new("Set Directive"))
            .clicked()
        {
            self.apply_simulation_directive_edit();
        }
    }

    fn apply_simulation_directive_edit(&mut self) {
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

    fn run_simulation_from_panel(&mut self) {
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

    fn poll_simulation_task(&mut self) {
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
                self.simulation_panel.last_run = Some(run);
            }
            Err(error) => {
                self.status_message = Some(error.clone());
                self.simulation_panel.last_run = None;
                self.simulation_panel.last_error = Some(error);
            }
        }
    }

    fn draw_simulation_run_status(&mut self, ui: &mut egui::Ui) {
        if self.simulation_panel.active_task.is_some() {
            ui.label("Background simulation is running");
        }
        if let Some(error) = &self.simulation_panel.last_error {
            ui.colored_label(severity_color(KicadDiagnosticSeverity::Error), error);
        }
        if let Some(run) = &self.simulation_panel.last_run {
            let color = match run.metadata.status {
                RunStatus::Passed => Color32::from_rgb(40, 140, 80),
                RunStatus::Failed => severity_color(KicadDiagnosticSeverity::Error),
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
            draw_waveform_summary(ui, &run.waveform);
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

fn draw_waveform_summary(ui: &mut egui::Ui, waveform: &GuiWaveformSummaryState) {
    match waveform {
        GuiWaveformSummaryState::Ready(summary) => draw_ready_waveform_summary(ui, summary),
        GuiWaveformSummaryState::Missing { raw_path } => {
            ui.label("Waveform: no waveform.raw");
            ui.monospace(raw_path.display().to_string());
        }
        GuiWaveformSummaryState::Error { raw_path, message } => {
            ui.colored_label(
                severity_color(KicadDiagnosticSeverity::Warning),
                format!("Waveform parse failed: {message}"),
            );
            ui.monospace(raw_path.display().to_string());
        }
    }
}

fn draw_ready_waveform_summary(ui: &mut egui::Ui, summary: &GuiWaveformSummary) {
    ui.separator();
    ui.label(format!(
        "Waveform: {} points, {} variables",
        summary.point_count, summary.variable_count
    ));
    if !summary.plot_name.is_empty() {
        ui.label(format!("Plot: {}", summary.plot_name));
    }
    if !summary.title.is_empty() {
        ui.label(format!("Title: {}", summary.title));
    }
    ui.monospace(summary.raw_path.display().to_string());

    egui::Grid::new("simulation_waveform_summary")
        .num_columns(5)
        .spacing(egui::Vec2::new(8.0, 2.0))
        .striped(true)
        .show(ui, |ui| {
            ui.strong("Signal");
            ui.strong("Last");
            ui.strong("Min");
            ui.strong("Max");
            ui.strong("P-P");
            ui.end_row();
            for variable in &summary.variables {
                ui.label(variable_label(
                    &variable.name,
                    &variable.unit,
                    variable.samples,
                ))
                .on_hover_text(variable_hover_text(variable.first, variable.rms));
                ui.monospace(format_compact_f64(variable.last));
                ui.monospace(format_compact_f64(variable.min));
                ui.monospace(format_compact_f64(variable.max));
                ui.monospace(format_compact_f64(variable.peak_to_peak));
                ui.end_row();
            }
        });
    if summary.omitted_variable_count > 0 {
        ui.label(format!("{} more variables", summary.omitted_variable_count));
    }
}

fn variable_hover_text(first: f64, rms: f64) -> String {
    format!(
        "First: {}\nRMS: {}",
        format_compact_f64(first),
        format_compact_f64(rms)
    )
}

fn variable_label(name: &str, unit: &str, samples: usize) -> String {
    if unit.is_empty() {
        format!("{name} ({samples})")
    } else {
        format!("{name} [{unit}] ({samples})")
    }
}

fn format_compact_f64(value: f64) -> String {
    if !value.is_finite() {
        return value.to_string();
    }
    let absolute = value.abs();
    if value == 0.0 {
        "0".to_string()
    } else if !(1.0e-3..1.0e4).contains(&absolute) {
        format!("{value:.3e}")
    } else {
        format!("{value:.4}")
    }
}

fn severity_color(severity: KicadDiagnosticSeverity) -> Color32 {
    match severity {
        KicadDiagnosticSeverity::Info => Color32::from_rgb(80, 120, 170),
        KicadDiagnosticSeverity::Warning => Color32::from_rgb(180, 120, 20),
        KicadDiagnosticSeverity::Error => Color32::from_rgb(190, 40, 40),
    }
}
