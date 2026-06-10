use super::NekoSpiceApp;
use super::localization::UiText;
use super::optimization_workspace_widgets::metric_card;
use super::theme::StudioTheme;
use eframe::egui;

impl NekoSpiceApp {
    pub(super) fn draw_optimization_center_workspace(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("optimization_center_scroll")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.horizontal_top(|ui| {
                        ui.vertical(|ui| {
                            ui.heading(self.text(UiText::OptimizationStudio));
                            ui.label(StudioTheme::muted_for(
                                mode,
                                self.text(UiText::OptimizationCaption),
                            ));
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                            let _ = ui.button(self.text(UiText::StartSweep));
                        });
                    });
                    ui.add_space(8.0);
                    self.draw_optimization_tabs(ui);
                    ui.add_space(8.0);
                    if ui.available_width() >= 820.0 {
                        ui.horizontal_top(|ui| {
                            ui.vertical(|ui| {
                                ui.set_width((ui.available_width() * 0.58).max(420.0));
                                self.draw_optimization_main_panel(ui);
                            });
                            ui.add_space(10.0);
                            ui.vertical(|ui| {
                                ui.set_width(ui.available_width().max(240.0));
                                self.draw_optimization_summary_panel(ui);
                            });
                        });
                    } else {
                        ui.vertical(|ui| {
                            self.draw_optimization_main_panel(ui);
                            ui.add_space(8.0);
                            self.draw_optimization_summary_panel(ui);
                        });
                    }
                });
        });
    }

    pub(super) fn draw_optimization_workspace_panel(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        ui.heading(self.text(UiText::OptimizationStudio));
        ui.label(StudioTheme::muted_for(
            mode,
            self.text(UiText::OptimizationCaption),
        ));
        ui.add_space(8.0);
        metric_card(
            ui,
            mode,
            self.text(UiText::Yield),
            "97.6%",
            self.text(UiText::MonteCarlo),
        );
        metric_card(
            ui,
            mode,
            self.text(UiText::Constraints),
            "6",
            self.text(UiText::Ready),
        );
    }
}
