use crate::app::NekoSpiceApp;
use crate::app::localization::UiText;
use super::workspace_widgets::{analysis_mode_button, code_preview_line, profile_row};
use crate::app::status_strip::severity_color;
use crate::app::theme::StudioTheme;
use eframe::egui;
use osl_kicad::KicadDiagnosticSeverity;

const NETLIST_CENTER_PREVIEW_LINES: usize = 18;

impl NekoSpiceApp {
    /// draw simulation analysis setup。
    pub(crate) fn draw_simulation_analysis_setup(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::AnalysisSetup),
            ));
            for row in analysis_modes().chunks(2) {
                ui.columns(2, |columns| {
                    for (column, (kind, title, caption)) in row.iter().enumerate() {
                        let active = self.simulation_panel.directive_kind == *kind;
                        if analysis_mode_button(&mut columns[column], mode, title, caption, active)
                        {
                            self.simulation_panel.directive_kind = *kind;
                        }
                    }
                });
                ui.add_space(6.0);
            }
            ui.separator();
            self.draw_simulation_directive_editor(ui);
            ui.separator();
            self.draw_simulation_profile_grid(ui);
        });
    }

    /// draw simulation netlist preview。
    pub(crate) fn draw_simulation_netlist_preview(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(StudioTheme::section_title_for(
                    mode,
                    self.text(UiText::NetlistPreview),
                ));
                if ui.small_button("Export .cir").on_hover_text("Save netlist to file").clicked() {
                    if let Some(document) = &self.document {
                        let profile = self.build_simulation_profile();
                        if let Ok(raw) = document.spice_netlist_preview() {
                            let netlist = osl_sim::inject_profile_directives(&raw, &profile);
                            let dialog = rfd::FileDialog::new()
                                .add_filter("SPICE Netlist", &["cir", "sp", "net"])
                                .set_file_name("schematic.cir");
                            if let Some(path) = dialog.save_file() {
                                let _ = std::fs::write(&path, &netlist);
                                self.status_message = Some(format!("Netlist exported to {}", path.display()));
                            }
                        }
                    }
                }
            });
            let Some(document) = &self.document else {
                ui.label(StudioTheme::muted_for(
                    mode,
                    self.text(UiText::NoEditableSchematicLoaded),
                ));
                return;
            };
            // Build profile and inject directives to show the ACTUAL netlist
            // that will be sent to the solver (not just the raw schematic netlist)
            let profile = self.build_simulation_profile();
            let netlist_result = document.spice_netlist_preview().map(|netlist| {
                let netlist = osl_sim::inject_profile_directives(&netlist, &profile);
                if self.simulation_panel.backend == super::panel::SimulationBackendKind::Xyce {
                    osl_sim::prepare_xyce_netlist_display(&netlist)
                } else {
                    netlist
                }
            });
            match netlist_result {
                Ok(netlist) => {
                    egui::ScrollArea::both()
                        .id_salt("simulation_center_netlist_preview")
                        .max_height(300.0)
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

    /// draw simulation run output。
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

    fn draw_simulation_profile_grid(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        ui.label(StudioTheme::section_title_for(
            mode,
            self.text(UiText::SimulationProfile),
        ));
        // Show actual analysis directive and profile state
        let analysis = format!("{} {}",
            self.simulation_panel.directive_kind.to_string(),
            self.simulation_panel.directive_body.trim()
        ).trim().to_string();
        let temp = self.simulation_profile_editor.options.temperature.clone();
        let tol = self.simulation_profile_editor.options.reltol.clone();
        let method = self.simulation_profile_editor.options.method.clone();
        profile_row(ui, mode, "Analysis", &analysis, "active");
        profile_row(ui, mode, self.text(UiText::TemperatureSweep), &format!("{} C", temp), "nominal");
        profile_row(ui, mode, self.text(UiText::Tolerance), &tol, "rel");
        profile_row(ui, mode, "Method", &method, "integration");
        profile_row(
            ui,
            mode,
            self.text(UiText::Backend),
            self.simulation_panel.backend.label(),
            "engine",
        );
    }
}

fn analysis_modes() -> [(
    osl_kicad::KicadSimulationDirectiveKind,
    &'static str,
    &'static str,
); 4] {
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
    ]
}
