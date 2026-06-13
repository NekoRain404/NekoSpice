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
                    ui.label(egui::RichText::new(self.simulation_panel.backend.label()).monospace());
                    ui.end_row();

                    ui.label(StudioTheme::muted_for(mode, "Analysis"));
                    ui.label(egui::RichText::new(format!(".{}", self.simulation_panel.directive_kind)).monospace());
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
                        super::state::StepSweep::Parametric { param_name, sweep_mode, .. } => {
                            ui.label(StudioTheme::muted_for(mode, "Step"));
                            ui.label(
                                egui::RichText::new(format!(".step {} {}", param_name, sweep_mode))
                                    .monospace()
                                    .color(self.theme_palette().accent),
                            );
                            ui.end_row();
                        }
                        super::state::StepSweep::Temperature { sweep_mode, start, stop, .. } => {
                            ui.label(StudioTheme::muted_for(mode, "Step"));
                            ui.label(
                                egui::RichText::new(format!(".step TEMP {} {}–{}", sweep_mode, start, stop))
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
                            egui::RichText::new(format!("{} directive(s)", self.simulation_measurements.len()))
                                .monospace(),
                        );
                        ui.end_row();
                    }

                    let ic_count = self.simulation_profile_editor.initial_conditions.len()
                        + self.simulation_profile_editor.nodesets.len();
                    if ic_count > 0 {
                        ui.label(StudioTheme::muted_for(mode, ".ic/.ns"));
                        ui.label(
                            egui::RichText::new(format!("{} entry(ies)", ic_count))
                                .monospace(),
                        );
                        ui.end_row();
                    }

                    let comp_count = self.simulation_profile_editor.component_params.len();
                    if comp_count > 0 {
                        ui.label(StudioTheme::muted_for(mode, "Components"));
                        ui.label(
                            egui::RichText::new(format!("{} defined", comp_count))
                                .monospace(),
                        );
                        ui.end_row();
                    }

                    let model_count = self.simulation_profile_editor.model_params.len();
                    if model_count > 0 {
                        ui.label(StudioTheme::muted_for(mode, "Models"));
                        ui.label(
                            egui::RichText::new(format!("{} defined", model_count))
                                .monospace(),
                        );
                        ui.end_row();
                    }
                });
        });
    }

    /// Collapsible netlist preview in the sidebar panel.
    pub(crate) fn draw_panel_netlist_preview(&self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let palette = StudioTheme::palette(mode);
        egui::CollapsingHeader::new(
            egui::RichText::new("Netlist Preview").color(palette.text),
        )
        .id_salt("panel_netlist_preview")
        .default_open(false)
        .show(ui, |ui| {
            let Some(document) = &self.document else {
                ui.label(StudioTheme::muted_for(mode, "No schematic loaded"));
                return;
            };
            let profile = self.build_simulation_profile();
            match document.spice_netlist_preview().map(|raw| {
                osl_sim::inject_profile_directives(&raw, &profile)
            }) {
                Ok(netlist) => {
                    let line_count = netlist.lines().count();
                    ui.label(StudioTheme::muted_for(
                        mode,
                        format!("{} lines — {} backend", line_count, self.simulation_panel.backend.label()),
                    ));
                    ui.add_space(2.0);
                    egui::ScrollArea::vertical()
                        .id_salt("panel_netlist_scroll")
                        .max_height(120.0)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            for line in netlist.lines().take(30) {
                                ui.monospace(egui::RichText::new(line).size(10.0).color(palette.text_muted));
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
