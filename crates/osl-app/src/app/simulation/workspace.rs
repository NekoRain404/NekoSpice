//! Simulation workspace — center panel for the simulation workspace.
//!
//! The simulation workspace provides two sub-views accessible via tabs:
//!
//! 1. **Overview** — High-level solver metrics (engine, status, netlist directives,
//!    last run duration), analysis mode setup (.tran/.ac/.dc/.op), netlist preview,
//!    and run output with diagnostics.
//!
//! 2. **Profile Editor** — Three-column layout for detailed parameter editing:
//!    - Left: Analysis setup, component parameters, model parameters
//!    - Center: Parameter definitions editor
//!    - Right: Simulation options, tolerances, run status, recent runs

use super::profile_editor::SimulationSubView;
use super::workspace_widgets::solver_metric_card;
use crate::app::NekoSpiceApp;
use crate::app::localization::StudioLocale;
use crate::app::localization::UiText;
use crate::app::theme::StudioTheme;
use eframe::egui;
use osl_core::RunStatus;

impl NekoSpiceApp {
    /// Main entry point for drawing the simulation center workspace.
    pub(crate) fn draw_simulation_center_workspace(&mut self, ui: &mut egui::Ui) {
        self.poll_simulation_task();
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            self.draw_simulation_workspace_header(ui);
            ui.add_space(8.0);
            self.draw_simulation_sub_view_tabs(ui);
            ui.add_space(8.0);
            let sub_view = self.simulation_profile_editor.sub_view;
            match sub_view {
                SimulationSubView::Overview => self.draw_simulation_overview(ui),
                SimulationSubView::ProfileEditor => self.draw_profile_editor(ui),
            }
        });
    }

    /// Tab bar for switching between simulation workspace sub-views.
    fn draw_simulation_sub_view_tabs(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let palette = StudioTheme::palette(mode);
        ui.horizontal(|ui| {
            for view in [
                SimulationSubView::Overview,
                SimulationSubView::ProfileEditor,
            ] {
                let active = self.simulation_profile_editor.sub_view == view;
                let label = match self.locale() {
                    StudioLocale::SimplifiedChinese => view.label_zh(),
                    _ => view.label(),
                };
                let btn = if active {
                    egui::Button::new(egui::RichText::new(label).strong().color(palette.text))
                        .fill(palette.accent_soft)
                        .stroke(egui::Stroke::new(1.0, palette.accent))
                } else {
                    egui::Button::new(egui::RichText::new(label).color(palette.text_muted))
                        .fill(palette.panel_soft)
                        .stroke(egui::Stroke::new(1.0, palette.border))
                };
                if ui.add(btn).clicked() {
                    self.simulation_profile_editor.sub_view = view;
                }
            }
        });
    }

    /// Overview sub-view: solver metrics + analysis + config summary + netlist + run output + export.
    fn draw_simulation_overview(&mut self, ui: &mut egui::Ui) {
        self.draw_simulation_solver_metrics(ui);
        ui.add_space(8.0);
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width((ui.available_width() * 0.34).max(260.0));
                self.draw_simulation_analysis_setup(ui);
                ui.add_space(8.0);
                self.draw_simulation_netlist_preview(ui);
            });
            ui.add_space(8.0);
            ui.vertical(|ui| {
                ui.set_width((ui.available_width() * 0.48).max(240.0));
                self.draw_simulation_profile_summary(ui);
                ui.add_space(8.0);
                self.draw_export_options_panel(ui);
            });
            ui.add_space(8.0);
            ui.vertical(|ui| {
                self.draw_simulation_run_output(ui);
                ui.add_space(8.0);
                self.draw_document_diagnostics_panel(ui, 170.0);
            });
        });
    }

    /// Workspace header with title, analysis summary, and run controls.
    fn draw_simulation_workspace_header(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let palette = self.theme_palette();
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.heading(self.text(UiText::SimulationSolver));
                // Show current analysis as a subtitle
                let analysis_summary = format!(
                    "{} {}",
                    self.simulation_panel.directive_kind,
                    self.simulation_panel.analysis_params.to_body().trim()
                );
                ui.label(StudioTheme::muted_for(mode, analysis_summary.trim()));
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                let running = self.simulation_panel.active_task.is_some();
                if running {
                    if ui
                        .button("Stop")
                        .on_hover_text("Cancel running simulation")
                        .clicked()
                    {
                        self.simulation_panel.active_task = None;
                        self.simulation_panel.run_start_time = None;
                        self.status_message = Some("Simulation cancelled".to_string());
                    }
                    // Show running indicator with elapsed time
                    ui.label(StudioTheme::status_dot(palette.warning));
                    let elapsed = self
                        .simulation_panel
                        .run_start_time
                        .map(|t| t.elapsed().as_secs())
                        .unwrap_or(0);
                    ui.label(
                        egui::RichText::new(format!(
                            "{} ({}s)",
                            self.text(UiText::Running),
                            elapsed
                        ))
                        .color(palette.warning)
                        .strong(),
                    );
                } else if ui
                    .add_enabled(
                        self.document.is_some(),
                        egui::Button::new(self.text(UiText::RunSimulation)),
                    )
                    .clicked()
                {
                    self.run_simulation_from_panel();
                }
                ui.separator();
                // Backend engine selector
                egui::ComboBox::from_id_salt("sim_workspace_backend")
                    .selected_text(self.simulation_panel.backend.label())
                    .show_ui(ui, |ui| {
                        for &kind in &super::state::SimulationBackendKind::ALL {
                            let label = match self.locale() {
                                StudioLocale::SimplifiedChinese => kind.label_zh(),
                                _ => kind.label(),
                            };
                            ui.selectable_value(&mut self.simulation_panel.backend, kind, label);
                        }
                    });
            });
        });
    }

    /// Four metric cards showing solver engine, status, analysis type, and last run.
    fn draw_simulation_solver_metrics(&self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let status = self.simulation_status_label();
        let analysis_label = self.simulation_panel.directive_kind.to_string();
        ui.columns(4, |columns| {
            solver_metric_card(
                &mut columns[0],
                mode,
                self.text(UiText::SolverEngine),
                self.simulation_panel.backend.label(),
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
                "Analysis",
                &analysis_label,
                match self.simulation_panel.directive_kind {
                    osl_kicad::KicadSimulationDirectiveKind::Tran => "time domain",
                    osl_kicad::KicadSimulationDirectiveKind::Ac => "small signal",
                    osl_kicad::KicadSimulationDirectiveKind::Dc => "sweep",
                    osl_kicad::KicadSimulationDirectiveKind::Op => "operating point",
                    osl_kicad::KicadSimulationDirectiveKind::Noise => "noise analysis",
                    _ => "",
                },
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
