//! Simulation right panel — thin orchestrator that composes the directive
//! editor, run controller, and status display into the sidebar panel.
//!
//! The heavy lifting lives in sibling modules:
//! - `state` — SimulationBackendKind, SimulationPanelState
//! - `directive_editor` — directive kind/body editing UI
//! - `run_controller` — profile building, run launch, task polling
//! - `status_display` — run results, log viewer, waveform summary
//!
//! The panel is organized as:
//! 1. Backend selector + Run/Stop controls
//! 2. Quick analysis directive editor
//! 3. Quick configuration summary (key settings at a glance)
//! 4. Diagnostics

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

        // Panel title
        ui.heading(self.text(UiText::SimulationWorkspace));
        ui.add_space(6.0);

        // Backend selector + Run/Stop button (prominent)
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
                // Show running indicator with stop button
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
                // Prominent run button
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

        // Quick analysis directive editor
        self.draw_simulation_directive_editor(ui);

        ui.add_space(8.0);

        // Quick configuration summary
        self.draw_panel_config_summary(ui);


        // Collapsible netlist preview
        self.draw_panel_netlist_preview(ui);
        ui.add_space(8.0);

        // Diagnostics
        self.draw_document_diagnostics_panel(ui, 150.0);
    }

    /// Compact configuration summary for the sidebar panel.
    /// Shows the key simulation settings at a glance without full profile editor.
    fn draw_panel_config_summary(&self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let opts = &self.simulation_profile_editor.options;
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(mode, "Quick Settings"));
            ui.add_space(4.0);
            egui::Grid::new("panel_config_summary")
                .num_columns(2)
                .spacing([8.0, 4.0])
                .show(ui, |ui| {
                    // Backend engine
                    ui.label(StudioTheme::muted_for(mode, "Backend"));
                    ui.label(egui::RichText::new(self.simulation_panel.backend.label()).monospace());
                    ui.end_row();

                    // Analysis type
                    ui.label(StudioTheme::muted_for(mode, "Analysis"));
                    ui.label(egui::RichText::new(format!(".{}", self.simulation_panel.directive_kind)).monospace());
                    ui.end_row();

                    ui.label(StudioTheme::muted_for(mode, "Temp"));
                    ui.label(egui::RichText::new(format!("{} °C", opts.temperature)).monospace());
                    ui.end_row();

                    ui.label(StudioTheme::muted_for(mode, "Method"));
                    ui.label(egui::RichText::new(&opts.method).monospace());
                    ui.end_row();

                    ui.label(StudioTheme::muted_for(mode, "RELTOL"));
                    ui.label(egui::RichText::new(&opts.reltol).monospace());
                    ui.end_row();

                    if self.simulation_profile_editor.active_preset != "default" {
                        ui.label(StudioTheme::muted_for(mode, "Preset"));
                        ui.label(
                            egui::RichText::new(&self.simulation_profile_editor.active_preset)
                                .monospace()
                                .color(self.theme_palette().accent),
                        );
                        ui.end_row();
                    }

                    // Step sweep status
                    if let super::state::StepSweep::Parametric { param_name, sweep_mode, .. } = &self.simulation_panel.step_sweep {
                        ui.label(StudioTheme::muted_for(mode, "Step"));
                        ui.label(
                            egui::RichText::new(format!(".step {} {}", param_name, sweep_mode))
                                .monospace()
                                .color(self.theme_palette().accent),
                        );
                        ui.end_row();
                    }

                    // Measurement count
                    if !self.simulation_measurements.is_empty() {
                        ui.label(StudioTheme::muted_for(mode, "Measures"));
                        ui.label(
                            egui::RichText::new(format!("{} directive(s)", self.simulation_measurements.len()))
                                .monospace(),
                        );
                        ui.end_row();
                    }

                    // IC/Nodeset count
                    let ic_count = self.simulation_profile_editor.initial_conditions.len()
                        + self.simulation_profile_editor.nodesets.len();
                    if ic_count > 0 {
                        ui.label(StudioTheme::muted_for(mode, ".ic/.ns"));
                        ui.label(
                            egui::RichText::new(format!("{} entry(ies)", ic_count))
                                .monospace(),
                        );
                        ui.end_row();
                    }

                    // Component parameters count
                    let comp_count = self.simulation_profile_editor.component_params.len();
                    if comp_count > 0 {
                        ui.label(StudioTheme::muted_for(mode, "Components"));
                        ui.label(
                            egui::RichText::new(format!("{} defined", comp_count))
                                .monospace(),
                        );
                        ui.end_row();
                    }

                    // Model parameters count
                    let model_count = self.simulation_profile_editor.model_params.len();
                    if model_count > 0 {
                        ui.label(StudioTheme::muted_for(mode, "Models"));
                        ui.label(
                            egui::RichText::new(format!("{} defined", model_count))
                                .monospace(),
                        );
                        ui.end_row();
                    }
                });
        });
    }

    /// Collapsible netlist preview in the sidebar panel.
    fn draw_panel_netlist_preview(&self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let palette = StudioTheme::palette(mode);
        egui::CollapsingHeader::new(
            egui::RichText::new("Netlist Preview").color(palette.text),
        )
        .id_salt("panel_netlist_preview")
        .default_open(false)
        .show(ui, |ui| {
            let Some(document) = &self.document else {
                ui.label(StudioTheme::muted_for(mode, "No schematic loaded"));
                return;
            };
            let profile = self.build_simulation_profile();
            match document.spice_netlist_preview().map(|raw| {
                osl_sim::inject_profile_directives(&raw, &profile)
            }) {
                Ok(netlist) => {
                    let line_count = netlist.lines().count();
                    ui.label(StudioTheme::muted_for(
                        mode,
                        format!("{} lines — {} backend", line_count, self.simulation_panel.backend.label()),
                    ));
                    ui.add_space(2.0);
                    egui::ScrollArea::vertical()
                        .id_salt("panel_netlist_scroll")
                        .max_height(120.0)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            for line in netlist.lines().take(30) {
                                ui.monospace(egui::RichText::new(line).size(10.0).color(palette.text_muted));
                            }
                            if line_count > 30 {
                                ui.label(StudioTheme::muted_for(
                                    mode,
                                    format!("... {} more lines", line_count - 30),
                                ));
                            }
                        });
                }
                Err(error) => {
                    ui.colored_label(palette.danger, format!("Netlist error: {}", error));
                }
            }
        });
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
