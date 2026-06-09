use super::NekoSpiceApp;
use crate::canvas;
use crate::viewport::CanvasViewport;
use eframe::egui::{self, Color32, Sense, Stroke, StrokeKind, Vec2};
use osl_kicad::KicadCanvasScene;
use std::path::PathBuf;

const SYMBOL_BROWSER_LIMIT: usize = 80;

impl NekoSpiceApp {
    pub(super) fn draw_library_browser(&mut self, ui: &mut egui::Ui) {
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
            .max_height(300.0)
            .show(ui, |ui| {
                for symbol in filtered.symbols.iter().take(SYMBOL_BROWSER_LIMIT) {
                    let selected = self.selected_symbol_id.as_deref() == Some(symbol.id.as_str());
                    if ui
                        .selectable_label(selected, format!("{}  {}", symbol.id, symbol.name))
                        .clicked()
                    {
                        self.selected_symbol_id = Some(symbol.id.clone());
                        self.selected_symbol_placement = Default::default();
                        self.placement = None;
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
        self.draw_symbol_details(ui);
        self.draw_symbol_placement_controls(ui);
    }

    fn draw_symbol_details(&mut self, ui: &mut egui::Ui) {
        ui.heading("Symbol Details");
        let Some(symbol_id) = self.selected_symbol_id.clone() else {
            ui.label("Select a symbol");
            return;
        };
        let Some((id, library_name, source, bounding_box, footprint_filters, pin_count, preview)) =
            self.library.as_ref().and_then(|library| {
                let symbol = library.symbol(&symbol_id)?;
                let config = self.selected_symbol_placement;
                Some((
                    symbol.id.clone(),
                    symbol.library.clone(),
                    symbol.source.clone(),
                    symbol.bounding_box,
                    symbol.footprint_filters.clone(),
                    symbol.pin_count,
                    library.symbol_preview(&symbol_id, config),
                ))
            })
        else {
            ui.label("Selected symbol is not in the loaded index");
            return;
        };

        ui.label(format!("ID: {id}"));
        ui.label(format!("Library: {library_name}"));
        ui.label(format!("Source: {source}"));
        if let Some(bounds) = bounding_box {
            ui.label(format!(
                "Bounds: {:.2} x {:.2} mm",
                bounds.width(),
                bounds.height()
            ));
        }
        if !footprint_filters.is_empty() {
            ui.label(format!("Footprints: {}", footprint_filters.join(", ")));
        }
        ui.label(format!("Pins: {pin_count}"));
        self.draw_symbol_scope_controls(ui, &symbol_id);
        if ui.button("Place").clicked() {
            self.start_symbol_placement();
        }

        match preview {
            Ok(preview) => {
                ui.add_space(6.0);
                draw_symbol_preview(ui, &preview.scene);
            }
            Err(error) => {
                ui.colored_label(Color32::from_rgb(190, 40, 40), error);
            }
        }
    }

    fn draw_symbol_placement_controls(&mut self, ui: &mut egui::Ui) {
        if let Some(placement) = &self.placement {
            ui.separator();
            ui.label(format!("Placing: {}", placement.symbol_id));
            let mut keep_active = placement.keep_active;
            if ui.checkbox(&mut keep_active, "Repeat").changed()
                && let Some(placement) = &mut self.placement
            {
                placement.keep_active = keep_active;
            }
            if ui.button("Cancel").clicked() {
                self.cancel_symbol_placement();
            }
        }
    }

    fn draw_symbol_scope_controls(&mut self, ui: &mut egui::Ui, symbol_id: &str) {
        let Some((units, body_styles)) = self.library.as_ref().and_then(|library| {
            let symbol = library.symbol(symbol_id)?;
            Some((symbol.units.clone(), symbol.body_styles.clone()))
        }) else {
            return;
        };

        let original_config = self.selected_symbol_placement;

        if !units.is_empty() {
            ui.horizontal(|ui| {
                ui.label("Unit");
                egui::ComboBox::from_id_salt("symbol_unit_selector")
                    .selected_text(format_unit_label(
                        self.selected_symbol_placement.unit,
                        &units,
                    ))
                    .show_ui(ui, |ui| {
                        for unit in &units {
                            let label = format_unit_entry(unit.unit, unit.name.as_deref());
                            ui.selectable_value(
                                &mut self.selected_symbol_placement.unit,
                                unit.unit,
                                label,
                            );
                        }
                    });
            });
        }

        if !body_styles.is_empty() {
            let selected_body_style = self.selected_symbol_placement.body_style.unwrap_or(1);
            let mut body_style = selected_body_style;
            ui.horizontal(|ui| {
                ui.label("Body");
                egui::ComboBox::from_id_salt("symbol_body_style_selector")
                    .selected_text(format_body_style_label(selected_body_style, &body_styles))
                    .show_ui(ui, |ui| {
                        for style in &body_styles {
                            let label =
                                format_body_style_entry(style.body_style, style.name.as_deref());
                            ui.selectable_value(&mut body_style, style.body_style, label);
                        }
                    });
            });
            self.selected_symbol_placement.body_style = Some(body_style);
        } else {
            self.selected_symbol_placement.body_style = None;
        }

        if self.selected_symbol_placement != original_config
            && let Some(placement) = &mut self.placement
            && placement.symbol_id == symbol_id
        {
            placement.config = self.selected_symbol_placement;
        }
    }
}

fn format_unit_label(unit: u32, units: &[osl_kicad::KicadIndexedSymbolUnit]) -> String {
    units
        .iter()
        .find(|candidate| candidate.unit == unit)
        .map(|candidate| format_unit_entry(candidate.unit, candidate.name.as_deref()))
        .unwrap_or_else(|| format!("Unit {unit}"))
}

fn format_unit_entry(unit: u32, name: Option<&str>) -> String {
    match name {
        Some(name) if !name.is_empty() => format!("{unit} {name}"),
        _ => unit.to_string(),
    }
}

fn format_body_style_label(
    body_style: u32,
    body_styles: &[osl_kicad::KicadIndexedSymbolBodyStyle],
) -> String {
    body_styles
        .iter()
        .find(|candidate| candidate.body_style == body_style)
        .map(|candidate| format_body_style_entry(candidate.body_style, candidate.name.as_deref()))
        .unwrap_or_else(|| format!("Style {body_style}"))
}

fn format_body_style_entry(body_style: u32, name: Option<&str>) -> String {
    match name {
        Some(name) if !name.is_empty() => format!("{body_style} {name}"),
        _ => body_style.to_string(),
    }
}

fn draw_symbol_preview(ui: &mut egui::Ui, scene: &KicadCanvasScene) {
    let available_width = ui.available_width().clamp(180.0, 360.0);
    let desired_size = Vec2::new(available_width, 180.0);
    let (rect, _) = ui.allocate_exact_size(desired_size, Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, Color32::from_rgb(252, 252, 252));
    painter.rect_stroke(
        rect,
        0.0,
        Stroke::new(1.0, Color32::from_rgb(210, 216, 222)),
        StrokeKind::Inside,
    );

    let viewport = CanvasViewport::for_rect(rect, scene.bounds);
    let visible_bounds = viewport.visible_world_bounds(rect);
    canvas::draw_scene(&painter, rect, scene, viewport, visible_bounds);
    if let Some(bounds) = scene.bounds {
        canvas::draw_bounds(
            &painter,
            rect,
            viewport,
            bounds,
            Color32::from_rgb(130, 150, 170),
            1.0,
        );
    }
}
