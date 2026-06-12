use crate::app::NekoSpiceApp;
use super::widgets::{metadata_row, validation_row};
use crate::app::localization::UiText;
use crate::app::theme::StudioTheme;
use eframe::egui;

impl NekoSpiceApp {
    /// draw library validation panel。
    pub(crate) fn draw_library_validation_panel(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::Validation),
            ));
            let (sym_count, lib_count, diag_count) = self.library.as_ref().map(|l| {
                let idx = l.index();
                (idx.symbols.len(), idx.libraries.len(), idx.diagnostics.len())
            }).unwrap_or((0, 0, 0));
            let status = if diag_count == 0 { "Passed" } else { &format!("{} issues", diag_count) };
            metadata_row(ui, mode, self.text(UiText::LibraryStatus), status);
            metadata_row(ui, mode, self.text(UiText::SymbolLibrary),
                &format!("{} libraries, {} symbols", lib_count, sym_count));
            ui.separator();
            validation_row(ui, mode, "Symbol Count", &sym_count.to_string(), diag_count == 0);
            validation_row(ui, mode, "Library Count", &lib_count.to_string(), true);
            validation_row(ui, mode, "Diagnostics", &diag_count.to_string(), diag_count == 0);
            ui.add_space(6.0);
            if ui.button(self.text(UiText::Run)).clicked() {
                self.status_message = Some("Library validation: running checks...".to_string());
            }
        });
        ui.add_space(8.0);
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::ModelSummary),
            ));
            if let Some(sym) = self.selected_library_symbol_snapshot() {
                metadata_row(ui, mode, self.text(UiText::Category), &sym.library);
                metadata_row(ui, mode, "Pins", &sym.pin_count.to_string());
                if let Some(desc) = &sym.description {
                    metadata_row(ui, mode, "Description", desc);
                }
                metadata_row(ui, mode, "Source", &sym.source);
            } else {
                metadata_row(ui, mode, self.text(UiText::NoSelectedItem), "-");
            }
        });
    }
}
