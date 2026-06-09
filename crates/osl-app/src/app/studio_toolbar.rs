use super::navigation::StudioWorkspace;
use super::{NekoSpiceApp, theme::StudioTheme};
use eframe::egui;

impl NekoSpiceApp {
    pub(super) fn draw_studio_top_bar(&mut self, ui: &mut egui::Ui) {
        self.draw_top_status_strip(ui);
        ui.separator();
        if ui
            .add_enabled(self.document.is_some(), egui::Button::new("Save"))
            .on_hover_text("Save the active KiCad schematic")
            .clicked()
        {
            self.save_document();
        }
        if ui
            .button("Fit")
            .on_hover_text("Fit the schematic to the canvas")
            .clicked()
        {
            self.viewport
                .fit_scene(self.scene.as_ref().and_then(|scene| scene.bounds));
        }
        if ui
            .add_enabled(self.document.is_some(), egui::Button::new("Run"))
            .on_hover_text("Run ngspice for the active schematic")
            .clicked()
        {
            self.run_simulation_from_panel();
            self.active_workspace = StudioWorkspace::Simulation;
        }
    }

    pub(super) fn draw_studio_canvas_frame(&mut self, ui: &mut egui::Ui) {
        StudioTheme::panel_frame().show(ui, |ui| {
            self.draw_canvas(ui);
        });
    }
}
