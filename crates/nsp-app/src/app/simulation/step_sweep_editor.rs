//! Step sweep editor — UI for configuring `.step` parameter sweeps.
//!
//! Orchestrates sweep type selection (parametric vs temperature) and
//! delegates the input grids to `step_sweep_grids.rs`. Shows a generated
//! directive preview at the bottom for verification.
//!
//! Supports four sweep modes:
//! - **List**: explicit values (`.step param R1 list 1k 2k 5k 10k`)
//! - **Linear**: start/stop/step (`.step param R1 lin 1k 10k 1k`)
//! - **Decade**: points per decade (`.step param R1 dec 10 1k 100k`)
//! - **Octave**: points per octave (`.step param R1 oct 10 1k 100k`)

use super::profile_editor_widgets::section_header;
use super::state::StepSweep;
use crate::app::NekoSpiceApp;
use crate::app::theme::{StudioTheme, StudioThemeMode};
use eframe::egui;

/// Default parameter name for new sweep configurations.
const DEFAULT_PARAM_NAME: &str = "R1";

impl NekoSpiceApp {
    /// Draw the step sweep editor section.
    /// Returns `true` when the sweep configuration changes.
    pub(crate) fn draw_step_sweep_editor(
        &mut self,
        ui: &mut egui::Ui,
        mode: StudioThemeMode,
    ) -> bool {
        let mut changed = false;
        let palette = StudioTheme::palette(mode);

        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            section_header(ui, mode, "Step Sweep (.step)");
            ui.add_space(4.0);

            // Toggle sweep on/off
            let sweep_active = self.simulation_panel.step_sweep != StepSweep::None;
            let mut active = sweep_active;
            if ui
                .checkbox(&mut active, "Enable .step parameter sweep")
                .changed()
            {
                if active && self.simulation_panel.step_sweep == StepSweep::None {
                    self.simulation_panel.step_sweep = StepSweep::Parametric {
                        param_name: DEFAULT_PARAM_NAME.to_string(),
                        sweep_mode: "lin".to_string(),
                        start: "1k".to_string(),
                        stop: "100k".to_string(),
                        step: "10k".to_string(),
                    };
                    changed = true;
                } else if !active {
                    self.simulation_panel.step_sweep = StepSweep::None;
                    changed = true;
                }
            }

            if self.simulation_panel.step_sweep != StepSweep::None {
                ui.add_space(4.0);

                // Sweep type selector: Parametric vs Temperature
                ui.label(StudioTheme::muted_for(mode, "Sweep Type"));
                let is_temp = matches!(
                    self.simulation_panel.step_sweep,
                    StepSweep::Temperature { .. }
                );
                ui.horizontal(|ui| {
                    for (label, is_param) in [("Parametric", !is_temp), ("Temperature", is_temp)] {
                        let btn = if is_param {
                            egui::Button::new(
                                egui::RichText::new(label).strong().color(palette.text),
                            )
                            .fill(palette.accent_soft)
                            .stroke(egui::Stroke::new(1.0, palette.accent))
                        } else {
                            egui::Button::new(egui::RichText::new(label).color(palette.text_muted))
                                .fill(palette.panel_soft)
                                .stroke(egui::Stroke::new(1.0, palette.border))
                        };
                        if ui.add(btn).clicked() {
                            let want_temp = label == "Temperature";
                            if want_temp && !is_temp {
                                self.simulation_panel.step_sweep = StepSweep::Temperature {
                                    sweep_mode: "lin".to_string(),
                                    start: "-40".to_string(),
                                    stop: "125".to_string(),
                                    step: "10".to_string(),
                                };
                                changed = true;
                            } else if !want_temp && is_temp {
                                self.simulation_panel.step_sweep = StepSweep::Parametric {
                                    param_name: DEFAULT_PARAM_NAME.to_string(),
                                    sweep_mode: "lin".to_string(),
                                    start: "1k".to_string(),
                                    stop: "100k".to_string(),
                                    step: "10k".to_string(),
                                };
                                changed = true;
                            }
                        }
                        ui.add_space(2.0);
                    }
                });

                ui.add_space(4.0);

                // Delegate to type-specific grid
                if is_temp {
                    changed |= self.draw_temperature_sweep_grid(ui, mode);
                } else {
                    changed |= self.draw_parametric_sweep_grid(ui, mode);
                }

                // Generated directive preview
                ui.add_space(4.0);
                ui.separator();
                ui.add_space(4.0);
                if let Some(directive) = self.simulation_panel.step_sweep.to_directive() {
                    ui.label(StudioTheme::muted_for(mode, "Generated directive:"));
                    ui.monospace(directive);
                }
            }
        });

        if changed {
            self.save_simulation_settings();
        }
        changed
    }
}
