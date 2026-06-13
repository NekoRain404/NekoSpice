//! Environment settings — temperature and nominal temperature.
//!
//! Uses field validation to show colored borders when values are outside
//! expected ranges (e.g. below absolute zero).

use crate::app::NekoSpiceApp;
use super::field_validation::{validate_temperature, validated_frame};
use super::profile_editor_widgets::section_header;
use crate::app::theme::StudioTheme;
use eframe::egui;

/// Draw the environment section (temperature, TNOM).
/// Returns `true` when any field changes.
pub(crate) fn draw_environment_section(
    app: &mut NekoSpiceApp,
    ui: &mut egui::Ui,
    mode: crate::app::theme::StudioThemeMode,
) -> bool {
    let mut changed = false;

    StudioTheme::panel_frame_for(mode).show(ui, |ui| {
        section_header(ui, mode, "Environment");
        ui.add_space(4.0);

        let palette = StudioTheme::palette(mode);

        egui::Grid::new("environment_grid")
            .num_columns(2)
            .spacing([8.0, 6.0])
            .show(ui, |ui| {
                // Temperature with validation
                let temp_validity = validate_temperature(&app.simulation_profile_editor.options.temperature);
                ui.label(StudioTheme::muted_for(mode, "Temperature (°C)"));
                validated_frame(temp_validity, &palette).show(ui, |ui| {
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut app.simulation_profile_editor.options.temperature)
                            .desired_width(120.0)
                            .hint_text("27"),
                    );
                    let tip = temp_validity.tooltip();
                    changed |= resp.changed();
                    if !tip.is_empty() { resp.on_hover_text(tip); }
                });
                ui.end_row();

                // TNOM with validation
                let tnom_validity = validate_temperature(&app.simulation_profile_editor.options.tnom);
                ui.label(StudioTheme::muted_for(mode, "TNOM (°C)"));
                validated_frame(tnom_validity, &palette).show(ui, |ui| {
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut app.simulation_profile_editor.options.tnom)
                            .desired_width(120.0)
                            .hint_text("27"),
                    );
                    let tip = tnom_validity.tooltip();
                    changed |= resp.changed();
                    if !tip.is_empty() { resp.on_hover_text(tip); }
                });
                ui.end_row();
            });
    });

    changed
}
