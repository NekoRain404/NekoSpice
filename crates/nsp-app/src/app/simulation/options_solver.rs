//! Transient solver, convergence, and output control sections.
//!
//! Each section returns `true` when any field is modified, enabling the
//! orchestrator to call `save_simulation_settings()` only when needed.

use super::profile_editor_widgets::{labeled_field, section_header};
use crate::app::NekoSpiceApp;
use crate::app::theme::{StudioTheme, StudioThemeMode};
use eframe::egui;

/// Draw transient solver settings: method selector and iteration limits.
/// Returns `true` when any field changes.
pub(crate) fn draw_transient_solver_section(
    app: &mut NekoSpiceApp,
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
) -> bool {
    let palette = StudioTheme::palette(mode);
    let mut changed = false;

    StudioTheme::panel_frame_for(mode).show(ui, |ui| {
        section_header(ui, mode, "Transient Solver");
        ui.add_space(4.0);

        // Integration method selector (Trap / Gear)
        ui.label(StudioTheme::muted_for(mode, "Integration Method"));
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            for method in ["Trap", "Gear"] {
                let active = app.simulation_profile_editor.options.method == method;
                let btn = if active {
                    egui::Button::new(egui::RichText::new(method).strong().color(palette.text))
                        .fill(palette.accent_soft)
                        .stroke(egui::Stroke::new(1.0, palette.accent))
                } else {
                    egui::Button::new(egui::RichText::new(method).color(palette.text_muted))
                        .fill(palette.panel_soft)
                        .stroke(egui::Stroke::new(1.0, palette.border))
                };
                if ui.add(btn).clicked() && app.simulation_profile_editor.options.method != method {
                    app.simulation_profile_editor.options.method = method.to_string();
                    changed = true;
                }
            }
        });

        ui.add_space(6.0);
        // Iteration limit and timestep controls
        egui::Grid::new("transient_grid")
            .num_columns(2)
            .spacing([8.0, 6.0])
            .show(ui, |ui| {
                changed |= labeled_field(
                    ui,
                    mode,
                    "ITL1 (DC iterations)",
                    &mut app.simulation_profile_editor.options.itl1,
                    100.0,
                );
                changed |= labeled_field(
                    ui,
                    mode,
                    "ITL2 (DC sweep iters)",
                    &mut app.simulation_profile_editor.options.itl2,
                    100.0,
                );
                changed |= labeled_field(
                    ui,
                    mode,
                    "ITL4 (tran iters/step)",
                    &mut app.simulation_profile_editor.options.itl4,
                    100.0,
                );
                changed |= labeled_field(
                    ui,
                    mode,
                    "ITL5 (tran total iters)",
                    &mut app.simulation_profile_editor.options.itl5,
                    100.0,
                );
                changed |= labeled_field(
                    ui,
                    mode,
                    "TRTOL (min timestep)",
                    &mut app.simulation_profile_editor.options.min_timestep,
                    100.0,
                );
                changed |= labeled_field(
                    ui,
                    mode,
                    "SRCSTEPS",
                    &mut app.simulation_profile_editor.options.srcsteps,
                    100.0,
                );
                changed |= labeled_field(
                    ui,
                    mode,
                    "GMINSTEPS",
                    &mut app.simulation_profile_editor.options.gminsteps,
                    100.0,
                );
            });

        ui.add_space(6.0);
        // Reset to SPICE defaults
        if ui.small_button("Reset Transient Defaults").clicked() {
            let defaults = super::sim_options::SimOptions::default();
            app.simulation_profile_editor.options.method = defaults.method;
            app.simulation_profile_editor.options.itl1 = defaults.itl1;
            app.simulation_profile_editor.options.itl2 = defaults.itl2;
            app.simulation_profile_editor.options.itl4 = defaults.itl4;
            app.simulation_profile_editor.options.itl5 = defaults.itl5;
            app.simulation_profile_editor.options.min_timestep = defaults.min_timestep;
            app.simulation_profile_editor.options.srcsteps = defaults.srcsteps;
            app.simulation_profile_editor.options.gminsteps = defaults.gminsteps;
            changed = true;
        }
    });

    changed
}

/// Draw convergence section with all tolerance settings.
/// Returns `true` when any field changes.
pub(crate) fn draw_convergence_section(
    app: &mut NekoSpiceApp,
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
) -> bool {
    let mut changed = false;

    StudioTheme::panel_frame_for(mode).show(ui, |ui| {
        section_header(ui, mode, "Convergence");
        ui.add_space(4.0);

        // Tolerance grid
        egui::Grid::new("convergence_grid")
            .num_columns(2)
            .spacing([8.0, 6.0])
            .show(ui, |ui| {
                changed |= labeled_field(
                    ui,
                    mode,
                    "RELTOL",
                    &mut app.simulation_profile_editor.options.reltol,
                    100.0,
                );
                changed |= labeled_field(
                    ui,
                    mode,
                    "ABSTOL (A)",
                    &mut app.simulation_profile_editor.options.abstol,
                    100.0,
                );
                changed |= labeled_field(
                    ui,
                    mode,
                    "VNTOL (V)",
                    &mut app.simulation_profile_editor.options.vntol,
                    100.0,
                );
                changed |= labeled_field(
                    ui,
                    mode,
                    "GMIN (S)",
                    &mut app.simulation_profile_editor.options.gmin,
                    100.0,
                );
                changed |= labeled_field(
                    ui,
                    mode,
                    "CHGTOL (C)",
                    &mut app.simulation_profile_editor.options.chgtol,
                    100.0,
                );
                changed |= labeled_field(
                    ui,
                    mode,
                    "PIVTOL",
                    &mut app.simulation_profile_editor.options.pivtol,
                    100.0,
                );
                changed |= labeled_field(
                    ui,
                    mode,
                    "PIVREL",
                    &mut app.simulation_profile_editor.options.pivrel,
                    100.0,
                );
            });

        ui.add_space(6.0);
        // Reset to SPICE defaults
        if ui.small_button("Reset Convergence Defaults").clicked() {
            let defaults = super::sim_options::SimOptions::default();
            app.simulation_profile_editor.options.reltol = defaults.reltol;
            app.simulation_profile_editor.options.abstol = defaults.abstol;
            app.simulation_profile_editor.options.vntol = defaults.vntol;
            app.simulation_profile_editor.options.gmin = defaults.gmin;
            app.simulation_profile_editor.options.chgtol = defaults.chgtol;
            app.simulation_profile_editor.options.pivtol = defaults.pivtol;
            app.simulation_profile_editor.options.pivrel = defaults.pivrel;
            changed = true;
        }
    });

    changed
}

/// Draw output control section (NUMDGT).
/// Returns `true` when the field changes.
pub(crate) fn draw_output_section(
    app: &mut NekoSpiceApp,
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
) -> bool {
    let mut changed = false;

    StudioTheme::panel_frame_for(mode).show(ui, |ui| {
        section_header(ui, mode, "Output Control");
        ui.add_space(4.0);
        egui::Grid::new("output_grid")
            .num_columns(2)
            .spacing([8.0, 6.0])
            .show(ui, |ui| {
                changed |= labeled_field(
                    ui,
                    mode,
                    "NUMDGT (digits)",
                    &mut app.simulation_profile_editor.options.numdgt,
                    100.0,
                );
            });
    });

    changed
}
