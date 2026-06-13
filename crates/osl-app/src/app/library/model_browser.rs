//! Model browser — tree view of available SPICE models and subcircuits.

use crate::app::NekoSpiceApp;
use super::preview::{draw_spice_preview, draw_symbol_preview};
use super::widgets::{library_metric_card, metadata_row, pin_mapping_row};
use crate::app::localization::UiText;
use crate::app::theme::StudioTheme;
use eframe::egui;
use osl_kicad::KicadIndexedSymbol;

impl NekoSpiceApp {
    /// draw library model browser。
    pub(crate) fn draw_library_model_browser(&mut self, ui: &mut egui::Ui) {
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

        self.draw_model_browser_header(ui, &symbol);
        ui.add_space(8.0);
        if ui.available_width() >= 820.0 {
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_width((ui.available_width() * 0.58).max(420.0));
                    self.draw_library_symbol_and_pins(ui, &symbol);
                });
                ui.add_space(10.0);
                ui.vertical(|ui| {
                    self.draw_library_spice_model_card(ui, &symbol);
                });
            });
        } else {
            self.draw_library_symbol_and_pins(ui, &symbol);
            ui.add_space(8.0);
            self.draw_library_spice_model_card(ui, &symbol);
        }
    }

    fn draw_model_browser_header(&mut self, ui: &mut egui::Ui, symbol: &KicadIndexedSymbol) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.heading(&symbol.id);
                    ui.label(StudioTheme::section_title_for(
                        mode,
                        self.text(UiText::SymbolDetails),
                    ));
                    ui.label(StudioTheme::muted_for(
                        mode,
                        symbol.description.as_deref().unwrap_or(&symbol.library),
                    ));
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                    if ui.button(self.text(UiText::Use)).clicked() {
                        self.selected_symbol_id = Some(symbol.id.clone());
                        self.start_symbol_placement();
                    }
                    if ui.button(self.text(UiText::Compare)).clicked() {
                        self.active_workspace = crate::app::navigation::StudioWorkspace::Reports;
                        self.status_message = Some("Compare mode: view reports".to_string());
                    }
                });
            });
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                metadata_row(ui, mode, self.text(UiText::Category), &symbol.library);
                metadata_row(
                    ui,
                    mode,
                    self.text(UiText::Pins),
                    &symbol.pin_count.to_string(),
                );
                let validation = if symbol.pins.iter().all(|p| !p.electrical_type.is_empty()) {
                    "Passed"
                } else {
                    "Incomplete"
                };
                metadata_row(ui, mode, self.text(UiText::Validation), validation);
            });
        });
    }

    fn draw_library_symbol_and_pins(&mut self, ui: &mut egui::Ui, symbol: &KicadIndexedSymbol) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::SymbolPreview),
            ));
            self.draw_symbol_scope_controls(ui, &symbol.id);
            match self.library.as_ref().and_then(|library| {
                library
                    .symbol_preview(&symbol.id, self.selected_symbol_placement.clone())
                    .ok()
            }) {
                Some(preview) => {
                    draw_symbol_preview(ui, &preview.scene, self.theme_palette().canvas, mode)
                }
                None => {
                    ui.label(StudioTheme::muted_for(mode, self.text(UiText::NoDocument)));
                }
            };
            ui.separator();
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::PinMapping),
            ));
            egui::Grid::new("library_pin_mapping_table")
                .num_columns(4)
                .spacing(egui::vec2(12.0, 4.0))
                .striped(true)
                .show(ui, |ui| {
                    ui.strong("Pin");
                    ui.strong(self.text(UiText::Label));
                    ui.strong(self.text(UiText::Kind));
                    ui.strong("Map To");
                    ui.end_row();
                    for pin in symbol.pins.iter().take(8) {
                        pin_mapping_row(
                            ui,
                            &pin.number,
                            &pin.name,
                            &pin.electrical_type,
                            &pin.name,
                        );
                    }
                });
        });
    }

    fn draw_library_spice_model_card(&self, ui: &mut egui::Ui, symbol: &KicadIndexedSymbol) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::SpiceModel),
            ));
            draw_spice_preview(ui, symbol);
            ui.separator();
            metadata_row(ui, mode, self.text(UiText::References), &symbol.source);
        });
    }

    /// draw library model status cards。
    pub(crate) fn draw_library_model_status_cards(&self, ui: &mut egui::Ui) {
        if ui.available_width() >= 760.0 {
            ui.columns(4, |columns| {
                self.draw_library_model_status_card(&mut columns[0], 0);
                self.draw_library_model_status_card(&mut columns[1], 1);
                self.draw_library_model_status_card(&mut columns[2], 2);
                self.draw_library_model_status_card(&mut columns[3], 3);
            });
        } else {
            ui.columns(2, |columns| {
                self.draw_library_model_status_card(&mut columns[0], 0);
                self.draw_library_model_status_card(&mut columns[1], 1);
            });
            ui.add_space(6.0);
            ui.columns(2, |columns| {
                self.draw_library_model_status_card(&mut columns[0], 2);
                self.draw_library_model_status_card(&mut columns[1], 3);
            });
        }
    }

    fn draw_library_model_status_card(&self, ui: &mut egui::Ui, index: usize) {
        let mode = self.theme_mode();
        let (symbol_count, lib_count, diag_count) = self.library.as_ref().map(|l| {
            let idx = l.index();
            (idx.symbols.len(), idx.libraries.len(), idx.diagnostics.len())
        }).unwrap_or((0, 0, 0));
        match index {
            0 => library_metric_card(ui, mode, self.text(UiText::ModelLibrary),
                &symbol_count.to_string(), "symbols"),
            1 => library_metric_card(ui, mode, self.text(UiText::Verified),
                &lib_count.to_string(), "libraries"),
            2 => {
                let label = if diag_count == 0 { "Passed" } else { "Issues" };
                library_metric_card(ui, mode, self.text(UiText::Validation),
                    label, &format!("{} diagnostics", diag_count))
            }
            _ => library_metric_card(ui, mode, self.text(UiText::VendorUpdates),
                &diag_count.to_string(), "diagnostics"),
        }
    }
}
