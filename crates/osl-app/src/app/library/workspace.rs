use crate::app::NekoSpiceApp;
use super::widgets::library_filter_tab;
use crate::app::localization::UiText;
use crate::app::theme::StudioTheme;
use eframe::egui;
use std::path::PathBuf;

impl NekoSpiceApp {
    /// draw library center workspace。
    pub(crate) fn draw_library_center_workspace(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("library_center_scroll")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    self.draw_library_workspace_header(ui);
                    ui.add_space(10.0);
                    self.draw_library_filter_tabs(ui);
                    ui.add_space(10.0);

                    // Vendor models toggle
                    ui.horizontal(|ui| {
                        if ui.selectable_label(self.show_vendor_panel, "Vendor Models (TI/ADI)").clicked() {
                            self.show_vendor_panel = !self.show_vendor_panel;
                        }
                    });
                    ui.add_space(4.0);

                    if self.show_vendor_panel {
                        self.draw_vendor_model_panel(ui);
                        ui.add_space(10.0);
                    }

                    self.draw_library_model_status_cards(ui);
                    ui.add_space(10.0);
                    if ui.available_width() >= 920.0 {
                        ui.horizontal_top(|ui| {
                            ui.vertical(|ui| {
                                ui.set_width((ui.available_width() * 0.30).clamp(240.0, 360.0));
                                self.draw_library_symbol_list(ui);
                            });
                            ui.add_space(10.0);
                            ui.vertical(|ui| {
                                self.draw_library_model_browser(ui);
                            });
                        });
                    } else {
                        self.draw_library_symbol_list(ui);
                        ui.add_space(10.0);
                        ui.vertical(|ui| {
                            self.draw_library_model_browser(ui);
                        });
                    }
                });
        });
    }

    fn draw_library_workspace_header(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.heading(self.text(UiText::ModelLibrary));
                ui.label(StudioTheme::muted_for(
                    mode,
                    self.text(UiText::ModelLibraryCaption),
                ));
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                let load = ui.button(self.text(UiText::LoadSymbols)).clicked();
                let response = ui.text_edit_singleline(&mut self.library_table_path);
                if load
                    || (response.lost_focus()
                        && ui.input(|input| input.key_pressed(egui::Key::Enter)))
                {
                    self.load_symbol_library(PathBuf::from(self.library_table_path.trim()));
                }
            });
        });
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.label(StudioTheme::muted_for(mode, self.text(UiText::Search)));
            ui.text_edit_singleline(&mut self.symbol_search);
        });
    }

    fn draw_library_filter_tabs(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        ui.horizontal_wrapped(|ui| {
            for filter in self.library_filters() {
                let active = self.symbol_search.trim().is_empty() && filter.name == "All"
                    || self.symbol_search.trim() == filter.name;
                if library_filter_tab(ui, mode, &filter.name, filter.count, active) {
                    self.symbol_search = if filter.name == "All" {
                        String::new()
                    } else {
                        filter.name
                    };
                }
            }
        });
    }
}
