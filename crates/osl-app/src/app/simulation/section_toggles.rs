//! Simulation section toggles — controls which optional sections are visible
//! in the profile editor and sidebar panel.
//!
//! Users can enable/disable sections they don't need, reducing visual clutter
//! and making the workflow more streamlined. Each toggle maps to a collapsible
//! section. Disabled sections keep their state but are hidden from the UI.

use crate::app::NekoSpiceApp;
use crate::app::theme::StudioTheme;
use eframe::egui;

/// Which optional simulation sections are enabled in the UI.
///
/// All sections default to `true` (visible). Users can toggle them off
/// via the section header or the customize menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SimSectionToggles {
    /// Step sweep editor (`.step` parameter sweep / temperature sweep).
    pub step_sweep: bool,
    /// Measurement directives (`.measure` post-simulation extraction).
    pub measurements: bool,
    /// Initial conditions (`.ic` / `.nodeset` entries).
    pub initial_conditions: bool,
    /// Component parameter overrides table.
    pub component_params: bool,
    /// Model parameter overrides table.
    pub model_params: bool,
    /// Quick Start templates panel (sidebar only).
    pub quick_start: bool,
    /// Netlist preview section.
    pub netlist_preview: bool,
    /// Run status and recent runs.
    pub run_status: bool,
    /// Transient solver settings (method, iteration limits).
    pub transient_solver: bool,
    /// Convergence tolerance settings.
    pub convergence: bool,
    /// Output control (NUMDGT).
    pub output_control: bool,
}

impl Default for SimSectionToggles {
    fn default() -> Self {
        Self {
            step_sweep: true,
            measurements: true,
            initial_conditions: false,
            component_params: true,
            model_params: false,
            quick_start: true,
            netlist_preview: true,
            run_status: true,
            transient_solver: false,
            convergence: false,
            output_control: true,
        }
    }
}

impl SimSectionToggles {
    /// Count how many optional sections are currently enabled.
    #[allow(dead_code)]
    pub(crate) fn enabled_count(&self) -> usize {
        [
            self.step_sweep,
            self.measurements,
            self.initial_conditions,
            self.component_params,
            self.model_params,
            self.quick_start,
            self.netlist_preview,
            self.run_status,
            self.transient_solver,
            self.convergence,
            self.output_control,
        ]
        .iter()
        .filter(|&&v| v)
        .count()
    }
}

/// Draw a section header with an enable/disable toggle checkbox.
///
/// Returns `true` if the section is visible (enabled), `false` if toggled off.
/// The header shows the section title and a small checkbox on the right.
#[allow(dead_code)]
pub(crate) fn toggleable_section_header(
    ui: &mut egui::Ui,
    mode: crate::app::theme::StudioThemeMode,
    title: &str,
    enabled: &mut bool,
) -> bool {
    ui.horizontal(|ui| {
        ui.label(StudioTheme::section_title_for(mode, title));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add(egui::Checkbox::without_text(enabled));
        });
    });
    *enabled
}

impl NekoSpiceApp {
    /// Draw the customize view menu that lets users toggle section visibility.
    pub(crate) fn draw_customize_view_menu(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let t = &mut self.simulation_profile_editor.toggles;

        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(mode, "Customize View"));
            ui.add_space(4.0);
            ui.label(StudioTheme::muted_for(mode, "Toggle optional sections on/off"));
            ui.add_space(4.0);

            egui::Grid::new("customize_view_grid")
                .num_columns(2)
                .spacing([8.0, 4.0])
                .show(ui, |ui| {
                    toggle_row(ui, mode, "Step Sweep", &mut t.step_sweep);
                    toggle_row(ui, mode, "Measurements", &mut t.measurements);
                    toggle_row(ui, mode, "Initial Conditions", &mut t.initial_conditions);
                    toggle_row(ui, mode, "Component Params", &mut t.component_params);
                    toggle_row(ui, mode, "Model Params", &mut t.model_params);
                    toggle_row(ui, mode, "Quick Start", &mut t.quick_start);
                    toggle_row(ui, mode, "Netlist Preview", &mut t.netlist_preview);
                    toggle_row(ui, mode, "Run Status", &mut t.run_status);
                    toggle_row(ui, mode, "Transient Solver", &mut t.transient_solver);
                    toggle_row(ui, mode, "Convergence", &mut t.convergence);
                    toggle_row(ui, mode, "Output Control", &mut t.output_control);
                });
        });
    }
}

/// Draw a single toggle row with label and checkbox.
fn toggle_row(
    ui: &mut egui::Ui,
    _mode: crate::app::theme::StudioThemeMode,
    label: &str,
    enabled: &mut bool,
) {
    ui.label(label);
    ui.add(egui::Checkbox::without_text(enabled));
    ui.end_row();
}
