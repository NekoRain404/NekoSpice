//! Vendor model browsing panel — displays TI/ADI SPICE models
//! imported from external directories.
//!
//! Users can browse subcircuits and .model statements, search/filter
//! the catalog, and import models into the current simulation profile
//! via the "+" button on each model entry.

use crate::app::NekoSpiceApp;
use crate::app::theme::StudioTheme;
use eframe::egui;

/// Display row for a vendor model entry.
struct VendorRow {
    name: String,
    badge: String,
    badge_color: egui::Color32,
    detail: String,
}

impl NekoSpiceApp {
    /// Draw the vendor model browsing panel.
    pub(crate) fn draw_vendor_model_panel(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let palette = self.theme_palette();

        // ── Header with directory picker ──
        ui.horizontal(|ui| {
            ui.heading(StudioTheme::section_title_for(
                mode,
                "Vendor Models (TI / ADI)",
            ));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Browse...").on_hover_text("Select model directory").clicked() {
                    if let Some(dir) = rfd::FileDialog::new()
                        .set_title("Select SPICE Model Directory")
                        .pick_folder()
                    {
                        let dir_str = dir.display().to_string();
                        self.vendor_model_path = dir_str;
                        self.import_vendor_models(&self.vendor_model_path.clone());
                    }
                }
            });
        });
        ui.add_space(4.0);

        // ── Path input + import ──
        ui.horizontal(|ui| {
            ui.label(StudioTheme::muted_for(mode, "Path:"));
            ui.text_edit_singleline(&mut self.vendor_model_path);
            if ui.small_button("Import").clicked() {
                if !self.vendor_model_path.is_empty() {
                    self.import_vendor_models(&self.vendor_model_path.clone());
                }
            }
        });
        ui.add_space(4.0);

        // ── Stats + Search ──
        let subckt_count = self.vendor_catalog.subckts.len();
        let model_count = self.vendor_catalog.models.len();
        ui.label(StudioTheme::muted_for(
            mode,
            format!("{} subcircuits, {} models", subckt_count, model_count),
        ));
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(StudioTheme::muted_for(mode, "Search:"));
            ui.text_edit_singleline(&mut self.vendor_search)
                .on_hover_text("Filter models by name");
        });
        ui.add_space(8.0);

        // ── Build display rows from catalog (collect data to avoid borrow conflicts) ──
        let search = self.vendor_search.trim().to_lowercase();
        let subckt_rows: Vec<VendorRow> = self.vendor_catalog.subckts.iter()
            .filter(|(name, _)| search.is_empty() || name.to_lowercase().contains(&search))
            .map(|(name, entry)| {
                let badge_color = match entry.vendor {
                    osl_model::VendorKind::Ti => palette.warning,
                    osl_model::VendorKind::Adi => palette.accent,
                    osl_model::VendorKind::Generic => palette.text_muted,
                };
                VendorRow {
                    name: name.clone(),
                    badge: entry.vendor.name().chars().take(2).collect(),
                    badge_color,
                    detail: format!("({} pins)", entry.pins.len()),
                }
            })
            .collect();
        let model_rows: Vec<VendorRow> = self.vendor_catalog.models.iter()
            .filter(|(name, _)| search.is_empty() || name.to_lowercase().contains(&search))
            .map(|(name, entry)| {
                let badge_color = match entry.vendor {
                    osl_model::VendorKind::Ti => palette.warning,
                    osl_model::VendorKind::Adi => palette.accent,
                    osl_model::VendorKind::Generic => palette.text_muted,
                };
                VendorRow {
                    name: name.clone(),
                    badge: entry.vendor.name().chars().take(2).collect(),
                    badge_color,
                    detail: String::new(),
                }
            })
            .collect();

        // ── Subcircuit List ──
        if !subckt_rows.is_empty() {
            ui.label(StudioTheme::section_title_for(mode, "Subcircuits"));
            ui.add_space(4.0);
            let mut clicked_subckt: Option<String> = None;
            egui::ScrollArea::vertical()
                .id_salt("vendor_subckt_scroll")
                .max_height(300.0)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for row in &subckt_rows {
                        ui.horizontal(|ui| {
                            ui.colored_label(row.badge_color, format!("[{}]", row.badge));
                            ui.label(StudioTheme::muted_for(mode, &row.name));
                            if !row.detail.is_empty() {
                                ui.label(StudioTheme::muted_for(mode, &row.detail));
                            }
                            if ui.small_button("+").on_hover_text("Add .subckt to simulation").clicked() {
                                clicked_subckt = Some(row.name.clone());
                            }
                        });
                    }
                });
            if let Some(name) = clicked_subckt {
                self.add_vendor_subckt_to_simulation(&name);
            }
        }

        // ── Model Statement List ──
        if !model_rows.is_empty() {
            ui.add_space(8.0);
            ui.label(StudioTheme::section_title_for(mode, ".MODEL Statements"));
            ui.add_space(4.0);
            let mut clicked_model: Option<String> = None;
            egui::ScrollArea::vertical()
                .id_salt("vendor_model_scroll")
                .max_height(200.0)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for row in &model_rows {
                        ui.horizontal(|ui| {
                            ui.colored_label(row.badge_color, format!("[{}]", row.badge));
                            ui.label(StudioTheme::muted_for(mode, &row.name));
                            if ui.small_button("+").on_hover_text("Add to simulation profile").clicked() {
                                clicked_model = Some(row.name.clone());
                            }
                        });
                    }
                });
            if let Some(name) = clicked_model {
                self.add_vendor_model_to_simulation(&name);
            }
        }

        // ── Empty State ──
        if subckt_rows.is_empty() && model_rows.is_empty() {
            ui.add_space(20.0);
            ui.vertical_centered(|ui| {
                ui.label(StudioTheme::muted_for(mode, "No vendor models loaded."));
                ui.add_space(8.0);
                ui.label(StudioTheme::muted_for(
                    mode,
                    "Click 'Browse...' to select a SPICE model directory.",
                ));
                ui.add_space(4.0);
                ui.label(StudioTheme::muted_for(
                    mode,
                    "Supported: TI (.lib), ADI/LTspice (.lib, .mod), Generic SPICE models",
                ));
            });
        }
    }

    /// Add a vendor subcircuit to the simulation profile's model parameters.
    fn add_vendor_subckt_to_simulation(&mut self, name: &str) {
        self.simulation_profile_editor
            .model_params
            .push((name.to_string(), "subckt".to_string(), "vendor".to_string()));
        self.status_message = Some(format!("Added subcircuit '{}' to simulation", name));
    }

    /// Add a vendor .model statement to the simulation profile.
    fn add_vendor_model_to_simulation(&mut self, name: &str) {
        self.simulation_profile_editor
            .model_params
            .push((name.to_string(), "model".to_string(), "vendor".to_string()));
        self.status_message = Some(format!("Added model '{}' to simulation", name));
    }
}
