//! 中央工作区内容调度。在左侧面板和右侧内容区之间分配空间。
//!
use super::NekoSpiceApp;
use super::navigation::StudioWorkspace;
use eframe::egui;

impl NekoSpiceApp {
    /// draw center workspace。
    pub(super) fn draw_center_workspace(&mut self, ui: &mut egui::Ui) {
        match self.active_workspace {
            StudioWorkspace::Home => self.draw_home_dashboard(ui),
            StudioWorkspace::Schematic => self.draw_schematic_center_workspace(ui),
            StudioWorkspace::Library => self.draw_library_center_workspace(ui),
            StudioWorkspace::Simulation => self.draw_simulation_center_workspace(ui),
            StudioWorkspace::Optimization => self.draw_optimization_center_workspace(ui),
            StudioWorkspace::Review => self.draw_review_center_workspace(ui),
            StudioWorkspace::Waveforms => self.draw_waveform_center_workspace(ui),
            StudioWorkspace::Reports => self.draw_reports_center_workspace(ui),
            StudioWorkspace::Settings => self.draw_settings_center_workspace(ui),
        }
    }
}
