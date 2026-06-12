//! Simulation workspace overview sections:
//!
//! - Analysis setup panel (analysis type selection + structured params)
//! - Netlist preview with syntax highlighting
//! - Run output and diagnostics
//! - Profile summary showing all configured simulation settings
//!
//! The overview provides a high-level view of the simulation configuration
//! and results, while the Profile Editor provides detailed editing.

use crate::app::NekoSpiceApp;
use crate::app::localization::UiText;
use super::state::AnalysisParams;
use super::workspace_widgets::{analysis_mode_button, code_preview_line, profile_row};
use crate::app::status_strip::severity_color;
use crate::app::theme::StudioTheme;
use eframe::egui;
use osl_kicad::KicadDiagnosticSeverity;

impl NekoSpiceApp {
    /// Draw simulation analysis setup with structured parameter fields.
    pub(crate) fn draw_simulation_analysis_setup(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::AnalysisSetup),
            ));
            // Analysis mode buttons in a 2x2 grid
            for row in analysis_modes().chunks(2) {
                ui.columns(2, |columns| {
                    for (column, (kind, title, caption)) in row.iter().enumerate() {
                        let active = self.simulation_panel.directive_kind == *kind;
                        if analysis_mode_button(&mut columns[column], mode, title, caption, active)
                        {
                            if self.simulation_panel.directive_kind != *kind {
                                self.simulation_panel.analysis_params = AnalysisParams::for_kind(*kind);
                            }
                            self.simulation_panel.directive_kind = *kind;
                        }
                    }
                });
                ui.add_space(6.0);
            }
            ui.separator();
            // Directive editor with structured fields
            self.draw_simulation_directive_editor(ui);
        });
    }

    /// Draw simulation netlist preview with line numbers.
    pub(crate) fn draw_simulation_netlist_preview(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(StudioTheme::section_title_for(
                    mode,
                    self.text(UiText::NetlistPreview),
                ));
                if ui.small_button("Export .cir").on_hover_text("Save netlist to file").clicked() {
                    self.export_netlist_dialog();
                }
            });
            let Some(document) = &self.document else {
                ui.label(StudioTheme::muted_for(
                    mode,
                    self.text(UiText::NoEditableSchematicLoaded),
                ));
                return;
            };
            let profile = self.build_simulation_profile();
            let netlist_result = document.spice_netlist_preview().map(|netlist| {
                let netlist = osl_sim::inject_profile_directives(&netlist, &profile);
                if self.simulation_panel.backend == super::state::SimulationBackendKind::Xyce {
                    osl_sim::prepare_xyce_netlist_display(&netlist)
                } else {
                    netlist
                }
            });
            match netlist_result {
                Ok(netlist) => {
                    let line_count = netlist.lines().count();
                    ui.label(StudioTheme::muted_for(
                        mode,
                        format!("{} lines, {} backend", line_count, self.simulation_panel.backend.label()),
                    ));
                    egui::ScrollArea::both()
                        .id_salt("simulation_center_netlist_preview")
                        .max_height(280.0)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            for (index, line) in netlist.lines().enumerate() {
                                code_preview_line(ui, index + 1, line);
                            }
                        });
                }
                Err(error) => {
                    ui.colored_label(severity_color(mode, KicadDiagnosticSeverity::Error), error);
                }
            }
        });
    }

    /// Draw simulation run output and diagnostics.
    pub(crate) fn draw_simulation_run_output(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::RunOutput),
            ));
            self.draw_simulation_run_status(ui);
            if self.simulation_panel.last_run.is_none()
                && self.simulation_panel.last_error.is_none()
                && self.simulation_panel.active_task.is_none()
            {
                ui.label(StudioTheme::muted_for(mode, self.text(UiText::NoRecentRun)));
            }
        });
    }

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

            // Step sweep
            if let super::state::StepSweep::Parametric { param_name, sweep_mode, start, stop, step } = &self.simulation_panel.step_sweep {
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

/// Analysis modes available in the overview.
fn analysis_modes() -> [(
    osl_kicad::KicadSimulationDirectiveKind,
    &'static str,
    &'static str,
); 7] {
    [
        (
            osl_kicad::KicadSimulationDirectiveKind::Op,
            ".op",
            "operating point",
        ),
        (osl_kicad::KicadSimulationDirectiveKind::Dc, ".dc", "sweep"),
        (
            osl_kicad::KicadSimulationDirectiveKind::Tran,
            ".tran",
            "time domain",
        ),
        (
            osl_kicad::KicadSimulationDirectiveKind::Ac,
            ".ac",
            "small signal",
        ),
        (
            osl_kicad::KicadSimulationDirectiveKind::Noise,
            ".noise",
            "noise analysis",
        ),
        (
            osl_kicad::KicadSimulationDirectiveKind::Disto,
            ".disto",
            "distortion",
        ),
        (
            osl_kicad::KicadSimulationDirectiveKind::Sens,
            ".sens",
            "sensitivity",
        ),
    ]
}
