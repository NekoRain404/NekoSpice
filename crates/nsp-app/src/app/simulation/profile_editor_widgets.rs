//! Shared widget helpers for the simulation profile editor.
//!
//! Pure rendering functions that produce UI elements. Change-aware widgets
//! return `true` when the user modifies a field, allowing callers to persist.

use crate::app::theme::{StudioTheme, StudioThemeMode};
use eframe::egui;

/// Render a section title header inside a profile editor panel.
pub(crate) fn section_header(ui: &mut egui::Ui, mode: StudioThemeMode, title: &str) {
    ui.label(StudioTheme::section_title_for(mode, title));
}

/// Render an editable parameter table with (name, value, unit) columns.
/// Rows are editable inline via `TextEdit::singleline`.
pub(crate) fn param_table(
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
    rows: &mut Vec<(String, String, String)>,
) {
    if rows.is_empty() {
        ui.label(StudioTheme::muted_for(mode, "No entries."));
        return;
    }

    egui::Grid::new(format!("param_table_{}", std::ptr::from_ref(rows) as usize))
        .num_columns(4)
        .spacing([6.0, 4.0])
        .striped(true)
        .show(ui, |ui| {
            // Column headers
            ui.label(StudioTheme::muted_for(mode, ""));
            ui.label(StudioTheme::muted_for(mode, "Name"));
            ui.label(StudioTheme::muted_for(mode, "Value"));
            ui.label(StudioTheme::muted_for(mode, "Unit"));
            ui.end_row();

            let mut remove_index = None;
            for (index, row) in rows.iter_mut().enumerate() {
                if ui.small_button("×").clicked() {
                    remove_index = Some(index);
                }
                ui.add(egui::TextEdit::singleline(&mut row.0).desired_width(80.0));
                ui.add(egui::TextEdit::singleline(&mut row.1).desired_width(80.0));
                ui.add(egui::TextEdit::singleline(&mut row.2).desired_width(60.0));
                ui.end_row();
            }

            if let Some(index) = remove_index {
                rows.remove(index);
            }
        });
}

/// Render a status pill (colored badge) for a given label and state.
#[allow(dead_code)]
pub(crate) fn status_pill(ui: &mut egui::Ui, mode: StudioThemeMode, label: &str, ok: bool) {
    let palette = StudioTheme::palette(mode);
    let color = if ok { palette.success } else { palette.warning };
    egui::Frame::new()
        .fill(palette.panel_soft)
        .corner_radius(10)
        .inner_margin(egui::Margin::same(6))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.colored_label(color, "●");
                ui.label(
                    egui::RichText::new(label)
                        .size(11.0)
                        .color(palette.text_muted),
                );
            });
        });
}

/// Labeled text field with consistent sizing for grid layouts.
///
/// Returns `true` when the user modifies the field value, so callers
/// can trigger persistence or recalculation as needed.
pub(crate) fn labeled_field(
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
    label: &str,
    value: &mut String,
    width: f32,
) -> bool {
    ui.label(StudioTheme::muted_for(mode, label));
    let response = ui.add(egui::TextEdit::singleline(value).desired_width(width));
    ui.end_row();
    response.changed()
}

/// Labeled text field with a placeholder hint. Returns the response for hover text.
pub(crate) fn labeled_edit(
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
    label: &str,
    value: &mut String,
    hint: &str,
) -> egui::Response {
    ui.label(StudioTheme::muted_for(mode, label));
    let response = ui.add(
        egui::TextEdit::singleline(value)
            .desired_width(120.0)
            .hint_text(hint),
    );
    ui.end_row();
    response
}
