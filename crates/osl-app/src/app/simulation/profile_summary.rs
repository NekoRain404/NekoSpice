//! Profile summary — shows all configured simulation settings
//! in a compact, readable format for the overview workspace.

use crate::app::NekoSpiceApp;
use crate::app::localization::UiText;
use super::workspace_widgets::profile_row;
use crate::app::theme::StudioTheme;
use eframe::egui;


impl NekoSpiceApp {
    /// Draw comprehensive profile summary showing all configured simulation settings.
    pub(crate) fn draw_simulation_profile_summary(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let palette = self.theme_palette();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::SimulationProfile),
            ));

            // Active preset indicator
            let preset_name = &self.simulation_profile_editor.active_preset;
            if preset_name != "default" {
                ui.horizontal(|ui| {
                    ui.label(StudioTheme::muted_for(mode, "Preset:"));
                    ui.label(egui::RichText::new(preset_name).strong().color(palette.accent));
                });
            }

            // Analysis directive (built from structured params)
            let analysis = format!("{} {}",
                self.simulation_panel.directive_kind.to_string(),
                self.simulation_panel.analysis_params.to_body().trim()
            ).trim().to_string();
            profile_row(ui, mode, "Analysis", &analysis, "active");

            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);

            // Environment settings
            ui.label(StudioTheme::muted_for(mode, "Environment"));
            let opts = &self.simulation_profile_editor.options;
            profile_row(ui, mode, "Temperature", &format!("{} °C", opts.temperature), "operating");
            if opts.tnom != "27" {
                profile_row(ui, mode, "TNOM", &format!("{} °C", opts.tnom), "nominal");
            }

            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);

            // Solver settings
            ui.label(StudioTheme::muted_for(mode, "Solver"));
            profile_row(ui, mode, "Method", &opts.method, "integration");
            profile_row(ui, mode, "RELTOL", &opts.reltol, "tolerance");
            if opts.abstol != "1e-12" {
                profile_row(ui, mode, "ABSTOL", &opts.abstol, "tolerance");
            }
            if opts.vntol != "1e-6" {
                profile_row(ui, mode, "VNTOL", &opts.vntol, "tolerance");
            }
            if opts.gmin != "1e-12" {
                profile_row(ui, mode, "GMIN", &opts.gmin, "conductance");
            }

            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);

            // Iteration limits
            ui.label(StudioTheme::muted_for(mode, "Iteration Limits"));
            profile_row(ui, mode, "ITL1 (DC)", &opts.itl1, "limit");
            if opts.itl4 != "10" {
                profile_row(ui, mode, "ITL4 (tran)", &opts.itl4, "limit");
            }
            if opts.itl5 != "5000" {
                profile_row(ui, mode, "ITL5 (total)", &opts.itl5, "limit");
            }

            // Step sweep (parametric or temperature)
            match &self.simulation_panel.step_sweep {
                super::state::StepSweep::Parametric { param_name, sweep_mode, start, stop, step } => {
                    ui.add_space(4.0);
                    ui.separator();
                    ui.add_space(4.0);
                    ui.label(StudioTheme::muted_for(mode, "Step Sweep"));
                    let sweep_desc = match sweep_mode.as_str() {
                        "list" => format!("{} list {}", param_name, start),
                        "lin" => format!("{} {} to {} step {}", param_name, start, stop, step),
                        "dec" => format!("{} dec {} pts/dec {} to {}", param_name, step, start, stop),
                        "oct" => format!("{} oct {} pts/oct {} to {}", param_name, step, start, stop),
                        _ => format!("{} {} {} {}", param_name, sweep_mode, start, stop),
                    };
                    profile_row(ui, mode, ".step", &sweep_desc, "sweep");
                }
                super::state::StepSweep::Temperature { sweep_mode, start, stop, step } => {
                    ui.add_space(4.0);
                    ui.separator();
                    ui.add_space(4.0);
                    ui.label(StudioTheme::muted_for(mode, "Temperature Sweep"));
                    let sweep_desc = match sweep_mode.as_str() {
                        "lin" => format!("TEMP {} to {} step {}", start, stop, step),
                        "dec" => format!("TEMP dec {} pts/dec {} to {}", step, start, stop),
                        "oct" => format!("TEMP oct {} pts/oct {} to {}", step, start, stop),
                        _ => format!("TEMP {} {} to {} step {}", sweep_mode, start, stop, step),
                    };
                    profile_row(ui, mode, ".step", &sweep_desc, "sweep");
                }
                super::state::StepSweep::None => {}
            }

            // Measurements
            if !self.simulation_measurements.is_empty() {
                ui.add_space(4.0);
                ui.separator();
                ui.add_space(4.0);
                ui.label(StudioTheme::muted_for(mode, "Measurements"));
                for entry in &self.simulation_measurements {
                    if !entry.name.is_empty() && !entry.expression.is_empty() {
                        profile_row(ui, mode, &entry.name, &entry.expression, "measure");
                    }
                }
            }

            // Initial conditions
            if !self.simulation_profile_editor.initial_conditions.is_empty() {
                ui.add_space(4.0);
                ui.separator();
                ui.add_space(4.0);
                ui.label(StudioTheme::muted_for(mode, "Initial Conditions"));
                for (node, value) in &self.simulation_profile_editor.initial_conditions {
                    if !node.trim().is_empty() {
                        profile_row(ui, mode, &format!(".ic {}", node), value, "ic");
                    }
                }
            }

            // Vendor models
            let vendor_count = self.vendor_catalog.total_count();
            let added_models = self.simulation_profile_editor.model_params.len();
            if vendor_count > 0 {
                ui.add_space(4.0);
                ui.separator();
                ui.add_space(4.0);
                ui.label(StudioTheme::muted_for(mode, "Vendor Models"));
                profile_row(ui, mode, "Available", &format!("{} models", vendor_count), "catalog");
                if added_models > 0 {
                    profile_row(ui, mode, "Added to profile", &format!("{} models", added_models), "active");
                }
            }

            // Backend engine
            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);
            profile_row(
                ui,
                mode,
                self.text(UiText::Backend),
                self.simulation_panel.backend.label(),
                "engine",
            );
        });
    }
}
