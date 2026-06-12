use super::NekoSpiceApp;
use super::localization::UiText;
use super::simulation_workspace_widgets::{analysis_mode_button, code_preview_line, profile_row};
use super::status_strip::severity_color;
use super::theme::StudioTheme;
use eframe::egui;
use osl_kicad::KicadDiagnosticSeverity;

const NETLIST_CENTER_PREVIEW_LINES: usize = 18;

impl NekoSpiceApp {
    pub(super) fn draw_simulation_analysis_setup(&mut self, ui: &mut egui::Ui) {
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

    pub(super) fn draw_simulation_netlist_preview(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::NetlistPreview),
            ));
            let Some(document) = &self.document else {
                ui.label(StudioTheme::muted_for(
                    mode,
                    self.text(UiText::NoEditableSchematicLoaded),
                ));
                return;
            };
            match document.spice_netlist_preview() {
                Ok(netlist) => {
                    egui::ScrollArea::both()
                        .id_salt("simulation_center_netlist_preview")
                        .max_height(300.0)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            for (index, line) in netlist
                                .lines()
                                .take(NETLIST_CENTER_PREVIEW_LINES)
                                .enumerate()
                            {
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

    pub(super) fn draw_simulation_run_output(&mut self, ui: &mut egui::Ui) {
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

    fn draw_simulation_profile_grid(&self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        ui.label(StudioTheme::section_title_for(
            mode,
            self.text(UiText::SimulationProfile),
        ));
        profile_row(ui, mode, self.text(UiText::StopTime), "1 ms", "tran");
        profile_row(ui, mode, self.text(UiText::StepSize), "1 us", "max");
        profile_row(ui, mode, self.text(UiText::Tolerance), "1e-6", "solver");
        profile_row(
            ui,
            mode,
            self.text(UiText::TemperatureSweep),
            "27 C",
            "nominal",
        );
        profile_row(
            ui,
            mode,
            self.text(UiText::OutputArtifacts),
            "raw/csv/html",
            "on",
        );
        profile_row(
            ui,
            mode,
            self.text(UiText::Backend),
            self.simulation_panel.backend.label(),
            "wgpu UI",
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
