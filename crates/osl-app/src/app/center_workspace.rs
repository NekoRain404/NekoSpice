use super::NekoSpiceApp;
use super::navigation::StudioWorkspace;
use eframe::egui;

impl NekoSpiceApp {
    pub(super) fn draw_center_workspace(&mut self, ui: &mut egui::Ui) {
        match self.active_workspace {
            StudioWorkspace::Home => self.draw_home_dashboard(ui),
            StudioWorkspace::Schematic => self.draw_schematic_center_workspace(ui),
            StudioWorkspace::Library
            | StudioWorkspace::Simulation
            | StudioWorkspace::Reports
            | StudioWorkspace::Settings => self.draw_studio_canvas_frame(ui),
        }
    }
}
