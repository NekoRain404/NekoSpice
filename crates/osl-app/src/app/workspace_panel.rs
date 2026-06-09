use super::navigation::StudioWorkspace;
use super::{NekoSpiceApp, theme::StudioTheme};
use eframe::egui;

impl NekoSpiceApp {
    pub(super) fn draw_right_workspace_panel(&mut self, ui: &mut egui::Ui) {
        match self.active_workspace {
            StudioWorkspace::Schematic => self.draw_schematic_workspace_panel(ui),
            StudioWorkspace::Library => self.draw_library_browser(ui),
            StudioWorkspace::Simulation => self.draw_simulation_panel(ui),
            StudioWorkspace::Reports => self.draw_reports_workspace_panel(ui),
        }
    }

    fn draw_schematic_workspace_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Schematic Tools");
        ui.label(StudioTheme::muted(
            "Place wires, labels, buses, sheets, and markers.",
        ));
        ui.add_space(8.0);
        StudioTheme::panel_frame().show(ui, |ui| {
            self.draw_schematic_tool_controls(ui);
        });
        ui.add_space(8.0);
        self.draw_document_diagnostics_panel(ui, 220.0);
    }

    fn draw_reports_workspace_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Reports & Results");
        ui.label(StudioTheme::muted(
            "Latest run artifacts, generated HTML reports, and waveform previews.",
        ));
        ui.add_space(8.0);
        self.draw_simulation_panel(ui);
    }
}
