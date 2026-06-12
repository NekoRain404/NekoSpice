use crate::app::NekoSpiceApp;
use super::widgets::metadata_row;
use crate::app::localization::UiText;
use crate::app::theme::StudioTheme;
use eframe::egui;

impl NekoSpiceApp {
    pub(crate) fn draw_library_workspace_panel(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        ui.heading(self.text(UiText::SymbolLibrary));
        ui.label(StudioTheme::muted_for(
            mode,
            self.text(UiText::ModelLibraryCaption),
        ));
        ui.add_space(8.0);
        self.draw_library_status_card(ui);
        ui.add_space(8.0);
        self.draw_library_selected_symbol_card(ui);
        ui.add_space(8.0);
        self.draw_library_validation_card(ui);
    }

    fn draw_library_status_card(&self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::LibraryStatus),
            ));
            let Some(library) = &self.library else {
                ui.label(StudioTheme::muted_for(
                    mode,
                    self.text(UiText::NoLibraryLoaded),
                ));
                return;
            };
            metadata_row(
                ui,
                mode,
                self.text(UiText::Libraries),
                &library.index().libraries.len().to_string(),
            );
            metadata_row(
                ui,
                mode,
                self.text(UiText::Symbols),
                &library.index().symbols.len().to_string(),
            );
            metadata_row(
                ui,
                mode,
                self.text(UiText::Diagnostics),
                &library.index().diagnostics.len().to_string(),
            );
            ui.separator();
            ui.label(StudioTheme::muted_for(
                mode,
                library.path().display().to_string(),
            ));
        });
    }

    fn draw_library_selected_symbol_card(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::SymbolDetails),
            ));
            let Some(symbol) = self.selected_library_symbol_snapshot() else {
                ui.label(StudioTheme::muted_for(
                    mode,
                    self.text(UiText::NoSelectedItem),
                ));
                return;
            };
            metadata_row(ui, mode, "ID", &symbol.id);
            metadata_row(ui, mode, self.text(UiText::Libraries), &symbol.library);
            metadata_row(
                ui,
                mode,
                self.text(UiText::Pins),
                &symbol.pin_count.to_string(),
            );
            metadata_row(
                ui,
                mode,
                self.text(UiText::Units),
                &symbol.unit_count.to_string(),
            );
            metadata_row(
                ui,
                mode,
                self.text(UiText::Graphics),
                &symbol.graphic_count.to_string(),
            );
            if let Some(bounds) = symbol.bounding_box {
                metadata_row(
                    ui,
                    mode,
                    self.text(UiText::Bounds),
                    &format!("{:.2} x {:.2} mm", bounds.width(), bounds.height()),
                );
            }
            if !symbol.footprint_filters.is_empty() {
                ui.separator();
                ui.label(StudioTheme::muted_for(
                    mode,
                    symbol.footprint_filters.join(", "),
                ));
            }
        });
    }

    fn draw_library_validation_card(&self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let palette = self.theme_palette();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::Validation),
            ));
            let status = if self.library_error.is_none() && self.library.is_some() {
                (self.text(UiText::Ready), palette.success)
            } else {
                (self.text(UiText::Missing), palette.warning)
            };
            ui.colored_label(status.1, status.0);
            if let Some(error) = &self.library_error {
                ui.colored_label(palette.danger, error);
            }
        });
    }
}
