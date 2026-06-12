use super::NekoSpiceApp;
use super::home_widgets::{
    measurement_row, project_row, queue_row, recommendation_row, section_header, section_header_clickable, template_card,
};
use super::localization::UiText;
use super::navigation::StudioWorkspace;
use super::theme::StudioTheme;
use super::widgets::metric_row;
use eframe::egui::{self, Vec2};
use osl_core::RunStatus;

impl NekoSpiceApp {
    pub(super) fn draw_recent_projects_panel(&mut self, ui: &mut egui::Ui) {
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
            let snapshot = self.studio_status_snapshot();
            project_row(
                ui,
                mode,
                &snapshot.project_name,
                &snapshot.source_path,
                self.text(UiText::Ready),
            );
            project_row(
                ui,
                mode,
                "KiCad schematic import",
                "examples/kicad_schematic/rc.kicad_sch",
                self.text(UiText::Saved),
            );
            project_row(
                ui,
                mode,
                "Symbol library bridge",
                "examples/kicad_schematic/sym-lib-table",
                self.text(UiText::Ready),
            );
        });
    }

    pub(super) fn draw_quick_actions_panel(&mut self, ui: &mut egui::Ui) {
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

    pub(super) fn draw_template_row(&mut self, ui: &mut egui::Ui) {
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

    pub(super) fn draw_simulation_queue_panel(&mut self, ui: &mut egui::Ui) {
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
                    "ngspice",
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

    pub(super) fn draw_solver_health_panel(&mut self, ui: &mut egui::Ui) {
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
            metric_row(ui, mode, "ngspice", status);
            metric_row(ui, mode, self.text(UiText::Threads), "8 / 16");
            metric_row(ui, mode, self.text(UiText::Renderer), "wgpu");
            metric_row(ui, mode, self.text(UiText::Backend), "CLI");
            ui.separator();
            ui.colored_label(
                StudioTheme::palette(mode).success,
                self.text(UiText::SystemOperational),
            );
        });
    }

    pub(super) fn draw_recent_measurements_panel(&mut self, ui: &mut egui::Ui) {
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
            } else {
                measurement_row(ui, mode, "DC Gain", "Av(1)", "82.45 dB");
                measurement_row(ui, mode, self.text(UiText::Average), "v(out)", "1.64 V");
                measurement_row(ui, mode, "Unity Gain Freq.", "fu", "3.21 MHz");
                measurement_row(ui, mode, "Phase Margin", "PM", "68.2 deg");
                measurement_row(ui, mode, "THD @ 1 kHz", "THD", "-92.3 dB");
            }
        });
    }

    pub(super) fn draw_recommendations_panel(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            section_header(
                ui,
                mode,
                self.text(UiText::RecommendedForYou),
                self.text(UiText::ViewAll),
            );
            recommendation_row(
                ui,
                mode,
                self.text(UiText::VerifyStability),
                self.text(UiText::Run),
            );
            recommendation_row(
                ui,
                mode,
                self.text(UiText::LoopStability),
                self.text(UiText::Run),
            );
            recommendation_row(
                ui,
                mode,
                self.text(UiText::TemperatureSweep),
                self.text(UiText::Run),
            );
            recommendation_row(
                ui,
                mode,
                self.text(UiText::ModelUpdateAvailable),
                self.text(UiText::Update),
            );
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
