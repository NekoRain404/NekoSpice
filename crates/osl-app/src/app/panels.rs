use super::{EditNudgeDirection, NekoSpiceApp};
use eframe::egui::{self, Color32, Vec2};
use std::path::PathBuf;

const SYMBOL_BROWSER_LIMIT: usize = 80;

impl NekoSpiceApp {
    fn draw_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("NekoSpice");
            ui.separator();
            ui.label("Schematic");
            let path_response = ui.text_edit_singleline(&mut self.schematic_path);
            let load_requested = ui.button("Open").clicked()
                || (path_response.lost_focus()
                    && ui.input(|input| input.key_pressed(egui::Key::Enter)));
            if load_requested {
                self.load_schematic(PathBuf::from(self.schematic_path.trim()));
            }
            if ui.button("Fit").clicked() {
                self.viewport
                    .fit_scene(self.scene.as_ref().and_then(|scene| scene.bounds));
            }
            let can_edit = self.document.is_some();
            if ui
                .add_enabled(can_edit, egui::Button::new("Save"))
                .clicked()
            {
                self.save_document();
            }
            let can_delete = self
                .selected_hit
                .as_ref()
                .and_then(|hit| hit.uuid.as_ref())
                .is_some();
            if ui
                .add_enabled(can_edit && can_delete, egui::Button::new("Delete"))
                .clicked()
            {
                self.delete_selected();
            }
            if can_edit && can_delete {
                ui.separator();
                if ui.button("Left").clicked() {
                    self.nudge_selected(EditNudgeDirection::Left);
                }
                if ui.button("Right").clicked() {
                    self.nudge_selected(EditNudgeDirection::Right);
                }
                if ui.button("Up").clicked() {
                    self.nudge_selected(EditNudgeDirection::Up);
                }
                if ui.button("Down").clicked() {
                    self.nudge_selected(EditNudgeDirection::Down);
                }
            }
            if let Some(message) = &self.status_message {
                ui.separator();
                ui.label(message);
            }
        });
    }

    fn draw_sidebar(&self, ui: &mut egui::Ui) {
        ui.heading("Project");
        ui.label("Renderer: wgpu");
        ui.separator();

        if let Some(error) = &self.load_error {
            ui.colored_label(Color32::from_rgb(190, 40, 40), error);
            return;
        }

        let Some(scene) = &self.scene else {
            ui.label("No schematic loaded");
            return;
        };

        ui.label(format!("Source: {}", scene.source));
        egui::Grid::new("project_stats")
            .num_columns(2)
            .spacing(Vec2::new(16.0, 4.0))
            .show(ui, |ui| {
                ui.label("Symbols");
                ui.label(scene.symbols.len().to_string());
                ui.end_row();
                ui.label("Wires");
                ui.label(scene.wires.len().to_string());
                ui.end_row();
                ui.label("Buses");
                ui.label(scene.buses.len().to_string());
                ui.end_row();
                ui.label("Labels");
                ui.label(scene.labels.len().to_string());
                ui.end_row();
                ui.label("Sheets");
                ui.label(scene.sheets.len().to_string());
                ui.end_row();
                ui.label("Graphics");
                ui.label(scene.graphics.len().to_string());
                ui.end_row();
            });
        ui.label(format!("Zoom: {:.1} px/mm", self.viewport.zoom));
        if let Some(document) = &self.document {
            ui.label(format!(
                "Dirty: {}",
                if document.is_dirty() { "yes" } else { "no" }
            ));
        }

        ui.separator();
        ui.heading("Selection");
        if let Some(hit) = &self.selected_hit {
            ui.label(format!("Kind: {}", hit.kind));
            ui.label(format!("Label: {}", hit.label));
            if let Some(uuid) = &hit.uuid {
                ui.monospace(uuid);
            }
        } else {
            ui.label("None");
        }
    }

    fn draw_library_browser(&mut self, ui: &mut egui::Ui) {
        ui.heading("Symbol Library");
        let library_response = ui.text_edit_singleline(&mut self.library_table_path);
        let load_requested = ui.button("Load Symbols").clicked()
            || (library_response.lost_focus()
                && ui.input(|input| input.key_pressed(egui::Key::Enter)));
        if load_requested {
            self.load_symbol_library(PathBuf::from(self.library_table_path.trim()));
        }
        if let Some(error) = &self.library_error {
            ui.colored_label(Color32::from_rgb(190, 40, 40), error);
            return;
        }

        let Some(library) = &self.library else {
            ui.label("No symbol library loaded");
            return;
        };

        ui.label(format!("Table: {}", library.path().display()));
        ui.label(format!(
            "{} libraries, {} symbols, {} diagnostics",
            library.index().libraries.len(),
            library.index().symbols.len(),
            library.index().diagnostics.len()
        ));
        ui.separator();
        ui.label("Search");
        ui.text_edit_singleline(&mut self.symbol_search);

        let filtered = library.filtered_index(&self.symbol_search);
        ui.label(format!("{} matches", filtered.symbols.len()));
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .max_height(420.0)
            .show(ui, |ui| {
                for symbol in filtered.symbols.iter().take(SYMBOL_BROWSER_LIMIT) {
                    let selected = self.selected_symbol_id.as_deref() == Some(symbol.id.as_str());
                    if ui
                        .selectable_label(selected, format!("{}  {}", symbol.id, symbol.name))
                        .clicked()
                    {
                        self.selected_symbol_id = Some(symbol.id.clone());
                    }
                    ui.label(format!(
                        "{} pins, {} units, {} graphics",
                        symbol.pin_count, symbol.unit_count, symbol.graphic_count
                    ));
                    if let Some(description) = &symbol.description {
                        ui.label(description);
                    }
                    ui.separator();
                }
                if filtered.symbols.len() > SYMBOL_BROWSER_LIMIT {
                    ui.label(format!(
                        "{} more symbols hidden by the browser limit",
                        filtered.symbols.len() - SYMBOL_BROWSER_LIMIT
                    ));
                }
            });

        ui.separator();
        ui.heading("Symbol Details");
        if let Some(symbol_id) = &self.selected_symbol_id {
            if let Some(symbol) = library.symbol(symbol_id) {
                ui.label(format!("ID: {}", symbol.id));
                ui.label(format!("Library: {}", symbol.library));
                ui.label(format!("Source: {}", symbol.source));
                if let Some(bounds) = symbol.bounding_box {
                    ui.label(format!(
                        "Bounds: {:.2} x {:.2} mm",
                        bounds.width(),
                        bounds.height()
                    ));
                }
                if !symbol.footprint_filters.is_empty() {
                    ui.label(format!(
                        "Footprints: {}",
                        symbol.footprint_filters.join(", ")
                    ));
                }
                ui.label(format!("Pins: {}", symbol.pin_count));
            }
        } else {
            ui.label("Select a symbol");
        }
    }
}

impl eframe::App for NekoSpiceApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Panel::top("nekospice_toolbar")
            .exact_size(46.0)
            .show_inside(ui, |ui| {
                ui.add_space(4.0);
                self.draw_toolbar(ui);
            });
        egui::Panel::left("nekospice_project_panel")
            .default_size(280.0)
            .min_size(220.0)
            .max_size(380.0)
            .show_inside(ui, |ui| {
                ui.add_space(8.0);
                self.draw_sidebar(ui);
            });
        egui::Panel::right("nekospice_library_panel")
            .default_size(340.0)
            .min_size(260.0)
            .max_size(480.0)
            .show_inside(ui, |ui| {
                ui.add_space(8.0);
                self.draw_library_browser(ui);
            });
        egui::CentralPanel::default().show_inside(ui, |ui| {
            self.draw_canvas(ui);
        });
    }
}
