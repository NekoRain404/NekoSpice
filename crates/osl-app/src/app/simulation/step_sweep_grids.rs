//! Step sweep grid helpers — parameter input grids for parametric
//! and temperature sweeps. Extracted from step_sweep_editor.rs.
//!
//! Each grid renders labeled TextEdit fields for sweep start/stop/step
//! values and returns whether any field changed.

use super::state::StepSweep;
use crate::app::NekoSpiceApp;
use crate::app::theme::{StudioTheme, StudioThemeMode};
use eframe::egui;

/// Sweep mode options with descriptions.
pub(crate) const SWEEP_MODES: [(&str, &str); 4] = [
    ("list", "List"),
    ("lin", "Linear"),
    ("dec", "Decade"),
    ("oct", "Octave"),
];

/// Temperature sweep mode options (no list mode).
pub(crate) const TEMP_SWEEP_MODES: [(&str, &str); 3] =
    [("lin", "Linear"), ("dec", "Decade"), ("oct", "Octave")];

impl NekoSpiceApp {
    /// Draw the parametric sweep input grid.
    /// Returns `true` when any field changes.
    pub(crate) fn draw_parametric_sweep_grid(
        &mut self,
        ui: &mut egui::Ui,
        mode: StudioThemeMode,
    ) -> bool {
        let mut changed = false;
        let palette = StudioTheme::palette(mode);
        let sweep_mode = match &self.simulation_panel.step_sweep {
            StepSweep::Parametric { sweep_mode, .. } => sweep_mode.clone(),
            _ => return false,
        };

        // Mode selector
        ui.label(StudioTheme::muted_for(mode, "Sweep Mode"));
        ui.horizontal(|ui| {
            for &(mid, mlabel) in &SWEEP_MODES {
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
                if ui.add(btn).clicked() {
                    if let StepSweep::Parametric {
                        sweep_mode: ref mut sm,
                        ..
                    } = self.simulation_panel.step_sweep
                    {
                        *sm = mid.to_string();
                    }
                    changed = true;
                }
                ui.add_space(2.0);
            }
        });

        ui.add_space(4.0);

        // Input grid
        if let StepSweep::Parametric {
            param_name,
            sweep_mode,
            start,
            stop,
            step,
        } = &mut self.simulation_panel.step_sweep
        {
            egui::Grid::new("param_sweep_grid")
                .num_columns(2)
                .spacing([8.0, 6.0])
                .show(ui, |ui| {
                    ui.label(StudioTheme::muted_for(mode, "Parameter"));
                    let r = ui.add(
                        egui::TextEdit::singleline(param_name)
                            .desired_width(100.0)
                            .hint_text("R1"),
                    );
                    changed |= r.changed();
                    ui.end_row();

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

        changed
    }

    /// Draw the temperature sweep input grid.
    /// Returns `true` when any field changes.
    pub(crate) fn draw_temperature_sweep_grid(
        &mut self,
        ui: &mut egui::Ui,
        mode: StudioThemeMode,
    ) -> bool {
        let mut changed = false;
        let palette = StudioTheme::palette(mode);
        let sweep_mode = match &self.simulation_panel.step_sweep {
            StepSweep::Temperature { sweep_mode, .. } => sweep_mode.clone(),
            _ => return false,
        };

        // Mode selector
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
                if ui.add(btn).clicked() {
                    if let StepSweep::Temperature {
                        sweep_mode: ref mut sm,
                        ..
                    } = self.simulation_panel.step_sweep
                    {
                        *sm = mid.to_string();
                    }
                    changed = true;
                }
                ui.add_space(2.0);
            }
        });

        ui.add_space(4.0);

        if let StepSweep::Temperature {
            sweep_mode,
            start,
            stop,
            step,
        } = &mut self.simulation_panel.step_sweep
        {
            egui::Grid::new("temp_sweep_grid")
                .num_columns(2)
                .spacing([8.0, 6.0])
                .show(ui, |ui| {
                    ui.label(StudioTheme::muted_for(mode, "Start (°C)"));
                    let r = ui.add(
                        egui::TextEdit::singleline(start)
                            .desired_width(100.0)
                            .hint_text("-40"),
                    );
                    changed |= r.changed();
                    ui.end_row();

                    ui.label(StudioTheme::muted_for(mode, "Stop (°C)"));
                    let r = ui.add(
                        egui::TextEdit::singleline(stop)
                            .desired_width(100.0)
                            .hint_text("125"),
                    );
                    changed |= r.changed();
                    ui.end_row();

                    let step_label = match sweep_mode.as_str() {
                        "lin" => "Step (°C)",
                        "dec" => "Pts/dec",
                        "oct" => "Pts/oct",
                        _ => "Step",
                    };
                    ui.label(StudioTheme::muted_for(mode, step_label));
                    let r = ui.add(
                        egui::TextEdit::singleline(step)
                            .desired_width(100.0)
                            .hint_text("10"),
                    );
                    changed |= r.changed();
                    ui.end_row();
                });
        }

        changed
    }
}
