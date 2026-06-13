//! Simulation preset indicator — shows active preset in the options panel.
//!
//! The main preset selector lives in profile_editor.rs (button row).
//! This module provides a compact status badge for the right-column options.

use crate::app::NekoSpiceApp;
use crate::app::theme::StudioTheme;
use eframe::egui;

/// Draw a compact preset status indicator in the right-column options panel.
pub(crate) fn draw_preset_indicator(
    app: &NekoSpiceApp,
    ui: &mut egui::Ui,
    mode: crate::app::theme::StudioThemeMode,
) {
    let palette = StudioTheme::palette(mode);
    let preset = &app.simulation_profile_editor.active_preset;

    StudioTheme::panel_frame_for(mode).show(ui, |ui| {
        ui.label(StudioTheme::section_title_for(mode, "Active Preset"));
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            let accent = palette.accent;
            ui.label(egui::RichText::new("●").color(accent).size(10.0));
            ui.label(egui::RichText::new(preset.as_str()).strong().color(palette.text));
        });

        let description = match preset.as_str() {
            "fast" => "Relaxed tolerances for quick iteration.",
            "accurate" => "Tight tolerances with Gear integration.",
            "high-freq" => "Optimized for high-frequency circuits.",
            "convergence-help" => "Aggressive convergence aids.",
            _ => "Standard SPICE defaults.",
        };
        ui.label(StudioTheme::muted_for(mode, description));
    });
}
