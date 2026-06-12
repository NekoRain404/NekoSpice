//! Simulation right panel — thin orchestrator that composes the directive
//! editor, run controller, and status display into the sidebar panel.
//!
//! The heavy lifting lives in sibling modules:
//! - `state` — SimulationBackendKind, SimulationPanelState
//! - `directive_editor` — directive kind/body editing UI
//! - `run_controller` — profile building, run launch, task polling
//! - `status_display` — run results, log viewer, waveform summary

use crate::app::NekoSpiceApp;
use crate::app::localization::UiText;
use eframe::egui;
use super::state::SimulationBackendKind;

impl NekoSpiceApp {
    /// Draw the full simulation right panel.
    pub(crate) fn draw_simulation_panel(&mut self, ui: &mut egui::Ui) {
        self.poll_simulation_task();
        self.request_simulation_repaint(ui.ctx());

        ui.heading(self.text(UiText::SimulationWorkspace));
        self.draw_simulation_directive_editor(ui);

        // Backend selector + Run/Stop button
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt("simulation_backend")
                .selected_text(self.simulation_panel.backend.label())
                .show_ui(ui, |ui| {
                    for &kind in &SimulationBackendKind::ALL {
                        let label = match self.locale() {
                            crate::app::localization::StudioLocale::SimplifiedChinese => kind.label_zh(),
                            _ => kind.label(),
                        };
                        ui.selectable_value(&mut self.simulation_panel.backend, kind, label);
                    }
                });
            ui.separator();
            let running = self.simulation_panel.active_task.is_some();
            if running {
                if ui.button("Stop").on_hover_text("Cancel running simulation").clicked() {
                    self.simulation_panel.active_task = None;
                    self.status_message = Some("Simulation cancelled".to_string());
                }
            } else if ui
                .add_enabled(self.document.is_some(), egui::Button::new(self.text(UiText::RunSimulation)))
                .clicked()
            {
                self.run_simulation_from_panel();
            }
        });
        ui.separator();

        // Diagnostics
        self.draw_document_diagnostics_panel(ui, 150.0);
    }
}
