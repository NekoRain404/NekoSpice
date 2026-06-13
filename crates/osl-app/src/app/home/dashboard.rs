//! Home workspace dashboard — project overview, quick actions, and command center.

use crate::app::NekoSpiceApp;
use super::widgets::two_column;
use crate::app::localization::UiText;
use crate::app::navigation::StudioWorkspace;
use crate::app::theme::StudioTheme;
use eframe::egui;

/// `SECTION_GAP` 常量。
pub(crate) const SECTION_GAP: f32 = 10.0;

impl NekoSpiceApp {
    /// draw home dashboard。
    pub(crate) fn draw_home_dashboard(&mut self, ui: &mut egui::Ui) {
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
                ui.add_space(SECTION_GAP + 4.0);
                self.draw_home_command_center(ui);
                ui.add_space(SECTION_GAP + 4.0);

                two_column(
                    self,
                    ui,
                    0.46,
                    |app, ui| app.draw_recent_projects_panel(ui),
                    |app, ui| {
                        app.draw_quick_actions_panel(ui);
                    },
                );
                ui.add_space(SECTION_GAP + 4.0);
                self.draw_template_row(ui);
                ui.add_space(SECTION_GAP + 4.0);

                two_column(
                    self,
                    ui,
                    0.50,
                    |app, ui| app.draw_simulation_queue_panel(ui),
                    |app, ui| {
                        app.draw_solver_health_panel(ui);
                    },
                );
                ui.add_space(SECTION_GAP + 4.0);
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
