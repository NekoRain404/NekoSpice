use crate::app::NekoSpiceApp;
use eframe::egui;
use osl_kicad::{KicadIndexedSymbolBodyStyle, KicadIndexedSymbolPin, KicadIndexedSymbolUnit};

impl NekoSpiceApp {
    pub(crate) fn draw_symbol_scope_controls(&mut self, ui: &mut egui::Ui, symbol_id: &str) {
        let Some((units, body_styles, pins)) = self.library.as_ref().and_then(|library| {
            let symbol = library.symbol(symbol_id)?;
            Some((
                symbol.units.clone(),
                symbol.body_styles.clone(),
                symbol.pins.clone(),
            ))
        }) else {
            return;
        };

        let original_config = self.selected_symbol_placement.clone();

        draw_unit_selector(ui, &mut self.selected_symbol_placement.unit, &units);
        draw_body_style_selector(
            ui,
            &mut self.selected_symbol_placement.body_style,
            &body_styles,
        );
        self.retain_valid_pin_alternates(&pins);
        self.draw_pin_alternate_controls(ui, &pins);

        if self.selected_symbol_placement != original_config
            && let Some(placement) = &mut self.placement
            && placement.symbol_id == symbol_id
        {
            placement.config = self.selected_symbol_placement.clone();
        }
    }

    fn draw_pin_alternate_controls(&mut self, ui: &mut egui::Ui, pins: &[KicadIndexedSymbolPin]) {
        let scoped_pins = scoped_alternate_pins(pins, &self.selected_symbol_placement);
        if scoped_pins.is_empty() {
            return;
        }

        ui.separator();
        ui.label("Pin Alternates");
        for pin in scoped_pins {
            let selected = self
                .selected_symbol_placement
                .pin_alternates
                .get(&pin.number)
                .map(String::as_str)
                .unwrap_or("");
            let mut selected = selected.to_string();
            ui.horizontal(|ui| {
                ui.label(format!("{} {}", pin.number, pin.name));
                egui::ComboBox::from_id_salt(format!("pin_alternate_{}", pin.number))
                    .selected_text(format_pin_alternate_label(pin, &selected))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut selected, String::new(), "Default");
                        for alternate in &pin.alternates {
                            ui.selectable_value(
                                &mut selected,
                                alternate.name.clone(),
                                format!(
                                    "{} {} {}",
                                    alternate.name, alternate.electrical_type, alternate.shape
                                ),
                            );
                        }
                    });
            });
            if selected.is_empty() {
                self.selected_symbol_placement
                    .pin_alternates
                    .remove(&pin.number);
            } else {
                self.selected_symbol_placement
                    .pin_alternates
                    .insert(pin.number.clone(), selected);
            }
        }
    }

    fn retain_valid_pin_alternates(&mut self, pins: &[KicadIndexedSymbolPin]) {
        let valid = scoped_alternate_pins(pins, &self.selected_symbol_placement);
        self.selected_symbol_placement
            .pin_alternates
            .retain(|pin_number, alternate| {
                valid.iter().any(|pin| {
                    pin.number == *pin_number
                        && pin
                            .alternates
                            .iter()
                            .any(|candidate| candidate.name == *alternate)
                })
            });
    }
}

fn draw_unit_selector(
    ui: &mut egui::Ui,
    selected_unit: &mut u32,
    units: &[KicadIndexedSymbolUnit],
) {
    if units.is_empty() {
        return;
    }

    ui.horizontal(|ui| {
        ui.label("Unit");
        egui::ComboBox::from_id_salt("symbol_unit_selector")
            .selected_text(format_unit_label(*selected_unit, units))
            .show_ui(ui, |ui| {
                for unit in units {
                    let label = format_unit_entry(unit.unit, unit.name.as_deref());
                    ui.selectable_value(selected_unit, unit.unit, label);
                }
            });
    });
}

fn draw_body_style_selector(
    ui: &mut egui::Ui,
    selected_body_style: &mut Option<u32>,
    body_styles: &[KicadIndexedSymbolBodyStyle],
) {
    if body_styles.is_empty() {
        *selected_body_style = None;
        return;
    }

    let current_body_style = selected_body_style.unwrap_or(1);
    let mut body_style = current_body_style;
    ui.horizontal(|ui| {
        ui.label("Body");
        egui::ComboBox::from_id_salt("symbol_body_style_selector")
            .selected_text(format_body_style_label(current_body_style, body_styles))
            .show_ui(ui, |ui| {
                for style in body_styles {
                    let label = format_body_style_entry(style.body_style, style.name.as_deref());
                    ui.selectable_value(&mut body_style, style.body_style, label);
                }
            });
    });
    *selected_body_style = Some(body_style);
}

fn format_unit_label(unit: u32, units: &[KicadIndexedSymbolUnit]) -> String {
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

fn format_body_style_label(body_style: u32, body_styles: &[KicadIndexedSymbolBodyStyle]) -> String {
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

fn scoped_alternate_pins<'a>(
    pins: &'a [KicadIndexedSymbolPin],
    config: &crate::placement_config::SymbolPlacementConfig,
) -> Vec<&'a KicadIndexedSymbolPin> {
    pins.iter()
        .filter(|pin| {
            symbol_scope_matches(
                pin.unit,
                pin.body_style,
                config.unit,
                config.selected_body_style(),
            ) && !pin.alternates.is_empty()
        })
        .collect()
}

fn symbol_scope_matches(
    item_unit: u32,
    item_body_style: u32,
    selected_unit: u32,
    selected_body_style: u32,
) -> bool {
    (item_unit == 0 || item_unit == selected_unit)
        && (item_body_style == 0 || item_body_style == selected_body_style)
}

fn format_pin_alternate_label(pin: &KicadIndexedSymbolPin, selected: &str) -> String {
    if selected.is_empty() {
        return "Default".to_string();
    }
    pin.alternates
        .iter()
        .find(|alternate| alternate.name == selected)
        .map(|alternate| {
            format!(
                "{} {} {}",
                alternate.name, alternate.electrical_type, alternate.shape
            )
        })
        .unwrap_or_else(|| selected.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::placement_config::SymbolPlacementConfig;
    use osl_kicad::KicadPinAlternate;

    #[test]
    fn scoped_alternate_pins_follow_selected_unit_and_body_style() {
        let pins = vec![
            indexed_pin("1", 1, 1, Vec::new()),
            indexed_pin("2", 2, 1, vec!["ALT2"]),
            indexed_pin("3", 2, 2, vec!["ALT3"]),
        ];
        let config = SymbolPlacementConfig {
            unit: 2,
            body_style: Some(2),
            pin_alternates: Default::default(),
        };

        let scoped = scoped_alternate_pins(&pins, &config);

        assert_eq!(scoped.len(), 1);
        assert_eq!(scoped[0].number, "3");
    }

    #[test]
    fn pin_alternate_label_names_selected_alternate_metadata() {
        let pin = indexed_pin("4", 1, 1, vec!["ALT4"]);

        assert_eq!(
            format_pin_alternate_label(&pin, "ALT4"),
            "ALT4 bidirectional line"
        );
        assert_eq!(format_pin_alternate_label(&pin, ""), "Default");
    }

    fn indexed_pin(
        number: &str,
        unit: u32,
        body_style: u32,
        alternates: Vec<&str>,
    ) -> KicadIndexedSymbolPin {
        KicadIndexedSymbolPin {
            number: number.to_string(),
            name: format!("P{number}"),
            electrical_type: "passive".to_string(),
            shape: "line".to_string(),
            unit,
            body_style,
            alternates: alternates
                .into_iter()
                .map(|name| KicadPinAlternate {
                    name: name.to_string(),
                    electrical_type: "bidirectional".to_string(),
                    shape: "line".to_string(),
                })
                .collect(),
        }
    }
}
