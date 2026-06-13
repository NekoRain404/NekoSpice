//! Sidebar panel sections — configuration summary and netlist preview.
//!
//! Extracted from panel.rs to keep individual files under 300 lines.

use crate::app::NekoSpiceApp;
use crate::app::theme::StudioTheme;
use eframe::egui;

impl NekoSpiceApp {
    /// Compact configuration summary for the sidebar panel.
    /// Shows the key simulation settings at a glance.
    pub(crate) fn draw_panel_config_summary(&self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let opts = &self.simulation_profile_editor.options;
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(mode, "Quick Settings"));
            ui.add_space(4.0);
            egui::Grid::new("panel_config_summary")
                .num_columns(2)
                .spacing([8.0, 4.0])
                .show(ui, |ui| {
                    ui.label(StudioTheme::muted_for(mode, "Backend"));
                    ui.label(
                        egui::RichText::new(self.simulation_panel.backend.label()).monospace(),
                    );
                    ui.end_row();

                    ui.label(StudioTheme::muted_for(mode, "Analysis"));
                    ui.label(
                        egui::RichText::new(format!(".{}", self.simulation_panel.directive_kind))
                            .monospace(),
                    );
                    ui.end_row();

                    ui.label(StudioTheme::muted_for(mode, "Temp"));
                    ui.label(egui::RichText::new(format!("{} °C", opts.temperature)).monospace());
                    ui.end_row();

                    ui.label(StudioTheme::muted_for(mode, "Method"));
                    ui.label(egui::RichText::new(&opts.method).monospace());
                    ui.end_row();

                    ui.label(StudioTheme::muted_for(mode, "RELTOL"));
                    ui.label(egui::RichText::new(&opts.reltol).monospace());
                    ui.end_row();

                    if self.simulation_profile_editor.active_preset != "default" {
                        ui.label(StudioTheme::muted_for(mode, "Preset"));
                        ui.label(
                            egui::RichText::new(&self.simulation_profile_editor.active_preset)
                                .monospace()
                                .color(self.theme_palette().accent),
                        );
                        ui.end_row();
                    }

                    match &self.simulation_panel.step_sweep {
                        super::state::StepSweep::Parametric {
                            param_name,
                            sweep_mode,
                            ..
                        } => {
                            ui.label(StudioTheme::muted_for(mode, "Step"));
                            ui.label(
                                egui::RichText::new(format!(".step {} {}", param_name, sweep_mode))
                                    .monospace()
                                    .color(self.theme_palette().accent),
                            );
                            ui.end_row();
                        }
                        super::state::StepSweep::Temperature {
                            sweep_mode,
                            start,
                            stop,
                            ..
                        } => {
                            ui.label(StudioTheme::muted_for(mode, "Step"));
                            ui.label(
                                egui::RichText::new(format!(
                                    ".step TEMP {} {}–{}",
                                    sweep_mode, start, stop
                                ))
                                .monospace()
                                .color(self.theme_palette().accent),
                            );
                            ui.end_row();
                        }
                        super::state::StepSweep::None => {}
                    }

                    if !self.simulation_measurements.is_empty() {
                        ui.label(StudioTheme::muted_for(mode, "Measures"));
                        ui.label(
                            egui::RichText::new(format!(
                                "{} directive(s)",
                                self.simulation_measurements.len()
                            ))
                            .monospace(),
                        );
                        ui.end_row();
                    }

                    let ic_count = self.simulation_profile_editor.initial_conditions.len()
                        + self.simulation_profile_editor.nodesets.len();
                    if ic_count > 0 {
                        ui.label(StudioTheme::muted_for(mode, ".ic/.ns"));
                        ui.label(
                            egui::RichText::new(format!("{} entry(ies)", ic_count)).monospace(),
                        );
                        ui.end_row();
                    }

                    let comp_count = self.simulation_profile_editor.component_params.len();
                    if comp_count > 0 {
                        ui.label(StudioTheme::muted_for(mode, "Components"));
                        ui.label(
                            egui::RichText::new(format!("{} defined", comp_count)).monospace(),
                        );
                        ui.end_row();
                    }

                    let model_count = self.simulation_profile_editor.model_params.len();
                    if model_count > 0 {
                        ui.label(StudioTheme::muted_for(mode, "Models"));
                        ui.label(
                            egui::RichText::new(format!("{} defined", model_count)).monospace(),
                        );
                        ui.end_row();
                    }
                });
        });
    }

    /// Netlist validation status indicator — shows pass/warning/error
    /// for the current netlist configuration.
    pub(crate) fn draw_panel_validation_status(&self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let palette = StudioTheme::palette(mode);

        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(mode, "Validation Status"));
            ui.add_space(4.0);

            if self.document.is_none() {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("●")
                            .color(palette.text_muted)
                            .size(10.0),
                    );
                    ui.label(StudioTheme::muted_for(mode, "No schematic loaded"));
                });
                return;
            }

            // Check netlist validity
            let profile = self.build_simulation_profile();
            let netlist_ok = self.document.as_ref().map(|doc| {
                doc.spice_netlist_preview()
                    .map(|raw| nsp_sim::inject_profile_directives(&raw, &profile))
            });

            match netlist_ok {
                Some(Ok(netlist)) => {
                    let warnings = nsp_sim::validate_netlist_for_simulation(&netlist);
                    if warnings.is_empty() {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("●").color(palette.success).size(10.0));
                            ui.label(egui::RichText::new("Netlist valid").color(palette.success));
                        });
                    } else {
                        for w in &warnings {
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new("●").color(palette.warning).size(10.0),
                                );
                                ui.label(StudioTheme::muted_for(mode, w));
                            });
                        }
                    }
                    // Show backend-specific status
                    ui.add_space(2.0);
                    ui.horizontal(|ui| {
                        let backend_color = match self.simulation_panel.backend {
                            super::state::SimulationBackendKind::Ngspice => palette.accent,
                            super::state::SimulationBackendKind::Xyce => palette.success,
                        };
                        ui.label(egui::RichText::new("●").color(backend_color).size(10.0));
                        ui.label(StudioTheme::muted_for(
                            mode,
                            format!("{} backend ready", self.simulation_panel.backend.label()),
                        ));
                    });
                }
                Some(Err(err)) => {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("●").color(palette.danger).size(10.0));
                        ui.label(StudioTheme::muted_for(mode, &err));
                    });
                }
                None => {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("●")
                                .color(palette.text_muted)
                                .size(10.0),
                        );
                        ui.label(StudioTheme::muted_for(mode, "No document"));
                    });
                }
            }
        });
    }

    /// Simulation readiness indicator — shows what's needed before running.
    pub(crate) fn draw_simulation_readiness(&self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let palette = StudioTheme::palette(mode);

        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(mode, "Readiness"));
            ui.add_space(4.0);

            let has_doc = self.document.is_some();
            let has_analysis = !self
                .simulation_panel
                .analysis_params
                .to_body()
                .trim()
                .is_empty();
            let running = self.simulation_panel.active_task.is_some();
            let _has_netlist = self.simulation_panel.last_error.is_none()
                || self.simulation_panel.last_run.is_some();

            let items = [
                ("Schematic loaded", has_doc),
                ("Analysis configured", has_analysis),
                ("Netlist ready", has_doc && has_analysis),
            ];

            for (label, ok) in items {
                ui.horizontal(|ui| {
                    let color = if ok {
                        palette.success
                    } else {
                        palette.text_muted
                    };
                    let icon = if ok { "✓" } else { "○" };
                    ui.label(egui::RichText::new(icon).color(color).size(12.0).strong());
                    ui.label(egui::RichText::new(label).color(if ok {
                        palette.text
                    } else {
                        palette.text_muted
                    }));
                });
            }

            if running {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("▶").color(palette.accent).size(12.0));
                    ui.label(
                        egui::RichText::new("Simulation running...")
                            .color(palette.accent)
                            .strong(),
                    );
                });
            }

            let all_ready = has_doc && has_analysis && !running;
            ui.add_space(4.0);
            if all_ready {
                ui.label(StudioTheme::muted_for(
                    mode,
                    "Ready to simulate — press F5 or click Run",
                ));
            } else if !has_doc {
                ui.label(StudioTheme::muted_for(mode, "Load a schematic to begin"));
            } else if !has_analysis {
                ui.label(StudioTheme::muted_for(
                    mode,
                    "Configure analysis parameters",
                ));
            }
        });
    }

    /// Collapsible netlist preview in the sidebar panel.
    pub(crate) fn draw_panel_netlist_preview(&self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let palette = StudioTheme::palette(mode);
        egui::CollapsingHeader::new(egui::RichText::new("Netlist Preview").color(palette.text))
            .id_salt("panel_netlist_preview")
            .default_open(false)
            .show(ui, |ui| {
                let Some(document) = &self.document else {
                    ui.label(StudioTheme::muted_for(mode, "No schematic loaded"));
                    return;
                };
                let profile = self.build_simulation_profile();
                match document
                    .spice_netlist_preview()
                    .map(|raw| nsp_sim::inject_profile_directives(&raw, &profile))
                {
                    Ok(netlist) => {
                        let line_count = netlist.lines().count();
                        ui.label(StudioTheme::muted_for(
                            mode,
                            format!(
                                "{} lines — {} backend",
                                line_count,
                                self.simulation_panel.backend.label()
                            ),
                        ));
                        ui.add_space(2.0);
                        egui::ScrollArea::vertical()
                            .id_salt("panel_netlist_scroll")
                            .max_height(120.0)
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                for line in netlist.lines().take(30) {
                                    ui.monospace(
                                        egui::RichText::new(line)
                                            .size(10.0)
                                            .color(palette.text_muted),
                                    );
                                }
                                if line_count > 30 {
                                    ui.label(StudioTheme::muted_for(
                                        mode,
                                        format!("... {} more lines", line_count - 30),
                                    ));
                                }
                            });
                    }
                    Err(error) => {
                        ui.colored_label(palette.danger, format!("Netlist error: {}", error));
                    }
                }
            });
    }
}
