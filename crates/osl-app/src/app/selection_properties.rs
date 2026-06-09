use super::NekoSpiceApp;
use eframe::egui;
use osl_kicad::KicadCanvasSymbol;

impl NekoSpiceApp {
    pub(super) fn draw_selection_property_editor(&mut self, ui: &mut egui::Ui) {
        let Some(symbol) = self.selected_symbol_for_property_editor() else {
            return;
        };
        let reference = symbol.reference.clone();
        let value = symbol.value.clone();
        let lib_id = symbol.lib_id.clone();
        let uuid = symbol.uuid.clone();

        ui.separator();
        ui.heading("Properties");
        ui.label(format!("Symbol: {lib_id}"));
        if let Some(uuid) = &uuid {
            ui.monospace(uuid);
        }
        ui.horizontal(|ui| {
            ui.label("Reference");
            ui.text_edit_singleline(&mut self.property_reference);
        });
        ui.horizontal(|ui| {
            ui.label("Value");
            ui.text_edit_singleline(&mut self.property_value);
        });

        let changed = self.property_reference != reference || self.property_value != value;
        if ui
            .add_enabled(changed, egui::Button::new("Apply Properties"))
            .clicked()
        {
            self.apply_selected_symbol_properties(reference);
        }
    }

    pub(super) fn sync_property_editor_from_selection(&mut self) {
        let Some(symbol) = self.selected_symbol_for_property_editor() else {
            self.clear_property_editor();
            return;
        };
        let uuid = symbol.uuid.clone();
        let reference = symbol.reference.clone();
        let value = symbol.value.clone();
        if self.property_selection_uuid == uuid {
            return;
        }
        self.property_selection_uuid = uuid;
        self.property_reference = reference;
        self.property_value = value;
    }

    pub(super) fn clear_property_editor(&mut self) {
        self.property_selection_uuid = None;
        self.property_reference.clear();
        self.property_value.clear();
    }

    fn apply_selected_symbol_properties(&mut self, original_reference: String) {
        let Some(uuid) = self.property_selection_uuid.clone() else {
            self.status_message = Some("No symbol selected for property edit".to_string());
            return;
        };
        let Some(document) = &mut self.document else {
            self.status_message = Some("No editable schematic loaded".to_string());
            return;
        };

        let mut applied = Vec::new();
        let reference = self.property_reference.trim().to_string();
        let value = self.property_value.trim().to_string();
        let reference_changed = reference != original_reference;

        if reference_changed {
            match document.set_symbol_property(
                original_reference.clone(),
                "Reference".to_string(),
                reference.clone(),
            ) {
                Ok(summary) => applied.push(summary),
                Err(error) => {
                    self.status_message = Some(error);
                    return;
                }
            }
        }

        let target_reference = if reference_changed {
            reference.clone()
        } else {
            original_reference
        };
        if let Err(error) = document
            .set_symbol_property(target_reference, "Value".to_string(), value)
            .map(|summary| applied.push(summary))
        {
            self.status_message = Some(error);
            return;
        }

        let scene = document.scene();
        self.selected_hit = scene.item_hit_by_uuid(&uuid);
        self.scene = Some(scene);
        self.sync_property_editor_from_selection();
        self.load_error = None;
        self.status_message = Some(format!("Edited {} symbol properties", applied.len()));
    }

    fn selected_symbol_for_property_editor(&self) -> Option<&KicadCanvasSymbol> {
        let hit = self.selected_hit.as_ref()?;
        if hit.kind != "symbol" {
            return None;
        }
        let uuid = hit.uuid.as_deref()?;
        self.scene
            .as_ref()?
            .symbols
            .iter()
            .find(|symbol| symbol.uuid.as_deref() == Some(uuid))
    }
}
