//! Right column of the simulation profile editor — thin orchestrator.
//!
//! Composes focused sub-modules and persists changes to disk whenever
//! the user modifies any simulation option field.

use crate::app::NekoSpiceApp;
use crate::app::theme::StudioTheme;
use eframe::egui;

use super::options_preset::draw_preset_indicator;
use super::options_environment::draw_environment_section;
use super::options_solver::{draw_transient_solver_section, draw_convergence_section, draw_output_section};
use super::options_ic::draw_initial_conditions_section;
use super::options_status::{draw_run_status_summary, draw_recent_runs};

/// Draw the complete right-column options panel.
pub(crate) fn draw_profile_options(app: &mut NekoSpiceApp, ui: &mut egui::Ui) {
    let mode = app.theme_mode();
    let palette = StudioTheme::palette(mode);
    let mut any_changed = false;

    // Active preset indicator
    draw_preset_indicator(app, ui, mode);
    ui.add_space(8.0);

    // Environment (temperature, TNOM)
    any_changed |= draw_environment_section(app, ui, mode);
    ui.add_space(8.0);

    // Transient solver (collapsible)
    egui::CollapsingHeader::new(egui::RichText::new("Transient Solver").color(palette.text))
        .id_salt("collapsible_transient")
        .default_open(false)
        .show(ui, |ui| {
            any_changed |= draw_transient_solver_section(app, ui, mode);
        });
    ui.add_space(8.0);

    // Convergence tolerances (collapsible)
    egui::CollapsingHeader::new(egui::RichText::new("Convergence").color(palette.text))
        .id_salt("collapsible_convergence")
        .default_open(false)
        .show(ui, |ui| {
            any_changed |= draw_convergence_section(app, ui, mode);
        });
    ui.add_space(8.0);

    // Output control
    any_changed |= draw_output_section(app, ui, mode);
    ui.add_space(8.0);

    // Initial conditions (collapsible)
    egui::CollapsingHeader::new(egui::RichText::new("Initial Conditions").color(palette.text))
        .id_salt("collapsible_ic")
        .default_open(false)
        .show(ui, |ui| {
            any_changed |= draw_initial_conditions_section(app, ui, mode);
        });
    ui.add_space(8.0);

    // Run status and recent runs
    draw_run_status_summary(app, ui, mode);
    ui.add_space(8.0);
    draw_recent_runs(app, ui, mode);
    ui.add_space(8.0);
    app.draw_history_panel(ui, mode);

    if any_changed {
        app.save_simulation_settings();
    }
}
