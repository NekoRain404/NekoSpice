//! Right column of the simulation profile editor — thin orchestrator.
//!
//! Composes focused sub-modules and persists changes to disk whenever
//! the user modifies any simulation option field.
//!
//! When the Xyce backend is selected, Xyce-specific solver options are
//! shown alongside the standard ngspice options. The run comparison panel
//! is available when there are at least 2 historical runs.

use crate::app::NekoSpiceApp;
use crate::app::theme::StudioTheme;
use eframe::egui;

use super::options_environment::draw_environment_section;
use super::options_preset::draw_preset_indicator;
use super::options_solver::{
    draw_convergence_section, draw_output_section, draw_transient_solver_section,
};
use super::options_status::{draw_recent_runs, draw_run_status_summary};
use super::state::SimulationBackendKind;

/// Draw the complete right-column options panel.
///
/// Sections are shown/hidden based on the user's `SimSectionToggles`.
/// Xyce-specific options are only shown when the Xyce backend is selected.
pub(crate) fn draw_profile_options(app: &mut NekoSpiceApp, ui: &mut egui::Ui) {
    let mode = app.theme_mode();
    let palette = StudioTheme::palette(mode);
    let mut any_changed = false;

    // Read toggle state first to avoid borrow conflicts
    let show_transient = app.simulation_profile_editor.toggles.transient_solver;
    let show_convergence = app.simulation_profile_editor.toggles.convergence;
    let show_output = app.simulation_profile_editor.toggles.output_control;
    let show_run_status = app.simulation_profile_editor.toggles.run_status;

    // Active preset indicator
    draw_preset_indicator(app, ui, mode);
    ui.add_space(8.0);

    // Environment (temperature, TNOM) — always shown
    any_changed |= draw_environment_section(app, ui, mode);
    ui.add_space(8.0);

    // Transient solver — togglable
    if show_transient {
        egui::CollapsingHeader::new(egui::RichText::new("Transient Solver").color(palette.text))
            .id_salt("collapsible_transient")
            .default_open(false)
            .show(ui, |ui| {
                any_changed |= draw_transient_solver_section(app, ui, mode);
            });
        ui.add_space(8.0);
    }

    // Convergence tolerances — togglable
    if show_convergence {
        egui::CollapsingHeader::new(egui::RichText::new("Convergence").color(palette.text))
            .id_salt("collapsible_convergence")
            .default_open(false)
            .show(ui, |ui| {
                any_changed |= draw_convergence_section(app, ui, mode);
            });
        ui.add_space(8.0);
    }

    // Output control — togglable
    if show_output {
        any_changed |= draw_output_section(app, ui, mode);
        ui.add_space(8.0);
    }

    // Xyce-specific options — only when Xyce backend is selected
    if app.simulation_panel.backend == SimulationBackendKind::Xyce {
        any_changed |= super::options_xyce::draw_xyce_options_section(app, ui, mode);
        ui.add_space(8.0);
    }

    // Run status and recent runs
    if show_run_status {
        draw_run_status_summary(app, ui, mode);
        ui.add_space(8.0);
        draw_recent_runs(app, ui, mode);
        ui.add_space(8.0);
    }

    app.draw_history_panel(ui, mode);
    ui.add_space(8.0);

    // Run comparison — available when there are 2+ runs
    app.draw_run_compare_panel(ui, mode);
    ui.add_space(8.0);

    // Custom Presets — save/load user-defined configurations
    app.draw_custom_presets_panel(ui, mode);
    ui.add_space(8.0);

    // Customize View — collapsible section toggles
    egui::CollapsingHeader::new(egui::RichText::new("Customize View").color(palette.text_muted))
        .id_salt("customize_view_toggles")
        .default_open(false)
        .show(ui, |ui| {
            app.draw_customize_view_menu(ui);
        });

    if any_changed {
        app.save_simulation_settings();
    }
}
