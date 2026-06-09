use super::NekoSpiceApp;
use super::home_widgets::two_column;
use super::localization::UiText;
use super::navigation::StudioWorkspace;
use super::theme::StudioTheme;
use eframe::egui;

pub(super) const SECTION_GAP: f32 = 10.0;

impl NekoSpiceApp {
    pub(super) fn draw_home_dashboard(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        egui::ScrollArea::vertical()
            .id_salt("studio_home_dashboard_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.heading(self.text(UiText::HomeTitle));
                        ui.label(StudioTheme::muted_for(
                            mode,
                            self.text(UiText::HomeSubtitle),
                        ));
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(self.text(UiText::OpenSchematic)).clicked() {
                            self.active_workspace = StudioWorkspace::Schematic;
                        }
                    });
                });
                ui.add_space(SECTION_GAP);

                two_column(
                    self,
                    ui,
                    0.46,
                    |app, ui| app.draw_recent_projects_panel(ui),
                    |app, ui| {
                        app.draw_quick_actions_panel(ui);
                    },
                );
                ui.add_space(SECTION_GAP);
                self.draw_template_row(ui);
                ui.add_space(SECTION_GAP);

                two_column(
                    self,
                    ui,
                    0.50,
                    |app, ui| app.draw_simulation_queue_panel(ui),
                    |app, ui| {
                        app.draw_solver_health_panel(ui);
                    },
                );
                ui.add_space(SECTION_GAP);
                two_column(
                    self,
                    ui,
                    0.50,
                    |app, ui| app.draw_recent_measurements_panel(ui),
                    |app, ui| {
                        app.draw_recommendations_panel(ui);
                    },
                );
            });
    }
}
