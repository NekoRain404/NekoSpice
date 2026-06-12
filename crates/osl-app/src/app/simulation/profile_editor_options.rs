//! Right column of the simulation profile editor — thin orchestrator.
//!
//! Delegates to focused sub-modules:
//! - `options_environment` — temperature, nominal temperature
//! - `options_solver` — transient solver, convergence, output control
//! - `options_ic` — initial conditions (.ic / .nodeset)
//! - `options_status` — run status and recent runs

use crate::app::NekoSpiceApp;
use crate::app::theme::StudioTheme;
use eframe::egui;

use super::options_environment::draw_environment_section;
use super::options_solver::{draw_transient_solver_section, draw_convergence_section, draw_output_section};
use super::options_ic::draw_initial_conditions_section;
use super::options_status::{draw_run_status_summary, draw_recent_runs};

/// Draw the complete right-column options panel with all sections.
pub(crate) fn draw_profile_options(app: &mut NekoSpiceApp, ui: &mut egui::Ui) {
    let mode = app.theme_mode();
    let palette = StudioTheme::palette(mode);

    draw_environment_section(app, ui, mode);
    ui.add_space(8.0);

    egui::CollapsingHeader::new(egui::RichText::new("Transient Solver").color(palette.text))
        .id_salt("collapsible_transient")
        .default_open(false)
        .show(ui, |ui| {
            draw_transient_solver_section(app, ui, mode);
        });
    ui.add_space(8.0);

    egui::CollapsingHeader::new(egui::RichText::new("Convergence").color(palette.text))
        .id_salt("collapsible_convergence")
        .default_open(false)
        .show(ui, |ui| {
            draw_convergence_section(app, ui, mode);
        });
    ui.add_space(8.0);

    draw_output_section(app, ui, mode);
    ui.add_space(8.0);

    egui::CollapsingHeader::new(egui::RichText::new("Initial Conditions").color(palette.text))
        .id_salt("collapsible_ic")
        .default_open(false)
        .show(ui, |ui| {
            draw_initial_conditions_section(app, ui, mode);
        });
    ui.add_space(8.0);

    draw_run_status_summary(app, ui, mode);
    ui.add_space(8.0);

    draw_recent_runs(app, ui, mode);
}
