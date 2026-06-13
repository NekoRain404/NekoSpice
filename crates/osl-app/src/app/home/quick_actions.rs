//! Home workspace quick action grid.
//!
//! Provides the 3x3 action button grid on the home dashboard.
//! Each button routes to a workspace or triggers a direct action.

use crate::app::NekoSpiceApp;
use crate::app::localization::UiText;
use crate::app::navigation::StudioWorkspace;
use crate::app::theme::StudioTheme;
use eframe::egui::{self, Vec2};

/// Quick action definition: (icon, label text key, optional workspace route).
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

impl NekoSpiceApp {
    /// Draw the quick actions panel with a responsive button grid.
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

    /// Render the responsive grid of quick action buttons.
    fn draw_quick_action_grid(&mut self, ui: &mut egui::Ui) {
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

    /// Handle a single quick action button click.
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
                    self.create_new_schematic("Untitled Schematic");
                }
                UiText::NewProject => {
                    self.create_new_schematic("New Project");
                }
                UiText::WaveformViewer => {
                    if self.simulation_panel.last_run.is_some() {
                        self.active_workspace = StudioWorkspace::Waveforms;
                    } else {
                        self.status_message =
                            Some("Run a simulation first to view waveforms".to_string());
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
