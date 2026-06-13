//! Simulation right panel — thin orchestrator that composes the directive
//! editor, run controller, and status display into the sidebar panel.
//!
//! Heavy lifting lives in sibling modules:
//! - `state` — SimulationBackendKind, SimulationPanelState
//! - `directive_editor` — directive kind/body editing UI
//! - `run_controller` — profile building, run launch, task polling
//! - `status_display` — run results, log viewer, waveform summary
//! - `panel_sections` — config summary, netlist preview

use crate::app::NekoSpiceApp;
use crate::app::localization::UiText;
use crate::app::theme::StudioTheme;
use eframe::egui;
use super::state::SimulationBackendKind;

impl NekoSpiceApp {
    /// Draw the full simulation right panel.
    pub(crate) fn draw_simulation_panel(&mut self, ui: &mut egui::Ui) {
        self.poll_simulation_task();
        self.request_simulation_repaint(ui.ctx());

        let mode = self.theme_mode();
        let palette = self.theme_palette();

        ui.heading(self.text(UiText::SimulationWorkspace));
        ui.add_space(6.0);

        // Backend selector + Run/Stop button
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(StudioTheme::muted_for(mode, self.text(UiText::Backend)));
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
            });
            ui.add_space(4.0);

            let running = self.simulation_panel.active_task.is_some();
            if running {
                ui.horizontal(|ui| {
                    ui.label(StudioTheme::status_dot(palette.warning));
                    ui.label(
                        egui::RichText::new(self.text(UiText::Running))
                            .color(palette.warning)
                            .strong(),
                    );
                    if ui.button("Stop").on_hover_text("Cancel running simulation").clicked() {
                        self.simulation_panel.active_task = None;
                        self.status_message = Some("Simulation cancelled".to_string());
                    }
                });
            } else {
                let run_btn = ui.add_enabled(
                    self.document.is_some(),
                    egui::Button::new(
                        egui::RichText::new(self.text(UiText::RunSimulation))
                            .strong()
                            .color(palette.text),
                    )
                    .fill(palette.accent_soft)
                    .min_size(egui::Vec2::new(ui.available_width(), 32.0)),
                );
                if run_btn.clicked() {
                    self.run_simulation_from_panel();
                }
            }
        });

        ui.add_space(8.0);

        // Workflow step indicator
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(mode, "Workflow"));
            ui.add_space(2.0);
            let has_doc = self.document.is_some();
            let running = self.simulation_panel.active_task.is_some();
            let has_result = self.simulation_panel.last_run.is_some();
            ui.horizontal(|ui| {
                workflow_step(ui, mode, "1", "Configure", has_doc, !running);
                workflow_step(ui, mode, "2", "Run", has_doc && !running, running);
                workflow_step(ui, mode, "3", "Analyze", has_result, has_result);
            });
        });

        // Quick Start templates
        super::quick_start::draw_quick_start_panel(self, ui, mode);

        ui.add_space(8.0);

        // Analysis directive editor
        self.draw_simulation_directive_editor(ui);

        ui.add_space(8.0);

        // Configuration summary (from panel_sections.rs)
        self.draw_panel_config_summary(ui);

        ui.add_space(8.0);

        // Parameter sweep (collapsible)
        egui::CollapsingHeader::new(
            egui::RichText::new("Parameter Sweep").color(palette.text),
        )
        .id_salt("panel_step_sweep")
        .default_open(false)
        .show(ui, |ui| {
            self.draw_step_sweep_editor(ui, mode);
        });

        ui.add_space(4.0);

        // Measurements (collapsible)
        egui::CollapsingHeader::new(
            egui::RichText::new("Measurements").color(palette.text),
        )
        .id_salt("panel_measurements")
        .default_open(false)
        .show(ui, |ui| {
            self.draw_measure_editor(ui, mode);
        });

        ui.add_space(8.0);

        // Netlist preview (from panel_sections.rs)
        self.draw_panel_netlist_preview(ui);
        ui.add_space(8.0);

        // Diagnostics
        self.draw_document_diagnostics_panel(ui, 150.0);
    }
}

/// Draw a workflow step indicator dot with label.
fn workflow_step(
    ui: &mut egui::Ui,
    mode: crate::app::theme::StudioThemeMode,
    num: &str,
    label: &str,
    completed: bool,
    active: bool,
) {
    let palette = StudioTheme::palette(mode);
    let color = if completed {
        palette.success
    } else if active {
        palette.accent
    } else {
        palette.text_muted
    };
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(num).strong().color(color).size(12.0));
        ui.label(
            egui::RichText::new(label)
                .color(if active || completed { palette.text } else { palette.text_muted })
                .size(11.0),
        );
    });
}
