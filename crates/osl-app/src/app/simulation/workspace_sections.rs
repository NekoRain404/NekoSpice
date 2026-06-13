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
use super::workspace_widgets::{analysis_mode_button, analysis_modes, code_preview_line};
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
}
