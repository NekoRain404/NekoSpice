//! 厂商模型浏览面板。显示 TI/ADI 导入的 SPICE 模型列表。
//!
use crate::app::NekoSpiceApp;
use crate::app::theme::StudioTheme;
use eframe::egui;

impl NekoSpiceApp {
    /// 绘制厂商模型浏览面板
    pub(crate) fn draw_vendor_model_panel(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let palette = self.theme_palette();

        // Header
        ui.horizontal(|ui| {
            ui.heading(StudioTheme::section_title_for(
                mode,
                "Vendor Models (TI / ADI)",
            ));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Import Dir").clicked() {
                    // Trigger directory import
                    if !self.vendor_model_path.is_empty() {
                        self.import_vendor_models(&self.vendor_model_path.clone());
                    }
                }
            });
        });
        ui.add_space(4.0);

        // Path input
        ui.horizontal(|ui| {
            ui.label(StudioTheme::muted_for(mode, "Model Directory:"));
            ui.text_edit_singleline(&mut self.vendor_model_path);
        });
        ui.add_space(4.0);

        // Stats
        let subckt_count = self.vendor_catalog.subckts.len();
        let model_count = self.vendor_catalog.models.len();
        ui.label(StudioTheme::muted_for(
            mode,
            format!("{} subcircuits, {} models", subckt_count, model_count),
        ));
        ui.add_space(4.0);

        // Search
        ui.horizontal(|ui| {
            ui.label(StudioTheme::muted_for(mode, "Search:"));
            ui.text_edit_singleline(&mut self.vendor_search);
        });
        ui.add_space(8.0);

        // Subcircuit list
        if subckt_count > 0 {
            ui.label(StudioTheme::section_title_for(mode, "Subcircuits"));
            ui.add_space(4.0);

            egui::ScrollArea::vertical()
                .id_salt("vendor_subckt_scroll")
                .max_height(300.0)
                .show(ui, |ui| {
                    let search = self.vendor_search.trim().to_lowercase();
                    for (name, entry) in &self.vendor_catalog.subckts {
                        if !search.is_empty() && !name.to_lowercase().contains(&search) {
                            continue;
                        }
                        ui.horizontal(|ui| {
                            // Vendor badge
                            let badge_color = match entry.vendor {
                                osl_model::VendorKind::Ti => palette.warning,
                                osl_model::VendorKind::Adi => palette.accent,
                                osl_model::VendorKind::Generic => palette.text_muted,
                            };
                            ui.colored_label(
                                badge_color,
                                format!("[{}]", entry.vendor.name().chars().take(2).collect::<String>()),
                            );
                            ui.label(StudioTheme::muted_for(mode, &entry.name));
                            ui.label(StudioTheme::muted_for(
                                mode,
                                format!("({} pins)", entry.pins.len()),
                            ));
                        });
                    }
                });
        }

        // Model list
        if model_count > 0 {
            ui.add_space(8.0);
            ui.label(StudioTheme::section_title_for(mode, ".MODEL Statements"));
            ui.add_space(4.0);

            egui::ScrollArea::vertical()
                .id_salt("vendor_model_scroll")
                .max_height(200.0)
                .show(ui, |ui| {
                    let search = self.vendor_search.trim().to_lowercase();
                    for (name, entry) in &self.vendor_catalog.models {
                        if !search.is_empty() && !name.to_lowercase().contains(&search) {
                            continue;
                        }
                        ui.horizontal(|ui| {
                            let badge_color = match entry.vendor {
                                osl_model::VendorKind::Ti => palette.warning,
                                osl_model::VendorKind::Adi => palette.accent,
                                osl_model::VendorKind::Generic => palette.text_muted,
                            };
                            ui.colored_label(
                                badge_color,
                                format!("[{}]", entry.vendor.name().chars().take(2).collect::<String>()),
                            );
                            ui.label(StudioTheme::muted_for(mode, &entry.name));
                        });
                    }
                });
        }

        // Empty state
        if subckt_count == 0 && model_count == 0 {
            ui.add_space(20.0);
            ui.vertical_centered(|ui| {
                ui.label(StudioTheme::muted_for(
                    mode,
                    "No vendor models loaded. Enter a directory path above and click 'Import Dir'.",
                ));
                ui.add_space(8.0);
                ui.label(StudioTheme::muted_for(
                    mode,
                    "Supported: TI (.lib), ADI/LTspice (.lib, .mod), Generic SPICE models",
                ));
            });
        }
    }
}
