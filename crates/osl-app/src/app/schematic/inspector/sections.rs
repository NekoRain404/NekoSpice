use crate::app::localization::UiText;
use super::widgets::{
    compact_action, property_row, section_caption, status_pill,
};
use crate::app::theme::StudioTheme;
use crate::app::widgets::metric_row;
use crate::app::{EditNudgeDirection, NekoSpiceApp};
use eframe::egui;

impl NekoSpiceApp {
    /// draw schematic properties tab。
    pub(crate) fn draw_schematic_properties_tab(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::SelectionProperties),
            ));
            if let Some(hit) = &self.selected_hit {
                property_row(ui, mode, self.text(UiText::Kind), &hit.kind);
                property_row(ui, mode, self.text(UiText::Label), &hit.label);
                if let Some(uuid) = &hit.uuid {
                    ui.label(StudioTheme::muted_for(mode, self.text(UiText::Uuid)));
                    ui.monospace(uuid);
                }
            } else {
                section_caption(ui, mode, self.text(UiText::NoSelectedItem));
            }
            self.draw_selection_property_editor(ui);
        });

        ui.add_space(8.0);
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::CrossProbe),
            ));
            self.draw_cross_probe_summary(ui);
        });
    }

    /// draw schematic kicad inspector tab。
    pub(crate) fn draw_schematic_kicad_inspector_tab(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::DocumentStructure),
            ));
            let Some(scene) = &self.scene else {
                section_caption(ui, mode, self.text(UiText::NoSchematicLoaded));
                return;
            };
            metric_row(
                ui,
                mode,
                self.text(UiText::Symbols),
                &scene.symbols.len().to_string(),
            );
            metric_row(
                ui,
                mode,
                self.text(UiText::Wires),
                &scene.wires.len().to_string(),
            );
            metric_row(
                ui,
                mode,
                self.text(UiText::Buses),
                &scene.buses.len().to_string(),
            );
            metric_row(
                ui,
                mode,
                self.text(UiText::Labels),
                &scene.labels.len().to_string(),
            );
            metric_row(
                ui,
                mode,
                self.text(UiText::Sheets),
                &scene.sheets.len().to_string(),
            );
            metric_row(
                ui,
                mode,
                self.text(UiText::Graphics),
                &scene.graphics.len().to_string(),
            );
            metric_row(
                ui,
                mode,
                self.text(UiText::Groups),
                &scene.groups.len().to_string(),
            );
        });

        ui.add_space(8.0);
        self.draw_document_diagnostics_panel(ui, 180.0);
    }

    /// draw schematic libraries tab。
    pub(crate) fn draw_schematic_libraries_tab(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::LibraryScope),
            ));
            let Some((path, library_count, symbol_count, diagnostic_count)) =
                self.library.as_ref().map(|library| {
                    (
                        library.path().display().to_string(),
                        library.index().libraries.len(),
                        library.index().symbols.len(),
                        library.index().diagnostics.len(),
                    )
                })
            else {
                section_caption(ui, mode, self.text(UiText::NoLibraryLoaded));
                return;
            };
            ui.label(StudioTheme::muted_for(mode, path));
            metric_row(
                ui,
                mode,
                self.text(UiText::Libraries),
                &library_count.to_string(),
            );
            metric_row(
                ui,
                mode,
                self.text(UiText::Symbols),
                &symbol_count.to_string(),
            );
            metric_row(
                ui,
                mode,
                self.text(UiText::Diagnostics),
                &diagnostic_count.to_string(),
            );
            ui.separator();
            section_caption(ui, mode, self.text(UiText::LibrarySearchHint));
            ui.text_edit_singleline(&mut self.symbol_search);
            let matches = self
                .library
                .as_ref()
                .map(|library| library.filtered_index(&self.symbol_search).symbols.len())
                .unwrap_or_default();
            property_row(ui, mode, self.text(UiText::Matches), &matches.to_string());
        });
    }

    fn draw_cross_probe_summary(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let palette = self.theme_palette();
        let can_edit = self.document.is_some();
        let can_target = self
            .selected_hit
            .as_ref()
            .and_then(|hit| hit.uuid.as_ref())
            .is_some();

        if !can_target {
            section_caption(ui, mode, self.text(UiText::NoSelection));
            return;
        }

        ui.horizontal_wrapped(|ui| {
            if compact_action(ui, mode, self.text(UiText::Fit)) {
                self.viewport
                    .fit_scene(self.scene.as_ref().and_then(|scene| scene.bounds));
            }
            if ui
                .add_enabled(
                    can_edit && can_target,
                    egui::Button::new(self.text(UiText::DeleteSelected)),
                )
                .clicked()
            {
                self.delete_selected();
            }
        });
        ui.horizontal_wrapped(|ui| {
            for (label, direction) in [
                (self.text(UiText::Left), EditNudgeDirection::Left),
                (self.text(UiText::Right), EditNudgeDirection::Right),
                (self.text(UiText::Up), EditNudgeDirection::Up),
                (self.text(UiText::Down), EditNudgeDirection::Down),
            ] {
                if ui
                    .add_enabled(can_edit && can_target, egui::Button::new(label))
                    .clicked()
                {
                    self.nudge_selected(direction);
                }
            }
        });
        ui.add_space(4.0);
        status_pill(ui, mode, self.text(UiText::CanvasLinked), palette.success);
    }
}
