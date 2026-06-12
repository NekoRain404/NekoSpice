//! Simulation Profile Editor — three-column layout for editing analysis parameters,
//! component/model parameters, simulation options, and viewing run status.
//!
//! Layout:
//!   Left   — Analysis Setup + Component/Model parameters
//!   Center — Parameter definitions editor
//!   Right  — Simulation Options + Run Status + Recent runs
//!
//! The `SimOptions` struct mirrors all fields in `SimulationProfile` so that
//! the GUI can edit them independently before committing to a profile build.

use crate::app::NekoSpiceApp;
use crate::app::localization::UiText;
use super::profile_editor_options::draw_profile_options;
use super::profile_editor_sections::{
    draw_analysis_setup_panel, draw_component_params, draw_model_params,
    draw_parameter_definitions,
};
use crate::app::theme::StudioTheme;
use eframe::egui;

/// Sub-views available within the simulation workspace.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum SimulationSubView {
    /// High-level solver metrics, analysis setup, and netlist preview.
    #[default]
    Overview,
    /// Detailed profile editor with three-column parameter layout.
    ProfileEditor,
}

impl SimulationSubView {
    /// English label for the sub-view tab.
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Overview => "Overview",
            Self::ProfileEditor => "Profile Editor",
        }
    }

    /// Simplified Chinese label for the sub-view tab.
    pub(crate) fn label_zh(self) -> &'static str {
        match self {
            Self::Overview => "概览",
            Self::ProfileEditor => "配置编辑器",
        }
    }
}

/// Persistent state for the simulation profile editor sub-view.
///
/// Holds editable parameters that the user configures before running a
/// simulation, including component values, model parameters, solver options,
/// initial conditions, and nodeset hints.
#[derive(Debug, Default)]
pub(crate) struct SimulationProfileEditorState {
    /// Currently active sub-view within the simulation workspace.
    pub(crate) sub_view: SimulationSubView,
    /// Component parameter rows: (name, value, unit).
    pub(crate) component_params: Vec<(String, String, String)>,
    /// Model parameter rows: (name, value, unit).
    pub(crate) model_params: Vec<(String, String, String)>,
    /// Solver and analysis options (mirrors SimulationProfile fields).
    pub(crate) options: SimOptions,
    /// `.ic` entries: (node_name, initial_voltage).
    pub(crate) initial_conditions: Vec<(String, String)>,
    /// `.nodeset` entries: (node_name, initial_guess).
    pub(crate) nodesets: Vec<(String, String)>,
    /// Name of the active preset ("default" or one from available_presets()).
    pub(crate) active_preset: String,
}

impl SimulationProfileEditorState {
    /// Load simulation settings from disk. Falls back to defaults if unavailable.
    pub(crate) fn from_disk() -> Self {
        let (opts, preset) = crate::app::preferences::StudioPreferences::load_simulation_settings();
        Self {
            sub_view: SimulationSubView::default(),
            component_params: Vec::new(),
            model_params: Vec::new(),
            options: opts,
            initial_conditions: Vec::new(),
            nodesets: Vec::new(),
            active_preset: preset,
        }
    }
}

/// Editable simulation options shown in the profile editor.
///
/// These map directly to SPICE `.options` directives. Organized into
/// logical groups matching the SimulationProfile struct.
#[derive(Debug)]
pub(crate) struct SimOptions {
    // ── Environment ────────────────────────────────────────────────────
    /// Simulation temperature in degrees Celsius.
    pub(crate) temperature: String,
    /// Nominal temperature for model evaluation (°C).
    pub(crate) tnom: String,

    // ── Transient Solver ───────────────────────────────────────────────
    /// SPICE integration method: "Gear" or "Trap".
    pub(crate) method: String,
    /// DC operating point iteration limit (ITL1).
    pub(crate) itl1: String,
    /// DC transfer curve iteration limit (ITL2).
    pub(crate) itl2: String,
    /// Transient timestep iteration limit (ITL4).
    pub(crate) itl4: String,
    /// Transient total iteration limit (ITL5, 0 = no limit).
    pub(crate) itl5: String,
    /// Minimum allowed timestep (TRTOL). 0 = auto.
    pub(crate) min_timestep: String,
    /// Source stepping iterations (0 = auto).
    pub(crate) srcsteps: String,
    /// GMIN stepping iterations (0 = auto).
    pub(crate) gminsteps: String,

    // ── Convergence ────────────────────────────────────────────────────
    /// Relative convergence tolerance (RELTOL).
    pub(crate) reltol: String,
    /// Absolute current convergence tolerance (ABSTOL).
    pub(crate) abstol: String,
    /// Absolute voltage convergence tolerance (VNTOL).
    pub(crate) vntol: String,
    /// Minimum conductance to ground (GMIN).
    pub(crate) gmin: String,
    /// Charge convergence tolerance (CHGTOL).
    pub(crate) chgtol: String,
    /// Absolute pivot tolerance for LU decomposition (PIVTOL).
    pub(crate) pivtol: String,
    /// Relative pivot tolerance (PIVREL).
    pub(crate) pivrel: String,

    // ── Output Control ─────────────────────────────────────────────────
    /// Number of significant digits in output (NUMDGT).
    pub(crate) numdgt: String,
}

impl Default for SimOptions {
    /// Default options matching standard SPICE defaults.
    fn default() -> Self {
        Self {
            temperature: "27".to_string(),
            tnom: "27".to_string(),
            method: "Trap".to_string(),
            itl1: "100".to_string(),
            itl2: "50".to_string(),
            itl4: "10".to_string(),
            itl5: "5000".to_string(),
            min_timestep: "0".to_string(),
            srcsteps: "0".to_string(),
            gminsteps: "0".to_string(),
            reltol: "0.001".to_string(),
            abstol: "1e-12".to_string(),
            vntol: "1e-6".to_string(),
            gmin: "1e-12".to_string(),
            chgtol: "1e-14".to_string(),
            pivtol: "1e-13".to_string(),
            pivrel: "1e-3".to_string(),
            numdgt: "6".to_string(),
        }
    }
}

impl SimOptions {
    /// Apply a named preset's values, overwriting all current settings.
    pub(crate) fn apply_preset(&mut self, preset: &str) {
        use osl_sim::{simulation_preset, SpiceMethod};
        let p = simulation_preset(preset);
        self.temperature = p.temperature;
        self.tnom = p.tnom;
        self.method = match p.method {
            SpiceMethod::Trap => "Trap".to_string(),
            SpiceMethod::Gear => "Gear".to_string(),
        };
        self.itl1 = p.itl1;
        self.itl2 = p.itl2;
        self.itl4 = p.itl4;
        self.itl5 = p.itl5;
        self.min_timestep = p.min_timestep;
        self.srcsteps = p.srcsteps;
        self.gminsteps = p.gminsteps;
        self.reltol = p.reltol;
        self.abstol = p.abstol;
        self.vntol = p.vntol;
        self.gmin = p.gmin;
        self.chgtol = p.chgtol;
        self.pivtol = p.pivtol;
        self.pivrel = p.pivrel;
        self.numdgt = p.numdgt;
    }
}

impl NekoSpiceApp {
    /// Draw the full profile editor: three-column layout within a panel frame.
    pub(crate) fn draw_profile_editor(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            self.draw_profile_editor_header(ui);
            ui.add_space(6.0);

            // Preset selector row at the top
            self.draw_preset_selector(ui);
            ui.add_space(8.0);

            // Three-column layout: left (analysis + params), center (definitions), right (options)
            ui.horizontal_top(|ui| {
                // Left column: Analysis Setup + Component Params + Model Params
                ui.vertical(|ui| {
                    ui.set_width((ui.available_width() * 0.34).max(240.0));
                    draw_analysis_setup_panel(self, ui);
                    ui.add_space(8.0);
                    self.draw_step_sweep_editor(ui, mode);
                    ui.add_space(8.0);
                    self.draw_measure_editor(ui, mode);
                    ui.add_space(8.0);
                    draw_component_params(self, ui);
                    ui.add_space(8.0);
                    draw_model_params(self, ui);
                });
                ui.add_space(8.0);
                // Center column: Parameter Definitions editor + Initial Conditions
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

    /// Preset selector row: quick access to named simulation presets.
    fn draw_preset_selector(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        ui.horizontal(|ui| {
            ui.label(StudioTheme::muted_for(mode, "Preset:"));
            for (name, label) in osl_sim::available_presets() {
                let active = self.simulation_profile_editor.active_preset == *name;
                let btn = if active {
                    egui::Button::new(
                        egui::RichText::new(*label).strong().color(self.theme_palette().text),
                    )
                    .fill(self.theme_palette().accent_soft)
                    .stroke(egui::Stroke::new(1.0, self.theme_palette().accent))
                } else {
                    egui::Button::new(
                        egui::RichText::new(*label).color(self.theme_palette().text_muted),
                    )
                    .fill(self.theme_palette().panel_soft)
                    .stroke(egui::Stroke::new(1.0, self.theme_palette().border))
                };
                if ui.add(btn).clicked() {
                    self.simulation_profile_editor.active_preset = name.to_string();
                    self.simulation_profile_editor.options.apply_preset(name);
                    self.save_simulation_settings();
                }
            }
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
