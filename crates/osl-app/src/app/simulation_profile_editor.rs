//! Simulation Profile Editor — three-column layout for editing analysis parameters,
//! component/model parameters, simulation options, and viewing run status.
//!
//! Layout:
//!   Left   — Analysis Setup + Component/Model parameters
//!   Center — Parameter definitions editor
//!   Right  — Simulation Options + Run Status + Recent runs

use super::NekoSpiceApp;
use super::localization::UiText;
use super::simulation_profile_editor_options::draw_profile_options;
use super::simulation_profile_editor_sections::{
    draw_analysis_setup_panel, draw_component_params, draw_model_params,
    draw_parameter_definitions,
};
use super::theme::StudioTheme;
use eframe::egui;

/// Sub-views available within the simulation workspace.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum SimulationSubView {
    /// High-level solver metrics, analysis setup, and netlist preview.
    #[default]
    Overview,
    /// Detailed profile editor with three-column parameter layout.
    ProfileEditor,
}

impl SimulationSubView {
    /// English label for the sub-view tab.
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Overview => "Overview",
            Self::ProfileEditor => "Profile Editor",
        }
    }

    /// Simplified Chinese label for the sub-view tab.
    pub(super) fn label_zh(self) -> &'static str {
        match self {
            Self::Overview => "概览",
            Self::ProfileEditor => "配置编辑器",
        }
    }
}

/// Persistent state for the simulation profile editor sub-view.
///
/// Holds editable parameters that the user configures before running a
/// simulation, including component values, model parameters, and solver options.
#[derive(Debug, Default)]
pub(crate) struct SimulationProfileEditorState {
    /// Currently active sub-view within the simulation workspace.
    pub(super) sub_view: SimulationSubView,
    /// Component parameter rows: (name, value, unit).
    pub(super) component_params: Vec<(String, String, String)>,
    /// Model parameter rows: (name, value, unit).
    pub(super) model_params: Vec<(String, String, String)>,
    /// Solver and analysis options.
    pub(super) options: SimOptions,
}

/// Editable simulation options shown in the right column of the profile editor.
///
/// These map to ngspice runtime parameters and `.options` directives.
#[derive(Debug)]
pub(super) struct SimOptions {
    /// Simulation temperature in degrees Celsius.
    pub(super) temperature: String,
    /// Maximum number of Newton-Raphson iterations per timestep.
    pub(super) max_iterations: String,
    /// Minimum allowed timestep (0 = auto).
    pub(super) min_timestep: String,
    /// SPICE integration method: "Gear" or "Trap".
    pub(super) method: String,
    /// Relative convergence tolerance.
    pub(super) reltol: String,
    /// Absolute current convergence tolerance.
    pub(super) abstol: String,
    /// Absolute voltage convergence tolerance.
    pub(super) vntol: String,
}

impl Default for SimOptions {
    /// Default options matching typical ngspice transient analysis settings.
    fn default() -> Self {
        Self {
            temperature: "27".to_string(),
            max_iterations: "200".to_string(),
            min_timestep: "0".to_string(),
            method: "Trap".to_string(),
            reltol: "0.001".to_string(),
            abstol: "1e-12".to_string(),
            vntol: "1e-6".to_string(),
        }
    }
}

impl NekoSpiceApp {
    /// Draw the full profile editor: three-column layout within a panel frame.
    pub(super) fn draw_profile_editor(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            self.draw_profile_editor_header(ui);
            ui.add_space(6.0);

            // Three-column layout: left (analysis + params), center (definitions), right (options)
            ui.horizontal_top(|ui| {
                // Left column: Analysis Setup + Component Params + Model Params
                ui.vertical(|ui| {
                    ui.set_width((ui.available_width() * 0.34).max(240.0));
                    draw_analysis_setup_panel(self, ui);
                    ui.add_space(8.0);
                    draw_component_params(self, ui);
                    ui.add_space(8.0);
                    draw_model_params(self, ui);
                });
                ui.add_space(8.0);
                // Center column: Parameter Definitions editor
                ui.vertical(|ui| {
                    ui.set_width((ui.available_width() * 0.48).max(240.0));
                    draw_parameter_definitions(self, ui);
                });
                ui.add_space(8.0);
                // Right column: Simulation Options + Run Status + Recent Runs
                ui.vertical(|ui| {
                    ui.set_min_width(200.0);
                    draw_profile_options(self, ui);
                });
            });
        });
    }

    /// Header with title and description for the profile editor view.
    fn draw_profile_editor_header(&self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        ui.horizontal_top(|ui| {
            ui.heading(self.text(UiText::SimulationProfile));
            ui.label(StudioTheme::muted_for(
                mode,
                "Configure analysis parameters, component values, and solver options.",
            ));
        });
    }
}
