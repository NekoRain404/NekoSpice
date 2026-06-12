//! Initial conditions section: .ic and .nodeset entries.
//!
//! Tracks changes and returns true when any entry is modified,
//! allowing the orchestrator to persist to disk.

use crate::app::NekoSpiceApp;
use super::profile_editor_widgets::section_header;
use crate::app::theme::{StudioTheme, StudioThemeMode};
use eframe::egui;

/// Draw the initial conditions section with .ic and .nodeset entries.
/// Returns `true` when any entry changes.
pub(crate) fn draw_initial_conditions_section(
    app: &mut NekoSpiceApp,
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
) -> bool {
    let mut changed = false;

    StudioTheme::panel_frame_for(mode).show(ui, |ui| {
        section_header(ui, mode, "Initial Conditions");
        ui.add_space(4.0);

        // ── .ic entries ─────────────────────────────────────────────
        ui.label(StudioTheme::muted_for(mode, ".ic — Node voltages"));
        ui.add_space(2.0);
        let mut remove_ic = None;
        for (i, (node, value)) in app.simulation_profile_editor.initial_conditions.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                let r1 = ui.add(egui::TextEdit::singleline(node).desired_width(80.0).hint_text("node"));
                let r2 = ui.add(egui::TextEdit::singleline(value).desired_width(80.0).hint_text("voltage"));
                changed |= r1.changed() || r2.changed();
                if ui.small_button("x").clicked() {
                    remove_ic = Some(i);
                }
            });
        }
        if let Some(idx) = remove_ic {
            app.simulation_profile_editor.initial_conditions.remove(idx);
            changed = true;
        }
        if ui.small_button("+ Add .ic").clicked() {
            app.simulation_profile_editor.initial_conditions.push((String::new(), String::new()));
            changed = true;
        }

        ui.add_space(6.0);

        // ── .nodeset entries ────────────────────────────────────────
        ui.label(StudioTheme::muted_for(mode, ".nodeset — Convergence hints"));
        ui.add_space(2.0);
        let mut remove_ns = None;
        for (i, (node, value)) in app.simulation_profile_editor.nodesets.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                let r1 = ui.add(egui::TextEdit::singleline(node).desired_width(80.0).hint_text("node"));
                let r2 = ui.add(egui::TextEdit::singleline(value).desired_width(80.0).hint_text("guess"));
                changed |= r1.changed() || r2.changed();
                if ui.small_button("x").clicked() {
                    remove_ns = Some(i);
                }
            });
        }
        if let Some(idx) = remove_ns {
            app.simulation_profile_editor.nodesets.remove(idx);
            changed = true;
        }
        if ui.small_button("+ Add .nodeset").clicked() {
            app.simulation_profile_editor.nodesets.push((String::new(), String::new()));
            changed = true;
        }
    });

    changed
}
