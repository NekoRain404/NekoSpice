use osl_kicad::{
    KicadAt, KicadCanvasScene, KicadEditSummary, KicadPoint, KicadSchematic, KicadSchematicEdit,
    KicadSymbolDef, read_kicad_schematic_with_libraries, write_kicad_schematic,
};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct KicadSymbolPlacementResult {
    pub(crate) summary: KicadEditSummary,
    pub(crate) reference: String,
    pub(crate) lib_id: String,
}

#[derive(Debug)]
pub(crate) struct KicadGuiDocument {
    path: PathBuf,
    schematic: KicadSchematic,
    dirty: bool,
}

impl KicadGuiDocument {
    pub(crate) fn load(path: PathBuf) -> Result<Self, String> {
        read_kicad_schematic_with_libraries(&path)
            .map(|schematic| Self {
                path,
                schematic,
                dirty: false,
            })
            .map_err(|error| error.to_string())
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub(crate) fn scene(&self) -> KicadCanvasScene {
        self.schematic.canvas_scene()
    }

    pub(crate) fn delete_item(&mut self, uuid: &str) -> Result<KicadEditSummary, String> {
        self.schematic
            .apply_edit(KicadSchematicEdit::DeleteItem {
                uuid: uuid.to_string(),
            })
            .inspect(|_| {
                self.dirty = true;
            })
            .map_err(|error| error.to_string())
    }

    pub(crate) fn move_item(
        &mut self,
        uuid: &str,
        delta: KicadPoint,
    ) -> Result<KicadEditSummary, String> {
        self.schematic
            .apply_edit(KicadSchematicEdit::MoveItem {
                uuid: uuid.to_string(),
                delta,
            })
            .inspect(|_| {
                self.dirty = true;
            })
            .map_err(|error| error.to_string())
    }

    pub(crate) fn place_symbol_from_definition(
        &mut self,
        definition: KicadSymbolDef,
        library_symbols: Vec<KicadSymbolDef>,
        at: KicadAt,
    ) -> Result<KicadSymbolPlacementResult, String> {
        let reference = self.next_reference_for_definition(&definition);
        let lib_id = definition.name.clone();
        let value = definition.property("Value").unwrap_or("").to_string();
        self.schematic
            .apply_edit(KicadSchematicEdit::PlaceSymbol {
                definition: Box::new(definition),
                library_symbols,
                reference: reference.clone(),
                value,
                at,
                unit: Some(1),
                body_style: None,
                pin_alternates: BTreeMap::new(),
                uuid: None,
            })
            .inspect(|_| {
                self.dirty = true;
            })
            .map(|summary| KicadSymbolPlacementResult {
                summary,
                reference,
                lib_id,
            })
            .map_err(|error| error.to_string())
    }

    pub(crate) fn save(&mut self) -> Result<(), String> {
        write_kicad_schematic(&self.path, &self.schematic)
            .inspect(|_| {
                self.dirty = false;
            })
            .map_err(|error| error.to_string())
    }

    fn next_reference_for_definition(&self, definition: &KicadSymbolDef) -> String {
        let prefix = reference_prefix(definition);

        let mut next = 1;
        for symbol in &self.schematic.symbols {
            let Some(reference) = symbol.reference() else {
                continue;
            };
            let Some(suffix) = reference.strip_prefix(prefix) else {
                continue;
            };
            if let Ok(number) = suffix.parse::<u32>() {
                next = next.max(number + 1);
            }
        }

        format!("{prefix}{next}")
    }
}

fn reference_prefix(definition: &KicadSymbolDef) -> &str {
    let prefix = definition
        .property("Reference")
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| definition.local_name())
        .trim_end_matches('?');
    if prefix.is_empty() {
        definition.local_name()
    } else {
        prefix
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn document_deletes_selected_uuid_and_saves_schematic() {
        let temp = crate::test_support::temp_schematic_copy("gui_delete");
        let temp_path = temp.path();

        let mut document = KicadGuiDocument::load(temp_path.to_path_buf()).unwrap();
        assert!(!document.is_dirty());
        assert_eq!(document.scene().wires.len(), 3);

        let summary = document
            .delete_item("22222222-2222-2222-2222-222222222222")
            .unwrap();
        assert_eq!(summary.operation, "delete-wire");
        assert!(document.is_dirty());
        assert_eq!(document.scene().wires.len(), 2);

        document.save().unwrap();
        assert!(!document.is_dirty());
        let saved = fs::read_to_string(temp_path).unwrap();
        assert!(!saved.contains("22222222-2222-2222-2222-222222222222"));
    }

    #[test]
    fn document_moves_selected_uuid_and_keeps_canvas_hit_addressable() {
        let temp = crate::test_support::temp_schematic_copy("gui_move");
        let temp_path = temp.path();

        let mut document = KicadGuiDocument::load(temp_path.to_path_buf()).unwrap();
        let original_hit = document
            .scene()
            .item_hit_by_uuid("22222222-2222-2222-2222-222222222222")
            .unwrap();

        let summary = document
            .move_item(
                "22222222-2222-2222-2222-222222222222",
                KicadPoint { x: 2.54, y: 0.0 },
            )
            .unwrap();
        assert_eq!(summary.operation, "move-wire");
        assert!(document.is_dirty());

        let moved_hit = document
            .scene()
            .item_hit_by_uuid("22222222-2222-2222-2222-222222222222")
            .unwrap();
        assert!((moved_hit.bounds.min.x - original_hit.bounds.min.x - 2.54).abs() < 1e-6);
        assert_eq!(moved_hit.kind, "wire");

        document.save().unwrap();
        assert!(!document.is_dirty());
        let reloaded_scene = read_kicad_schematic_with_libraries(temp_path)
            .unwrap()
            .canvas_scene();
        let saved_hit = reloaded_scene
            .item_hit_by_uuid("22222222-2222-2222-2222-222222222222")
            .unwrap();
        assert_eq!(saved_hit.kind, "wire");
        assert!((saved_hit.bounds.min.x - original_hit.bounds.min.x - 2.54).abs() < 1e-6);
        assert!((saved_hit.bounds.min.y - original_hit.bounds.min.y).abs() < 1e-6);
    }

    #[test]
    fn document_places_library_symbol_with_next_reference() {
        let temp = crate::test_support::temp_schematic_copy("gui_place");
        let temp_path = temp.path();

        let mut document = KicadGuiDocument::load(temp_path.to_path_buf()).unwrap();
        let definition = document
            .schematic
            .library_symbols
            .iter()
            .find(|symbol| symbol.name == "NekoSpice:R")
            .cloned()
            .unwrap();

        let placement = document
            .place_symbol_from_definition(
                definition,
                Vec::new(),
                KicadAt {
                    x: 101.6,
                    y: 50.8,
                    rotation: 0.0,
                },
            )
            .unwrap();

        assert_eq!(placement.summary.operation, "place-symbol");
        assert_eq!(placement.summary.target, "R2 NekoSpice:R");
        assert_eq!(placement.reference, "R2");
        assert_eq!(placement.lib_id, "NekoSpice:R");
        assert!(document.is_dirty());
        assert!(
            document
                .scene()
                .symbols
                .iter()
                .any(|symbol| symbol.reference == "R2")
        );
    }

    #[test]
    fn symbol_reference_prefix_ignores_kicad_placeholder_suffix() {
        let mut definition = test_symbol_definition("NekoSpice:R");
        definition.properties.push(osl_kicad::KicadProperty {
            name: "Reference".to_string(),
            value: "R?".to_string(),
            id: None,
            at: None,
            hide: None,
            show_name: None,
            do_not_autoplace: None,
            effects: None,
        });

        assert_eq!(reference_prefix(&definition), "R");
    }

    fn test_symbol_definition(name: &str) -> KicadSymbolDef {
        KicadSymbolDef {
            name: name.to_string(),
            extends: None,
            power: None,
            body_styles: None,
            exclude_from_sim: None,
            in_bom: None,
            on_board: None,
            in_pos_files: None,
            duplicate_pin_numbers_are_jumpers: None,
            jumper_pin_groups: Vec::new(),
            embedded_fonts: None,
            pin_names: None,
            pin_numbers: None,
            unit_names: Default::default(),
            properties: Vec::new(),
            graphics: Vec::new(),
            pins: Vec::new(),
        }
    }
}
