//! Step sweep editor — UI for configuring `.step` parameter sweeps.
//!
//! Supports four sweep modes:
//! - **List**: explicit values (`.step param R1 list 1k 2k 5k 10k`)
//! - **Linear**: start/stop/step (`.step param R1 lin 1k 10k 1k`)
//! - **Decade**: points per decade (`.step param R1 dec 10 1k 100k`)
//! - **Octave**: points per octave (`.step param R1 oct 10 1k 100k`)

use crate::app::NekoSpiceApp;
use super::state::StepSweep;
use super::profile_editor_widgets::section_header;
use crate::app::theme::{StudioTheme, StudioThemeMode};
use eframe::egui;

/// Sweep mode options with descriptions.
const SWEEP_MODES: [(&str, &str); 4] = [
    ("list", "List"),
    ("lin", "Linear"),
    ("dec", "Decade"),
    ("oct", "Octave"),
];

/// Temperature sweep mode options (no list mode).
const TEMP_SWEEP_MODES: [(&str, &str); 3] = [
    ("lin", "Linear"),
    ("dec", "Decade"),
    ("oct", "Octave"),
];

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
            if ui.checkbox(&mut active, "Enable .step parameter sweep").changed() {
                if active && self.simulation_panel.step_sweep == StepSweep::None {
                    // Initialize with default parametric sweep
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
                let is_temp = matches!(self.simulation_panel.step_sweep, StepSweep::Temperature { .. });
                ui.horizontal(|ui| {
                    let btn_param = if !is_temp {
                        egui::Button::new(egui::RichText::new("Parametric").strong().color(palette.text))
                            .fill(palette.accent_soft)
                            .stroke(egui::Stroke::new(1.0, palette.accent))
                    } else {
                        egui::Button::new(egui::RichText::new("Parametric").color(palette.text_muted))
                            .fill(palette.panel_soft)
                            .stroke(egui::Stroke::new(1.0, palette.border))
                    };
                    if ui.add(btn_param).clicked() && is_temp {
                        self.simulation_panel.step_sweep = StepSweep::Parametric {
                            param_name: DEFAULT_PARAM_NAME.to_string(),
                            sweep_mode: "lin".to_string(),
                            start: "1k".to_string(),
                            stop: "100k".to_string(),
                            step: "10k".to_string(),
                        };
                        changed = true;
                    }
                    let btn_temp = if is_temp {
                        egui::Button::new(egui::RichText::new("Temperature").strong().color(palette.text))
                            .fill(palette.accent_soft)
                            .stroke(egui::Stroke::new(1.0, palette.accent))
                    } else {
                        egui::Button::new(egui::RichText::new("Temperature").color(palette.text_muted))
                            .fill(palette.panel_soft)
                            .stroke(egui::Stroke::new(1.0, palette.border))
                    };
                    if ui.add(btn_temp).clicked() && !is_temp {
                        self.simulation_panel.step_sweep = StepSweep::Temperature {
                            sweep_mode: "lin".to_string(),
                            start: "-40".to_string(),
                            stop: "125".to_string(),
                            step: "10".to_string(),
                        };
                        changed = true;
                    }
                });

                ui.add_space(4.0);

                if let StepSweep::Parametric {
                    param_name,
                    sweep_mode,
                    start,
                    stop,
                    step,
                } = &mut self.simulation_panel.step_sweep
                {
                    // Parameter name field
                    ui.label(StudioTheme::muted_for(mode, "Parameter"));
                    let name_resp = ui.add(
                        egui::TextEdit::singleline(param_name)
                            .desired_width(140.0)
                            .hint_text("e.g. R1, TEMP"),
                    );
                    changed |= name_resp.changed();
                    ui.add_space(4.0);

                    // Sweep mode selector buttons
                    ui.label(StudioTheme::muted_for(mode, "Mode"));
                    ui.horizontal(|ui| {
                        for &(mid, mlabel) in &SWEEP_MODES {
                            let active = sweep_mode.as_str() == mid;
                            let btn = if active {
                                egui::Button::new(
                                    egui::RichText::new(mlabel)
                                        .strong()
                                        .color(palette.text),
                                )
                                .fill(palette.accent_soft)
                                .stroke(egui::Stroke::new(1.0, palette.accent))
                            } else {
                                egui::Button::new(
                                    egui::RichText::new(mlabel).color(palette.text_muted),
                                )
                                .fill(palette.panel_soft)
                                .stroke(egui::Stroke::new(1.0, palette.border))
                            };
                            if ui.add(btn).clicked() && sweep_mode.as_str() != mid {
                                *sweep_mode = mid.to_string();
                                changed = true;
                            }
                        }
                    });

                    ui.add_space(4.0);

                    // Mode-specific input fields
                    match sweep_mode.as_str() {
                        "list" => {
                            ui.label(StudioTheme::muted_for(mode, "Values (space-separated)"));
                            let resp = ui.add(
                                egui::TextEdit::singleline(start)
                                    .desired_width(200.0)
                                    .hint_text("e.g. 1k 2k 5k 10k"),
                            );
                            changed |= resp.changed();
                        }
                        _ => {
                            egui::Grid::new("step_sweep_grid")
                                .num_columns(2)
                                .spacing([8.0, 6.0])
                                .show(ui, |ui| {
                                    ui.label(StudioTheme::muted_for(mode, "Start"));
                                    let r = ui.add(
                                        egui::TextEdit::singleline(start)
                                            .desired_width(100.0)
                                            .hint_text("1k"),
                                    );
                                    changed |= r.changed();
                                    ui.end_row();

                                    ui.label(StudioTheme::muted_for(mode, "Stop"));
                                    let r = ui.add(
                                        egui::TextEdit::singleline(stop)
                                            .desired_width(100.0)
                                            .hint_text("100k"),
                                    );
                                    changed |= r.changed();
                                    ui.end_row();

                                    let step_label = match sweep_mode.as_str() {
                                        "lin" => "Step",
                                        "dec" => "Pts/dec",
                                        "oct" => "Pts/oct",
                                        _ => "Step",
                                    };
                                    ui.label(StudioTheme::muted_for(mode, step_label));
                                    let r = ui.add(
                                        egui::TextEdit::singleline(step)
                                            .desired_width(100.0)
                                            .hint_text("1k"),
                                    );
                                    changed |= r.changed();
                                    ui.end_row();
                                });
                        }
                    }

                    // Show generated directive preview
                    ui.add_space(4.0);
                    ui.separator();
                    ui.add_space(4.0);
                    if let Some(directive) = self.simulation_panel.step_sweep.to_directive() {
                        ui.label(StudioTheme::muted_for(mode, "Generated directive:"));
                        ui.monospace(directive);
                    }
                } else if let StepSweep::Temperature { sweep_mode, start, stop, step } = &mut self.simulation_panel.step_sweep {
                    // Temperature sweep UI (no parameter name needed)
                    ui.label(StudioTheme::muted_for(mode, "Sweep Mode"));
                    ui.horizontal(|ui| {
                        for &(mid, mlabel) in &TEMP_SWEEP_MODES {
                            let active = sweep_mode.as_str() == mid;
                            let btn = if active {
                                egui::Button::new(egui::RichText::new(mlabel).strong().color(palette.text))
                                    .fill(palette.accent_soft)
                                    .stroke(egui::Stroke::new(1.0, palette.accent))
                            } else {
                                egui::Button::new(egui::RichText::new(mlabel).color(palette.text_muted))
                                    .fill(palette.panel_soft)
                                    .stroke(egui::Stroke::new(1.0, palette.border))
                            };
                            if ui.add(btn).clicked() && sweep_mode.as_str() != mid {
                                *sweep_mode = mid.to_string();
                                changed = true;
                            }
                        }
                    });

                    ui.add_space(4.0);
                    egui::Grid::new("temp_sweep_grid")
                        .num_columns(2)
                        .spacing([8.0, 6.0])
                        .show(ui, |ui| {
                            ui.label(StudioTheme::muted_for(mode, "Start (°C)"));
                            let r = ui.add(egui::TextEdit::singleline(start).desired_width(100.0).hint_text("-40"));
                            changed |= r.changed();
                            ui.end_row();

                            ui.label(StudioTheme::muted_for(mode, "Stop (°C)"));
                            let r = ui.add(egui::TextEdit::singleline(stop).desired_width(100.0).hint_text("125"));
                            changed |= r.changed();
                            ui.end_row();

                            let step_label = match sweep_mode.as_str() {
                                "lin" => "Step (°C)",
                                "dec" => "Pts/dec",
                                "oct" => "Pts/oct",
                                _ => "Step",
                            };
                            ui.label(StudioTheme::muted_for(mode, step_label));
                            let r = ui.add(egui::TextEdit::singleline(step).desired_width(100.0).hint_text("10"));
                            changed |= r.changed();
                            ui.end_row();
                        });

                    // Show generated directive preview
                    ui.add_space(4.0);
                    ui.separator();
                    ui.add_space(4.0);
                    if let Some(directive) = self.simulation_panel.step_sweep.to_directive() {
                        ui.label(StudioTheme::muted_for(mode, "Generated directive:"));
                        ui.monospace(directive);
                    }
                }
            }
        });

        if changed {
            self.save_simulation_settings();
        }
        changed
    }
}
