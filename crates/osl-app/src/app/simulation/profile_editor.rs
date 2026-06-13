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

use super::profile_editor_options::draw_profile_options;
use super::profile_editor_sections::{
    draw_analysis_setup_panel, draw_component_params, draw_model_params, draw_parameter_definitions,
};
use super::sim_options::SimOptions;
use crate::app::NekoSpiceApp;
use crate::app::localization::UiText;
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
    /// Persisted backend name (for restoring from disk).
    #[allow(dead_code)]
    pub(crate) persisted_backend: String,
    /// Persisted directive kind (for restoring from disk).
    #[allow(dead_code)]
    pub(crate) persisted_directive_kind: String,
    /// Section visibility toggles — controls which optional sections are shown.
    pub(crate) toggles: super::section_toggles::SimSectionToggles,
}

impl SimulationProfileEditorState {
    /// Load simulation settings from disk. Falls back to defaults if unavailable.
    pub(crate) fn from_disk() -> Self {
        let (opts, preset, backend, directive_kind, toggles) =
            crate::app::preferences::StudioPreferences::load_simulation_settings();
        Self {
            sub_view: SimulationSubView::default(),
            component_params: Vec::new(),
            model_params: Vec::new(),
            options: opts,
            initial_conditions: Vec::new(),
            nodesets: Vec::new(),
            active_preset: preset,
            persisted_backend: backend,
            persisted_directive_kind: directive_kind,
            toggles,
        }
    }
}

impl NekoSpiceApp {
    /// Draw the full profile editor: three-column layout within a panel frame.
    pub(crate) fn draw_profile_editor(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let palette = self.theme_palette();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            self.draw_profile_editor_header(ui);
            ui.add_space(4.0);

            // Quick run bar: preset selector + run button
            self.draw_preset_selector(ui);
            ui.add_space(4.0);

            // Slim divider
            ui.separator();
            ui.add_space(6.0);

            // Three-column layout
            let total_width = ui.available_width();
            let left_width = (total_width * 0.30).max(220.0);
            let center_width = (total_width * 0.38).max(220.0);

            ui.horizontal_top(|ui| {
                // ── Left Column: Analysis + Optional Sections ──
                ui.vertical(|ui| {
                    ui.set_width(left_width);
                    draw_analysis_setup_panel(self, ui);
                    ui.add_space(6.0);
                    // Read toggle state to avoid borrow conflicts
                    let show_step = self.simulation_profile_editor.toggles.step_sweep;
                    let show_meas = self.simulation_profile_editor.toggles.measurements;
                    let show_comp = self.simulation_profile_editor.toggles.component_params;
                    let show_model = self.simulation_profile_editor.toggles.model_params;
                    let show_ic = self.simulation_profile_editor.toggles.initial_conditions;
                    if show_step {
                        self.draw_step_sweep_editor(ui, mode);
                        ui.add_space(6.0);
                    }
                    if show_meas {
                        self.draw_measure_editor(ui, mode);
                        ui.add_space(6.0);
                    }
                    if show_comp {
                        draw_component_params(self, ui);
                        ui.add_space(6.0);
                    }
                    if show_model {
                        draw_model_params(self, ui);
                        ui.add_space(6.0);
                    }
                    if show_ic {
                        super::options_ic::draw_initial_conditions_section(self, ui, mode);
                    }
                });

                // Vertical separator
                let sep_rect = egui::Rect::from_min_size(
                    ui.cursor().min,
                    egui::Vec2::new(1.0, ui.available_height()),
                );
                ui.painter().rect_filled(sep_rect, 0.0, palette.border);
                ui.allocate_space(egui::Vec2::new(8.0, 0.0));

                // ── Center Column: Parameter Definitions ──
                ui.vertical(|ui| {
                    ui.set_width(center_width);
                    draw_parameter_definitions(self, ui);
                });

                // Vertical separator
                let sep_rect = egui::Rect::from_min_size(
                    ui.cursor().min,
                    egui::Vec2::new(1.0, ui.available_height()),
                );
                ui.painter().rect_filled(sep_rect, 0.0, palette.border);
                ui.allocate_space(egui::Vec2::new(8.0, 0.0));

                // ── Right Column: Options + Status ──
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
                        egui::RichText::new(*label)
                            .strong()
                            .color(self.theme_palette().text),
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
