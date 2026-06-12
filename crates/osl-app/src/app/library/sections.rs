use crate::app::NekoSpiceApp;
use super::widgets::{metadata_row, symbol_list_row};
use crate::app::localization::UiText;
use crate::app::theme::StudioTheme;
use eframe::egui;
use osl_kicad::KicadIndexedSymbol;

const SYMBOL_LIST_LIMIT: usize = 80;

impl NekoSpiceApp {
    pub(crate) fn draw_library_symbol_list(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::Symbols),
            ));
            let symbols = self.filtered_library_symbols();
            metadata_row(
                ui,
                mode,
                self.text(UiText::Matches),
                &symbols.len().to_string(),
            );
            ui.separator();
            egui::ScrollArea::vertical()
                .id_salt("library_workspace_symbol_list")
                .auto_shrink([false, false])
                .max_height((ui.available_height() - 32.0).max(280.0))
                .show(ui, |ui| {
                    for symbol in symbols.into_iter().take(SYMBOL_LIST_LIMIT) {
                        self.draw_library_symbol_row(ui, mode, symbol);
                        ui.add_space(6.0);
                    }
                });
        });
    }

    fn draw_library_symbol_row(
        &mut self,
        ui: &mut egui::Ui,
        mode: crate::app::theme::StudioThemeMode,
        symbol: KicadIndexedSymbol,
    ) {
        let active = self.selected_symbol_id.as_deref() == Some(symbol.id.as_str());
        let detail = symbol
            .description
            .clone()
            .unwrap_or_else(|| symbol.library.clone());
        let stats = format!("{} pins", symbol.pin_count);
        if symbol_list_row(ui, mode, &symbol.id, &detail, &stats, active) {
            self.selected_symbol_id = Some(symbol.id);
            self.selected_symbol_placement = Default::default();
            self.placement = None;
        }
    }
}
