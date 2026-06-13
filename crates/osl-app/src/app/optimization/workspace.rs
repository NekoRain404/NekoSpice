//! Optimization workspace — center panel for parametric optimization and yield analysis.

use crate::app::NekoSpiceApp;
use crate::app::localization::UiText;
use super::widgets::metric_card;
use crate::app::theme::StudioTheme;
use eframe::egui;

impl NekoSpiceApp {
    /// draw optimization center workspace。
    pub(crate) fn draw_optimization_center_workspace(&mut self, ui: &mut egui::Ui) {
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
                            if ui.button(self.text(UiText::StartSweep)).clicked() {
                        self.active_workspace = crate::app::navigation::StudioWorkspace::Simulation;
                        self.status_message = Some("Parametric sweep: configure in Simulation workspace".to_string());
                    }
                        });
                    });
                    ui.add_space(10.0);
                    self.draw_optimization_tabs(ui);
                    ui.add_space(10.0);
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
                            ui.add_space(10.0);
                            self.draw_optimization_summary_panel(ui);
                        });
                    }
                });
        });
    }

    /// draw optimization workspace panel。
    pub(crate) fn draw_optimization_workspace_panel(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        ui.heading(self.text(UiText::OptimizationStudio));
        ui.label(StudioTheme::muted_for(
            mode,
            self.text(UiText::OptimizationCaption),
        ));
        ui.add_space(10.0);
        let run_count = self.simulation_panel.last_run.as_ref()
            .map(|r| r.metadata.parameters.len().max(1).to_string())
            .unwrap_or_else(|| "0".to_string());
        let status = self.simulation_panel.last_run.as_ref()
            .map(|r| r.metadata.status.as_str().to_string())
            .unwrap_or_else(|| self.text(UiText::Queued).to_string());
        metric_card(ui, mode, self.text(UiText::Yield),
            &format!("{} runs", run_count), self.text(UiText::MonteCarlo));
        metric_card(ui, mode, self.text(UiText::Constraints),
            &status, self.text(UiText::Ready));
    }
}
