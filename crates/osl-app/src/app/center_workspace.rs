use super::NekoSpiceApp;
use super::navigation::StudioWorkspace;
use eframe::egui;

impl NekoSpiceApp {
    pub(super) fn draw_center_workspace(&mut self, ui: &mut egui::Ui) {
        match self.active_workspace {
            StudioWorkspace::Home => self.draw_home_dashboard(ui),
            StudioWorkspace::Schematic => self.draw_schematic_center_workspace(ui),
            StudioWorkspace::Library => self.draw_library_center_workspace(ui),
            StudioWorkspace::Simulation => self.draw_simulation_center_workspace(ui),
            StudioWorkspace::Waveforms => self.draw_waveform_center_workspace(ui),
            StudioWorkspace::Reports => self.draw_reports_center_workspace(ui),
            StudioWorkspace::Settings => self.draw_studio_canvas_frame(ui),
        }
    }
}
