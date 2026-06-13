//! Simulation preset selector — quick-apply common solver configurations.
//!
//! Provides a dropdown and description panel for the built-in presets:
//! Default, Fast, Accurate, High Frequency, Convergence Aid.

use crate::app::NekoSpiceApp;
use crate::app::theme::StudioTheme;
use eframe::egui;
use osl_sim::available_presets;

/// Draw the preset selector section at the top of the options panel.
/// Returns `true` when a preset is applied (caller should persist).
pub(crate) fn draw_preset_selector(
    app: &mut NekoSpiceApp,
    ui: &mut egui::Ui,
    mode: crate::app::theme::StudioThemeMode,
) -> bool {
    let _palette = StudioTheme::palette(mode);
    let mut changed = false;

    StudioTheme::panel_frame_for(mode).show(ui, |ui| {
        ui.label(StudioTheme::section_title_for(mode, "Simulation Preset"));
        ui.add_space(4.0);

        let presets = available_presets();
        let current_preset = app.simulation_profile_editor.active_preset.clone();
        let current_label = presets.iter()
            .find(|(name, _)| *name == current_preset.as_str())
            .map(|(_, label)| *label)
            .unwrap_or("Custom");

        egui::ComboBox::from_id_salt("preset_selector")
            .selected_text(current_label)
            .width(200.0)
            .show_ui(ui, |ui| {
                for &(name, label) in presets {
                    let selected = current_preset == name;
                    if ui.selectable_label(selected, label).clicked() && !selected {
                        apply_preset(app, name);
                        app.simulation_profile_editor.active_preset = name.to_string();
                        changed = true;
                    }
                }
            });

        // Show preset description
        let description = match current_preset.as_str() {
            "fast" => "Relaxed tolerances for quick iteration. Trades accuracy for speed.",
            "accurate" => "Tight tolerances with Gear integration. Best for precision analysis.",
            "high-freq" => "Optimized for high-frequency circuits. Balanced Trap method.",
            "convergence-help" => "Aggressive convergence aids. Use when simulation fails to converge.",
            _ => "Standard SPICE defaults. Balanced speed and accuracy.",
        };
        ui.add_space(2.0);
        ui.label(StudioTheme::muted_for(mode, description));

        // Apply preset button (manual re-apply)
        ui.add_space(4.0);
        if ui.button("Apply Preset").clicked() {
            let preset_name = app.simulation_profile_editor.active_preset.clone();
            apply_preset(app, &preset_name);
            changed = true;
        }
    });

    changed
}

/// Apply a named preset to the simulation profile editor options.
fn apply_preset(app: &mut NekoSpiceApp, name: &str) {
    use osl_sim::simulation_preset;
    let profile = simulation_preset(name);
    let opts = &mut app.simulation_profile_editor.options;
    opts.reltol = profile.reltol;
    opts.abstol = profile.abstol;
    opts.vntol = profile.vntol;
    opts.gmin = profile.gmin;
    opts.chgtol = profile.chgtol;
    opts.pivtol = profile.pivtol;
    opts.pivrel = profile.pivrel;
    opts.itl1 = profile.itl1;
    opts.itl2 = profile.itl2;
    opts.itl4 = profile.itl4;
    opts.itl5 = profile.itl5;
    opts.min_timestep = profile.min_timestep;
    opts.srcsteps = profile.srcsteps;
    opts.gminsteps = profile.gminsteps;
    opts.method = match profile.method {
        osl_sim::SpiceMethod::Gear => "Gear".to_string(),
        osl_sim::SpiceMethod::Trap => "Trap".to_string(),
    };
}
