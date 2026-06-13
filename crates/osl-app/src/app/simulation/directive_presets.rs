//! Analysis range presets — quick-fill buttons for common frequency,
//! voltage, and time ranges in the directive editor.
//!
//! Each preset fills in the corresponding fields with values commonly
//! used for specific circuit types (audio, RF, logic, etc.).

use crate::app::NekoSpiceApp;
use super::state::AnalysisParams;
use crate::app::theme::StudioTheme;
use eframe::egui;

/// AC frequency range presets.
const AC_PRESETS: [(&str, &str, &str); 4] = [
    ("Audio", "20", "20k"),
    ("RF", "1M", "6G"),
    ("Low-freq", "1", "100k"),
    ("Wideband", "10", "1G"),
];

/// DC voltage range presets.
const DC_PRESETS: [(&str, &str, &str, &str); 4] = [
    ("3.3V logic", "0", "3.3", "0.1"),
    ("5V logic", "0", "5", "0.1"),
    ("12V rail", "0", "12", "0.5"),
    ("Battery", "2.5", "4.2", "0.01"),
];

/// Transient time range presets.
const TRAN_PRESETS: [(&str, &str, &str); 4] = [
    ("RC 1kHz", "10u", "5m"),
    ("RC 10kHz", "1u", "500u"),
    ("Switching 100kHz", "100n", "100u"),
    ("Power startup", "1u", "10m"),
];

impl NekoSpiceApp {
    /// Draw AC frequency range presets below the AC parameter grid.
    pub(crate) fn draw_ac_range_presets(&mut self, ui: &mut egui::Ui, mode: crate::app::theme::StudioThemeMode) {
        let AnalysisParams::Ac { ref mut fstart, ref mut fstop, .. } = self.simulation_panel.analysis_params else {
            return;
        };
        ui.add_space(4.0);
        ui.label(StudioTheme::muted_for(mode, "Range presets:"));
        ui.horizontal_wrapped(|ui| {
            for &(label, fs, fe) in &AC_PRESETS {
                if ui.small_button(label).on_hover_text(format!("{} Hz to {} Hz", fs, fe)).clicked() {
                    *fstart = fs.to_string();
                    *fstop = fe.to_string();
                }
            }
        });
    }

    /// Draw DC voltage range presets below the DC parameter grid.
    pub(crate) fn draw_dc_range_presets(&mut self, ui: &mut egui::Ui, mode: crate::app::theme::StudioThemeMode) {
        let AnalysisParams::Dc { ref mut vstart, ref mut vstop, ref mut vincr, .. } = self.simulation_panel.analysis_params else {
            return;
        };
        ui.add_space(4.0);
        ui.label(StudioTheme::muted_for(mode, "Range presets:"));
        ui.horizontal_wrapped(|ui| {
            for &(label, vs, ve, vi) in &DC_PRESETS {
                if ui.small_button(label).clicked() {
                    *vstart = vs.to_string();
                    *vstop = ve.to_string();
                    *vincr = vi.to_string();
                }
            }
        });
    }

    /// Draw transient time range presets below the Tran parameter grid.
    pub(crate) fn draw_tran_range_presets(&mut self, ui: &mut egui::Ui, mode: crate::app::theme::StudioThemeMode) {
        let AnalysisParams::Tran { ref mut tstep, ref mut tstop, .. } = self.simulation_panel.analysis_params else {
            return;
        };
        ui.add_space(4.0);
        ui.label(StudioTheme::muted_for(mode, "Time range presets:"));
        ui.horizontal_wrapped(|ui| {
            for &(label, ts, te) in &TRAN_PRESETS {
                if ui.small_button(label).clicked() {
                    *tstep = ts.to_string();
                    *tstop = te.to_string();
                }
            }
        });
    }
}
