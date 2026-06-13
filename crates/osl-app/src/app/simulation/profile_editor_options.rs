//! Right column of the simulation profile editor — thin orchestrator.
//!
//! Composes focused sub-modules and persists changes to disk whenever
//! the user modifies any simulation option field.
//!
//! Delegates to:
//! - `options_preset` — quick-apply simulation presets
//! - `options_environment` — temperature, nominal temperature
//! - `options_solver` — transient solver, convergence, output control
//! - `options_ic` — initial conditions (.ic / .nodeset)
//! - `options_status` — run status and recent runs

use crate::app::NekoSpiceApp;
use crate::app::theme::StudioTheme;
use eframe::egui;

use super::options_preset::draw_preset_selector;
use super::options_environment::draw_environment_section;
use super::options_solver::{draw_transient_solver_section, draw_convergence_section, draw_output_section};
use super::options_ic::draw_initial_conditions_section;
use super::options_status::{draw_run_status_summary, draw_recent_runs};

/// Draw the complete right-column options panel with all sections.
///
/// Each section reports whether any field was changed. If so, the
/// entire simulation settings are persisted to disk.
pub(crate) fn draw_profile_options(app: &mut NekoSpiceApp, ui: &mut egui::Ui) {
    let mode = app.theme_mode();
    let palette = StudioTheme::palette(mode);
    let mut any_changed = false;

    // Preset selector (top of panel)
    any_changed |= draw_preset_selector(app, ui, mode);
    ui.add_space(8.0);

    // Environment (temperature, TNOM)
    any_changed |= draw_environment_section(app, ui, mode);
    ui.add_space(8.0);

    // Transient solver (method, iteration limits, timestep)
    egui::CollapsingHeader::new(egui::RichText::new("Transient Solver").color(palette.text))
        .id_salt("collapsible_transient")
        .default_open(false)
        .show(ui, |ui| {
            any_changed |= draw_transient_solver_section(app, ui, mode);
        });
    ui.add_space(8.0);

    // Convergence tolerances (RELTOL, ABSTOL, VNTOL, GMIN, etc.)
    egui::CollapsingHeader::new(egui::RichText::new("Convergence").color(palette.text))
        .id_salt("collapsible_convergence")
        .default_open(false)
        .show(ui, |ui| {
            any_changed |= draw_convergence_section(app, ui, mode);
        });
    ui.add_space(8.0);

    // Output control (NUMDGT)
    any_changed |= draw_output_section(app, ui, mode);
    ui.add_space(8.0);

    // Initial conditions (.ic / .nodeset)
    egui::CollapsingHeader::new(egui::RichText::new("Initial Conditions").color(palette.text))
        .id_salt("collapsible_ic")
        .default_open(false)
        .show(ui, |ui| {
            any_changed |= draw_initial_conditions_section(app, ui, mode);
        });
    ui.add_space(8.0);

    // Run status and recent runs (read-only)
    draw_run_status_summary(app, ui, mode);
    ui.add_space(8.0);
    draw_recent_runs(app, ui, mode);
    ui.add_space(8.0);
    app.draw_history_panel(ui, mode);

    // Persist to disk when any field was modified
    if any_changed {
        app.save_simulation_settings();
    }
}
