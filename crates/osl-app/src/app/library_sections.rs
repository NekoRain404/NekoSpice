use super::NekoSpiceApp;
use super::library_preview::{draw_spice_preview, draw_symbol_preview};
use super::library_widgets::{metadata_row, symbol_list_row};
use super::localization::UiText;
use super::theme::StudioTheme;
use eframe::egui;
use osl_kicad::KicadIndexedSymbol;

const SYMBOL_LIST_LIMIT: usize = 80;

impl NekoSpiceApp {
    pub(super) fn draw_library_symbol_list(&mut self, ui: &mut egui::Ui) {
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

    pub(super) fn draw_library_symbol_workspace(&mut self, ui: &mut egui::Ui) {
        let Some(symbol) = self.selected_library_symbol_snapshot() else {
            let mode = self.theme_mode();
            StudioTheme::panel_frame_for(mode).show(ui, |ui| {
                ui.label(StudioTheme::section_title_for(
                    mode,
                    self.text(UiText::ModelPreview),
                ));
                ui.label(StudioTheme::muted_for(
                    mode,
                    self.text(UiText::NoSelectedItem),
                ));
            });
            return;
        };

        if ui.available_width() < 700.0 {
            self.draw_library_symbol_preview_card(ui, &symbol);
            ui.add_space(8.0);
            self.draw_library_spice_preview_card(ui, &symbol);
        } else {
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_width((ui.available_width() * 0.50).max(300.0));
                    self.draw_library_symbol_preview_card(ui, &symbol);
                });
                ui.add_space(10.0);
                ui.vertical(|ui| {
                    self.draw_library_spice_preview_card(ui, &symbol);
                });
            });
        }
    }

    fn draw_library_symbol_row(
        &mut self,
        ui: &mut egui::Ui,
        mode: super::theme::StudioThemeMode,
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

    fn draw_library_symbol_preview_card(&mut self, ui: &mut egui::Ui, symbol: &KicadIndexedSymbol) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(StudioTheme::section_title_for(
                    mode,
                    self.text(UiText::SymbolPreview),
                ));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(self.text(UiText::Use)).clicked() {
                        self.selected_symbol_id = Some(symbol.id.clone());
                        self.start_symbol_placement();
                    }
                });
            });
            self.draw_symbol_scope_controls(ui, &symbol.id);
            match self.library.as_ref().and_then(|library| {
                library
                    .symbol_preview(&symbol.id, self.selected_symbol_placement.clone())
                    .ok()
            }) {
                Some(preview) => {
                    draw_symbol_preview(ui, &preview.scene, self.theme_palette().canvas)
                }
                None => {
                    ui.label(StudioTheme::muted_for(mode, self.text(UiText::NoDocument)));
                }
            };
        });
    }

    fn draw_library_spice_preview_card(&self, ui: &mut egui::Ui, symbol: &KicadIndexedSymbol) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::ModelPreview),
            ));
            draw_spice_preview(ui, symbol);
        });
    }
}
