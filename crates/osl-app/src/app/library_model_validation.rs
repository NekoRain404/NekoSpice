use super::NekoSpiceApp;
use super::library_widgets::{metadata_row, validation_row};
use super::localization::UiText;
use super::theme::StudioTheme;
use eframe::egui;

impl NekoSpiceApp {
    pub(super) fn draw_library_validation_panel(&self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::Validation),
            ));
            metadata_row(ui, mode, self.text(UiText::LibraryStatus), "Ready");
            metadata_row(ui, mode, self.text(UiText::SymbolLibrary), "KiCad");
            ui.separator();
            validation_row(ui, mode, "Syntax Check", "Passed", true);
            validation_row(ui, mode, "Pin Count", "8 / 8 pins", true);
            validation_row(ui, mode, "Subckt Reference", "Valid", true);
            validation_row(ui, mode, "DC Operating Point", "Stable", true);
            validation_row(ui, mode, "Temperature Sweep", "Stable", true);
            ui.add_space(6.0);
            let _ = ui.button(self.text(UiText::Run));
        });
        ui.add_space(8.0);
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::ModelSummary),
            ));
            metadata_row(ui, mode, self.text(UiText::Category), "Op-Amp");
            metadata_row(ui, mode, "Supply Range", "+/-2.5 V to +/-18 V");
            metadata_row(ui, mode, "GBW", "20 MHz");
            metadata_row(ui, mode, "Slew Rate", "9 V/us");
            metadata_row(ui, mode, "Input Bias", "+/-20 fA");
            metadata_row(ui, mode, "Noise", "4.1 nV/sqrtHz");
            metadata_row(ui, mode, self.text(UiText::Bounds), "12.7 x 10.2 mm");
        });
    }
}
