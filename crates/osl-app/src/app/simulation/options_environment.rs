//! Environment section: operating temperature and nominal model temperature.
//!
//! Persists changes to disk via `save_simulation_settings()` whenever
//! the user modifies a field, so settings survive app restarts.

use crate::app::NekoSpiceApp;
use super::profile_editor_widgets::{labeled_field, section_header};
use crate::app::theme::{StudioTheme, StudioThemeMode};
use eframe::egui;

/// Draw the environment section with temperature settings.
/// Returns `true` if any field was changed (caller should save).
pub(crate) fn draw_environment_section(app: &mut NekoSpiceApp, ui: &mut egui::Ui, mode: StudioThemeMode) -> bool {
    let mut changed = false;
    StudioTheme::panel_frame_for(mode).show(ui, |ui| {
        section_header(ui, mode, "Environment");
        ui.add_space(4.0);
        egui::Grid::new("env_grid")
            .num_columns(2)
            .spacing([8.0, 6.0])
            .show(ui, |ui| {
                changed |= labeled_field(ui, mode, "Operating Temp (°C)", &mut app.simulation_profile_editor.options.temperature, 100.0);
                changed |= labeled_field(ui, mode, "Nominal Temp (°C)", &mut app.simulation_profile_editor.options.tnom, 100.0);
            });
    });
    changed
}
