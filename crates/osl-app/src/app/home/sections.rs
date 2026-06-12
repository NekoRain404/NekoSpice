use crate::app::NekoSpiceApp;
use super::widgets::{
    measurement_row, project_row, queue_row, recommendation_row, section_header, section_header_clickable, template_card,
};
use crate::app::localization::UiText;
use crate::app::navigation::StudioWorkspace;
use crate::app::theme::StudioTheme;
use crate::app::widgets::metric_row;
use eframe::egui::{self, Vec2};
use osl_core::RunStatus;

impl NekoSpiceApp {
    /// draw recent projects panel。
    pub(crate) fn draw_recent_projects_panel(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            if section_header_clickable(
                ui,
                mode,
                self.text(UiText::RecentProjects),
                self.text(UiText::ViewAll),
            ) {
                self.active_workspace = StudioWorkspace::Schematic;
            }
            // 当前加载的原理图
            let snapshot = self.studio_status_snapshot();
            project_row(
                ui,
                mode,
                &snapshot.project_name,
                &snapshot.source_path,
                &snapshot.document_state,
            );
            // 符号库
            if !self.library_table_path.is_empty() {
                let lib_name = std::path::Path::new(&self.library_table_path)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| self.library_table_path.clone());
                let status = if self.library.is_some() {
                    self.text(UiText::Ready)
                } else {
                    self.text(UiText::Missing)
                };
                project_row(
                    ui,
                    mode,
                    &lib_name,
                    &self.library_table_path,
                    status,
                );
            }
        });
    }

    /// draw quick actions panel。
    pub(crate) fn draw_quick_actions_panel(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::QuickActions),
            ));
            ui.add_space(4.0);
            self.draw_quick_action_grid(ui);
        });
    }

    /// draw template row。
    pub(crate) fn draw_template_row(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        if section_header_clickable(
            ui,
            mode,
            self.text(UiText::StartTemplate),
            self.text(UiText::ViewAll),
        ) {
            self.active_workspace = StudioWorkspace::Schematic;
        }
        ui.add_space(4.0);
        let spacing = 10.0;
        let available_width = ui.available_width();
        let columns: usize = if available_width >= 850.0 {
            5
        } else if available_width >= 620.0 {
            3
        } else if available_width >= 390.0 {
            2
        } else {
            1
        };
        let card_width = ((available_width - spacing * (columns.saturating_sub(1) as f32))
            / columns as f32)
            .max(150.0);

        egui::Grid::new("home_template_grid")
            .num_columns(columns)
            .spacing(Vec2::new(spacing, spacing))
            .show(ui, |ui| {
                for (index, template) in home_templates().into_iter().enumerate() {
                    ui.allocate_ui_with_layout(
                        Vec2::new(card_width, 126.0),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            template_card(
                                ui,
                                mode,
                                self.text(template.name),
                                template.caption,
                                self.text(UiText::Use),
                            );
                        },
                    );
                    if (index + 1) % columns == 0 {
                        ui.end_row();
                    }
                }
            });
    }

    /// draw simulation queue panel。
    pub(crate) fn draw_simulation_queue_panel(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            section_header(
                ui,
                mode,
                self.text(UiText::SimulationQueue),
                self.text(UiText::ViewAll),
            );
            if self.simulation_panel.active_task.is_some() {
                queue_row(
                    ui,
                    mode,
                    "1",
                    self.simulation_panel.backend.label(),
                    &self.schematic_path,
                    self.text(UiText::Running),
                );
            } else if let Some(run) = &self.simulation_panel.last_run {
                queue_row(
                    ui,
                    mode,
                    "1",
                    run.metadata.backend.as_str(),
                    &run.output_dir.display().to_string(),
                    run.metadata.status.as_str(),
                );
            } else {
                queue_row(
                    ui,
                    mode,
                    "1",
                    "Transient",
                    self.text(UiText::NoRecentRun),
                    self.text(UiText::Queued),
                );
            }
        });
    }

    /// draw solver health panel。
    pub(crate) fn draw_solver_health_panel(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            section_header(
                ui,
                mode,
                self.text(UiText::SolverHealth),
                self.text(UiText::Diagnostics),
            );
            let status = if self.simulation_panel.active_task.is_some() {
                self.text(UiText::Running)
            } else {
                self.text(UiText::HealthReady)
            };
            metric_row(ui, mode, self.simulation_panel.backend.label(), status);
            let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
            metric_row(ui, mode, self.text(UiText::Threads), &format!("{threads} threads"));
            metric_row(ui, mode, self.text(UiText::Renderer), "wgpu");
            metric_row(ui, mode, self.text(UiText::Backend), "CLI");
            ui.separator();
            ui.colored_label(
                StudioTheme::palette(mode).success,
                self.text(UiText::SystemOperational),
            );
        });
    }

    /// draw recent measurements panel。
    pub(crate) fn draw_recent_measurements_panel(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            section_header(
                ui,
                mode,
                self.text(UiText::RecentMeasurements),
                self.text(UiText::ViewAll),
            );
            if let Some(run) = &self.simulation_panel.last_run {
                let status = match run.metadata.status {
                    RunStatus::Passed => self.text(UiText::Saved),
                    RunStatus::Failed => self.text(UiText::WaveformError),
                };
                measurement_row(ui, mode, "Run", run.metadata.backend.as_str(), status);
                measurement_row(ui, mode, "Duration", "time", &format!("{} ms", run.metadata.duration_ms));
                // Show waveform signals if available
                if let crate::waveform_summary::GuiWaveformSummaryState::Ready(summary) = &run.waveform {
                    for var in summary.variables.iter().take(4) {
                        measurement_row(ui, mode, &var.name, &var.unit, &format!("max={:.3}", var.max));
                    }
                }
            } else {
                ui.label(StudioTheme::muted_for(mode, "Run a simulation to see measurements"));
            }
        });
    }

    /// draw recommendations panel。
    pub(crate) fn draw_recommendations_panel(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            section_header(
                ui,
                mode,
                self.text(UiText::RecommendedForYou),
                self.text(UiText::ViewAll),
            );
            // Context-aware recommendations based on current state
            if self.document.is_none() {
                recommendation_row(ui, mode, "Open a schematic to get started", self.text(UiText::Open));
            } else {
                let has_run = self.simulation_panel.last_run.is_some();
                if !has_run {
                    recommendation_row(ui, mode, "Run a simulation to validate your design", self.text(UiText::Run));
                }
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

    fn draw_quick_action_grid(&mut self, ui: &mut egui::Ui) {
        const ACTIONS: [(&str, UiText, Option<StudioWorkspace>); 9] = [
            ("+", UiText::NewProject, None),
            ("[]", UiText::OpenProject, Some(StudioWorkspace::Schematic)),
            ("Ki", UiText::ImportKiCad, Some(StudioWorkspace::Schematic)),
            ("SC", UiText::NewSchematic, Some(StudioWorkspace::Schematic)),
            (
                ">",
                UiText::RunSimulation,
                Some(StudioWorkspace::Simulation),
            ),
            ("~", UiText::WaveformViewer, Some(StudioWorkspace::Reports)),
            (
                "SW",
                UiText::ParametricSweep,
                Some(StudioWorkspace::Simulation),
            ),
            ("MC", UiText::MonteCarlo, Some(StudioWorkspace::Simulation)),
            (
                "OP",
                UiText::Optimization,
                Some(StudioWorkspace::Simulation),
            ),
        ];

        let spacing = 8.0;
        let available_width = ui.available_width();
        let columns: usize = if available_width >= 620.0 {
            3
        } else if available_width >= 360.0 {
            2
        } else {
            1
        };
        let button_width = ((available_width - spacing * (columns.saturating_sub(1) as f32))
            / columns as f32)
            .max(120.0);

        egui::Grid::new("home_quick_actions")
            .num_columns(columns)
            .spacing(Vec2::new(spacing, spacing))
            .show(ui, |ui| {
                for (index, (icon, text, route)) in ACTIONS.into_iter().enumerate() {
                    self.quick_action(ui, button_width, icon, text, route);
                    if (index + 1) % columns == 0 {
                        ui.end_row();
                    }
                }
            });
    }

    fn quick_action(
        &mut self,
        ui: &mut egui::Ui,
        width: f32,
        icon: &'static str,
        text: UiText,
        route: Option<StudioWorkspace>,
    ) {
        let label = format!("{}  {}", icon, self.text(text));
        let response = ui.add_sized([width, 38.0], egui::Button::new(label));
        if response.clicked() {
            match text {
                UiText::RunSimulation => {
                    self.run_simulation_from_panel();
                    self.active_workspace = StudioWorkspace::Simulation;
                }
                UiText::OpenProject | UiText::ImportKiCad => {
                    self.open_file_dialog();
                }
                UiText::NewSchematic => {
                    self.status_message = Some("New schematic (use File > Open to load)".to_string());
                }
                UiText::NewProject => {
                    self.status_message = Some("New project (use File > Open to load)".to_string());
                }
                UiText::WaveformViewer => {
                    if self.simulation_panel.last_run.is_some() {
                        self.active_workspace = StudioWorkspace::Waveforms;
                    } else {
                        self.status_message = Some("Run a simulation first to view waveforms".to_string());
                    }
                }
                _ => {
                    if let Some(workspace) = route {
                        self.active_workspace = workspace;
                    }
                    self.status_message = Some(self.text(text).to_string());
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct HomeTemplate {
    name: UiText,
    caption: &'static str,
}

fn home_templates() -> [HomeTemplate; 5] {
    [
        HomeTemplate {
            name: UiText::TemplateOpAmp,
            caption: "Single / Dual OpAmp",
        },
        HomeTemplate {
            name: UiText::TemplateDcDc,
            caption: "Buck / Boost",
        },
        HomeTemplate {
            name: UiText::TemplateLdo,
            caption: "Low Dropout",
        },
        HomeTemplate {
            name: UiText::TemplateDifferentialPair,
            caption: "Analog Front End",
        },
        HomeTemplate {
            name: UiText::TemplatePowerSupply,
            caption: "SMPS / Flyback",
        },
    ]
}
