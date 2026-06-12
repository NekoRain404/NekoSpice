use crate::app::NekoSpiceApp;
use crate::app::localization::UiText;
use eframe::egui;
use osl_kicad::KicadCanvasSymbol;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SelectionPropertyEditorState {
    reference: String,
    value: String,
    mirror: SymbolMirrorSelection,
    selection_uuid: Option<String>,
}

impl Default for SelectionPropertyEditorState {
    fn default() -> Self {
        Self {
            reference: String::new(),
            value: String::new(),
            mirror: SymbolMirrorSelection::None,
            selection_uuid: None,
        }
    }
}

impl SelectionPropertyEditorState {
    fn clear(&mut self) {
        self.selection_uuid = None;
        self.reference.clear();
        self.value.clear();
        self.mirror = SymbolMirrorSelection::None;
    }

    fn sync_from_snapshot(
        &mut self,
        uuid: Option<String>,
        snapshot: SelectedSymbolPropertySnapshot,
    ) {
        self.selection_uuid = uuid;
        self.reference = snapshot.reference;
        self.value = snapshot.value;
        self.mirror = snapshot.mirror;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SelectedSymbolPropertySnapshot {
    reference: String,
    value: String,
    mirror: SymbolMirrorSelection,
}

impl SelectedSymbolPropertySnapshot {
    fn from_symbol(symbol: &KicadCanvasSymbol) -> Self {
        Self {
            reference: symbol.reference.clone(),
            value: symbol.value.clone(),
            mirror: SymbolMirrorSelection::from_kicad(symbol.mirror.as_deref()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SymbolMirrorSelection {
    None,
    X,
    Y,
    Xy,
}

impl SymbolMirrorSelection {
    const ALL: [Self; 4] = [Self::None, Self::X, Self::Y, Self::Xy];

    fn label(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::X => "x",
            Self::Y => "y",
            Self::Xy => "xy",
        }
    }

    fn from_kicad(mirror: Option<&str>) -> Self {
        match mirror {
            Some("x") => Self::X,
            Some("y") => Self::Y,
            Some("x y") | Some("y x") => Self::Xy,
            _ => Self::None,
        }
    }

    fn to_kicad(self) -> Option<String> {
        match self {
            Self::None => None,
            Self::X => Some("x".to_string()),
            Self::Y => Some("y".to_string()),
            Self::Xy => Some("x y".to_string()),
        }
    }
}

impl NekoSpiceApp {
    /// draw selection property editor。
    pub(crate) fn draw_selection_property_editor(&mut self, ui: &mut egui::Ui) {
        let Some(symbol) = self.selected_symbol_for_property_editor() else {
            return;
        };
        let original = SelectedSymbolPropertySnapshot::from_symbol(symbol);
        let lib_id = symbol.lib_id.clone();
        let uuid = symbol.uuid.clone();

        ui.separator();
        ui.heading(self.text(UiText::Properties));
        ui.label(format!("Symbol: {lib_id}"));
        if let Some(uuid) = &uuid {
            ui.monospace(uuid);
        }
        ui.horizontal(|ui| {
            ui.label("Reference");
            ui.text_edit_singleline(&mut self.selection_properties.reference);
        });
        ui.horizontal(|ui| {
            ui.label("Value");
            ui.text_edit_singleline(&mut self.selection_properties.value);
        });
        ui.horizontal(|ui| {
            ui.label("Mirror");
            egui::ComboBox::from_id_salt("selected_symbol_mirror")
                .selected_text(self.selection_properties.mirror.label())
                .show_ui(ui, |ui| {
                    for option in SymbolMirrorSelection::ALL {
                        ui.selectable_value(
                            &mut self.selection_properties.mirror,
                            option,
                            option.label(),
                        );
                    }
                });
        });

        let changed = self.selection_properties.reference != original.reference
            || self.selection_properties.value != original.value
            || self.selection_properties.mirror != original.mirror;
        if ui
            .add_enabled(
                changed,
                egui::Button::new(self.text(UiText::ApplyProperties)),
            )
            .clicked()
        {
            self.apply_selected_symbol_properties(original);
        }
    }

    /// sync property editor from selection。
    pub(crate) fn sync_property_editor_from_selection(&mut self) {
        let Some(symbol) = self.selected_symbol_for_property_editor() else {
            self.clear_property_editor();
            return;
        };
        let uuid = symbol.uuid.clone();
        let snapshot = SelectedSymbolPropertySnapshot::from_symbol(symbol);
        if self.selection_properties.selection_uuid == uuid {
            return;
        }
        self.selection_properties.sync_from_snapshot(uuid, snapshot);
    }

    /// clear property editor。
    pub(crate) fn clear_property_editor(&mut self) {
        self.selection_properties.clear();
    }

    fn apply_selected_symbol_properties(&mut self, original: SelectedSymbolPropertySnapshot) {
        let Some(uuid) = self.selection_properties.selection_uuid.clone() else {
            self.status_message = Some("No symbol selected for property edit".to_string());
            return;
        };
        let Some(document) = &mut self.document else {
            self.status_message = Some("No editable schematic loaded".to_string());
            return;
        };

        let mut applied = Vec::new();
        let reference = self.selection_properties.reference.trim().to_string();
        let value = self.selection_properties.value.trim().to_string();
        let mirror = self.selection_properties.mirror.to_kicad();
        let reference_changed = reference != original.reference;
        let value_changed = value != original.value;
        let mirror_changed = self.selection_properties.mirror != original.mirror;

        if reference_changed {
            match document.set_symbol_property(
                original.reference.clone(),
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
            original.reference
        };
        if value_changed {
            match document.set_symbol_property(target_reference.clone(), "Value".to_string(), value)
            {
                Ok(summary) => applied.push(summary),
                Err(error) => {
                    self.status_message = Some(error);
                    return;
                }
            }
        }
        if mirror_changed {
            match document.configure_symbol_mirror(target_reference, mirror) {
                Ok(summary) => applied.push(summary),
                Err(error) => {
                    self.status_message = Some(error);
                    return;
                }
            }
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
