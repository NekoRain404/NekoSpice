use super::NekoSpiceApp;
use super::theme::StudioTheme;
use eframe::egui;

impl eframe::App for NekoSpiceApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        super::theme::StudioTheme::apply(ui.ctx());
        egui::CentralPanel::default()
            .frame(StudioTheme::page_frame())
            .show_inside(ui, |ui| {
                egui::Panel::top("nekospice_top_status")
                    .exact_size(58.0)
                    .frame(StudioTheme::strip_frame())
                    .show_inside(ui, |ui| {
                        self.draw_studio_top_bar(ui);
                    });
                egui::Panel::bottom("nekospice_bottom_status")
                    .exact_size(32.0)
                    .frame(StudioTheme::strip_frame())
                    .show_inside(ui, |ui| {
                        self.draw_bottom_status_strip(ui);
                    });
                egui::Panel::left("nekospice_navigation")
                    .exact_size(190.0)
                    .frame(StudioTheme::strip_frame())
                    .show_inside(ui, |ui| {
                        self.draw_workspace_navigation(ui);
                    });
                egui::Panel::left("nekospice_project_context")
                    .default_size(280.0)
                    .min_size(220.0)
                    .max_size(360.0)
                    .frame(StudioTheme::page_frame())
                    .show_inside(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .id_salt("studio_project_context")
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                ui.add_space(8.0);
                                self.draw_project_sidebar(ui);
                            });
                    });
                egui::Panel::right("nekospice_workspace_context")
                    .default_size(360.0)
                    .min_size(280.0)
                    .max_size(520.0)
                    .frame(StudioTheme::page_frame())
                    .show_inside(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .id_salt("studio_workspace_context")
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                ui.add_space(8.0);
                                self.draw_right_workspace_panel(ui);
                            });
                    });
                egui::CentralPanel::default()
                    .frame(StudioTheme::page_frame())
                    .show_inside(ui, |ui| {
                        self.draw_studio_canvas_frame(ui);
                    });
            });
    }
}
