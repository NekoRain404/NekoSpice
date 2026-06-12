//! Measure directive editor — UI for adding `.measure` post-simulation measurements.
//!
//! `.measure` directives extract key values from simulation results:
//! - `rise time`, `fall time`, `bandwidth`, `peak`, `RMS`, etc.
//! - These are processed by the solver and printed in the log output.

use crate::app::NekoSpiceApp;
use super::profile_editor_widgets::section_header;
use crate::app::theme::{StudioTheme, StudioThemeMode};
use eframe::egui;

/// A single measurement definition.
#[derive(Debug, Clone)]
pub(crate) struct MeasureEntry {
    /// Measurement name (e.g. "rise_time", "vout_rms").
    pub(crate) name: String,
    /// Measurement expression (e.g. "v(out) rise 10% 90%").
    pub(crate) expression: String,
}

impl Default for MeasureEntry {
    fn default() -> Self {
        Self {
            name: String::new(),
            expression: String::new(),
        }
    }
}

/// Common measurement templates.
const MEASURE_TEMPLATES: [(&str, &str); 6] = [
    ("rise_time", "v(out) rise 10% 90%"),
    ("fall_time", "v(out) fall 90% 10%"),
    ("bandwidth", "v(out) -3dB"),
    ("peak", "v(out) max"),
    ("vout_rms", "v(out) rms"),
    ("power_avg", "v(vdd)*i(vdd) avg"),
];

impl NekoSpiceApp {
    /// Draw the measure directive editor section.
    /// Returns the list of `.measure` directives as strings.
    pub(crate) fn draw_measure_editor(
        &mut self,
        ui: &mut egui::Ui,
        mode: StudioThemeMode,
    ) {
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            section_header(ui, mode, "Measurements (.measure)");
            ui.add_space(4.0);

            let palette = StudioTheme::palette(mode);

            // Show current entries
            let mut remove_index = None;
            for (i, entry) in self.simulation_measurements.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut entry.name)
                            .desired_width(80.0)
                            .hint_text("name"),
                    );
                    ui.add(
                        egui::TextEdit::singleline(&mut entry.expression)
                            .desired_width(180.0)
                            .hint_text("expression"),
                    );
                    if ui.small_button("x").clicked() {
                        remove_index = Some(i);
                    }
                });
            }
            if let Some(idx) = remove_index {
                self.simulation_measurements.remove(idx);
            }

            ui.add_space(4.0);

            // Add new entry
            ui.horizontal(|ui| {
                if ui.small_button("+ Add").clicked() {
                    self.simulation_measurements.push(MeasureEntry::default());
                }
                // Template buttons
                for (label, expr) in &MEASURE_TEMPLATES {
                    if ui.small_button(*label).on_hover_text(*expr).clicked() {
                        self.simulation_measurements.push(MeasureEntry {
                            name: label.to_string(),
                            expression: expr.to_string(),
                        });
                    }
                }
            });

            // Show generated directives
            if !self.simulation_measurements.is_empty() {
                ui.add_space(4.0);
                ui.separator();
                ui.add_space(4.0);
                ui.label(StudioTheme::muted_for(mode, "Generated directives:"));
                for entry in &self.simulation_measurements {
                    if !entry.name.is_empty() && !entry.expression.is_empty() {
                        ui.monospace(format!(".measure {} {}", entry.name, entry.expression));
                    }
                }
            }
        });
    }

    /// Build all `.measure` directive lines from current measurements.
    pub(crate) fn build_measure_directives(&self) -> Vec<String> {
        self.simulation_measurements
            .iter()
            .filter(|e| !e.name.trim().is_empty() && !e.expression.trim().is_empty())
            .map(|e| format!(".measure {} {}", e.name.trim(), e.expression.trim()))
            .collect()
    }
}
