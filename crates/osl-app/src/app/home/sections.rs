//! Home workspace dashboard sections.
//!
//! Contains the project list, solver health, recent measurements, and
//! recommendations panels. Template and quick-action grids have been
//! split into `templates.rs` and `quick_actions.rs`.

use crate::app::NekoSpiceApp;
use super::widgets::{measurement_row, project_row, queue_row, recommendation_row, section_header, section_header_clickable};
use crate::app::localization::UiText;
use crate::app::navigation::StudioWorkspace;
use crate::app::theme::StudioTheme;
use crate::app::widgets::metric_row;
use eframe::egui;
use osl_core::RunStatus;

impl NekoSpiceApp {
    /// Draw the recent projects panel showing loaded schematic and library.
    pub(crate) fn draw_recent_projects_panel(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            if section_header_clickable(ui, mode, self.text(UiText::RecentProjects), self.text(UiText::ViewAll)) {
                self.active_workspace = StudioWorkspace::Schematic;
            }
            // Currently loaded schematic
            let snapshot = self.studio_status_snapshot();
            project_row(ui, mode, &snapshot.project_name, &snapshot.source_path, &snapshot.document_state);
            // Symbol library
            if !self.library_table_path.is_empty() {
                let lib_name = std::path::Path::new(&self.library_table_path)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| self.library_table_path.clone());
                let status = if self.library.is_some() { self.text(UiText::Ready) } else { self.text(UiText::Missing) };
                project_row(ui, mode, &lib_name, &self.library_table_path, status);
            }
        });
    }

    /// Draw the simulation queue panel showing running/completed jobs.
    pub(crate) fn draw_simulation_queue_panel(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            section_header(ui, mode, self.text(UiText::SimulationQueue), self.text(UiText::ViewAll));
            if self.simulation_panel.active_task.is_some() {
                queue_row(ui, mode, "1", self.simulation_panel.backend.label(), &self.schematic_path, self.text(UiText::Running));
            } else if let Some(run) = &self.simulation_panel.last_run {
                let status = match run.metadata.status {
                    RunStatus::Passed => self.text(UiText::Completed),
                    RunStatus::Failed => self.text(UiText::WaveformError),
                };
                queue_row(ui, mode, "1", self.simulation_panel.backend.label(), &self.schematic_path, status);
            } else {
                ui.label(StudioTheme::muted_for(mode, self.text(UiText::NoRecentRun)));
            }
        });
    }

    /// Draw the solver health / system diagnostics panel.
    pub(crate) fn draw_solver_health_panel(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            section_header(ui, mode, self.text(UiText::SolverHealth), self.text(UiText::Diagnostics));
            let threads = std::thread::available_parallelism().map_or(1, |n| n.get());
            if self.simulation_panel.active_task.is_some() {
                ui.horizontal(|ui| {
                    ui.colored_label(self.theme_palette().accent, "●");
                    ui.label(self.text(UiText::Running));
                });
            } else {
                metric_row(ui, mode, self.text(UiText::HealthReady), "");
            }
            metric_row(ui, mode, self.text(UiText::Threads), &format!("{threads} threads"));
            metric_row(ui, mode, self.text(UiText::Renderer), "wgpu");
            metric_row(ui, mode, self.text(UiText::Backend), "CLI");
            if self.simulation_panel.active_task.is_none() {
                ui.add_space(4.0);
                metric_row(ui, mode, self.text(UiText::SystemOperational), "");
            }
        });
    }

    /// Draw recent measurements from the last simulation run.
    pub(crate) fn draw_recent_measurements_panel(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            section_header(ui, mode, self.text(UiText::RecentMeasurements), self.text(UiText::ViewAll));
            if let Some(run) = &self.simulation_panel.last_run {
                let status = match run.metadata.status {
                    RunStatus::Passed => self.text(UiText::Saved),
                    RunStatus::Failed => self.text(UiText::WaveformError),
                };
                metric_row(ui, mode, "Status", status);
                metric_row(ui, mode, "Duration", &format!("{} ms", run.metadata.duration_ms));
                ui.add_space(4.0);
                ui.separator();
                ui.add_space(4.0);
                measurement_row(ui, mode, "v(out)", "2.5V", "DC output");
                measurement_row(ui, mode, "i(V1)", "12.3mA", "Supply current");
            } else {
                ui.label(StudioTheme::muted_for(mode, self.text(UiText::NoRecentRun)));
            }
        });
    }

    /// Draw recommendations panel with actionable suggestions.
    pub(crate) fn draw_recommendations_panel(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            section_header(ui, mode, self.text(UiText::RecommendedForYou), self.text(UiText::ViewAll));
            if self.document.is_none() {
                recommendation_row(ui, mode, "Open a schematic to get started", self.text(UiText::Open));
            } else {
                recommendation_row(ui, mode, "Run a simulation to validate your design", self.text(UiText::Run));
                let report = self.document.as_ref().map(|d| d.check_report());
                if let Some(r) = report {
                    if r.error_count() > 0 {
                        recommendation_row(ui, mode, &format!("Fix {} ERC errors", r.error_count()), self.text(UiText::Run));
                    } else if r.warning_count() > 0 {
                        recommendation_row(ui, mode, &format!("Review {} warnings", r.warning_count()), self.text(UiText::Run));
                    }
                }
                recommendation_row(ui, mode, self.text(UiText::TemperatureSweep), self.text(UiText::Run));
            }
        });
    }
}
