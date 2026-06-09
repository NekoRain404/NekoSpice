use crate::placement_config::SymbolPlacementConfig;
use osl_kicad::{
    KicadAt, KicadCanvasScene, KicadEditSummary, KicadLabelKind, KicadPoint, KicadSchematic,
    KicadSchematicEdit, KicadSheetPin, KicadSimulationDirectiveKind, KicadSize, KicadSymbolDef,
    read_kicad_schematic_with_libraries, write_kicad_schematic,
};
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
        self.apply_edit(KicadSchematicEdit::DeleteItem {
            uuid: uuid.to_string(),
        })
    }

    pub(crate) fn move_item(
        &mut self,
        uuid: &str,
        delta: KicadPoint,
    ) -> Result<KicadEditSummary, String> {
        self.apply_edit(KicadSchematicEdit::MoveItem {
            uuid: uuid.to_string(),
            delta,
        })
    }

    pub(crate) fn set_symbol_property(
        &mut self,
        reference: String,
        name: String,
        value: String,
    ) -> Result<KicadEditSummary, String> {
        self.apply_edit(KicadSchematicEdit::SetSymbolProperty {
            reference,
            name,
            value,
            at: None,
        })
    }

    pub(crate) fn configure_symbol_mirror(
        &mut self,
        reference: String,
        mirror: Option<String>,
    ) -> Result<KicadEditSummary, String> {
        self.apply_edit(KicadSchematicEdit::ConfigureSymbol {
            reference,
            unit: None,
            body_style: None,
            mirror: Some(mirror),
            pin_alternates: None,
        })
    }

    pub(crate) fn place_symbol_from_definition(
        &mut self,
        definition: KicadSymbolDef,
        library_symbols: Vec<KicadSymbolDef>,
        at: KicadAt,
        config: SymbolPlacementConfig,
    ) -> Result<KicadSymbolPlacementResult, String> {
        let reference = self.next_reference_for_definition(&definition);
        let lib_id = definition.name.clone();
        let value = definition.property("Value").unwrap_or("").to_string();
        self.apply_edit(KicadSchematicEdit::PlaceSymbol {
            definition: Box::new(definition),
            library_symbols,
            reference: reference.clone(),
            value,
            at,
            unit: config.unit_option(),
            body_style: config.body_style,
            pin_alternates: config.pin_alternates,
            uuid: None,
        })
        .map(|summary| KicadSymbolPlacementResult {
            summary,
            reference,
            lib_id,
        })
    }

    pub(crate) fn add_wire(&mut self, points: Vec<KicadPoint>) -> Result<KicadEditSummary, String> {
        self.apply_edit(KicadSchematicEdit::AddWire { points, uuid: None })
    }

    pub(crate) fn add_bus(&mut self, points: Vec<KicadPoint>) -> Result<KicadEditSummary, String> {
        self.apply_edit(KicadSchematicEdit::AddBus { points, uuid: None })
    }

    pub(crate) fn add_bus_entry(
        &mut self,
        at: KicadPoint,
        size: KicadSize,
    ) -> Result<KicadEditSummary, String> {
        self.apply_edit(KicadSchematicEdit::AddBusEntry {
            at,
            size,
            uuid: None,
        })
    }

    pub(crate) fn add_junction(&mut self, at: KicadPoint) -> Result<KicadEditSummary, String> {
        self.apply_edit(KicadSchematicEdit::AddJunction { at, uuid: None })
    }

    pub(crate) fn add_no_connect(&mut self, at: KicadPoint) -> Result<KicadEditSummary, String> {
        self.apply_edit(KicadSchematicEdit::AddNoConnect { at, uuid: None })
    }

    pub(crate) fn add_label(
        &mut self,
        text: String,
        kind: KicadLabelKind,
        at: KicadAt,
    ) -> Result<KicadEditSummary, String> {
        self.apply_edit(KicadSchematicEdit::AddLabel {
            text,
            kind,
            at,
            uuid: None,
        })
    }

    pub(crate) fn add_text(
        &mut self,
        text: String,
        at: KicadAt,
    ) -> Result<KicadEditSummary, String> {
        self.apply_edit(KicadSchematicEdit::AddText {
            text,
            at,
            uuid: None,
        })
    }

    pub(crate) fn set_simulation_directive(
        &mut self,
        kind: KicadSimulationDirectiveKind,
        body: String,
        at: Option<KicadAt>,
    ) -> Result<KicadEditSummary, String> {
        self.apply_edit(KicadSchematicEdit::SetSimulationDirective {
            kind,
            body,
            at,
            uuid: None,
        })
    }

    pub(crate) fn add_sheet(
        &mut self,
        name: String,
        file: String,
        at: KicadAt,
        size: KicadSize,
        pins: Vec<KicadSheetPin>,
    ) -> Result<KicadEditSummary, String> {
        self.apply_edit(KicadSchematicEdit::AddSheet {
            name,
            file,
            at,
            size,
            pins,
            uuid: None,
        })
    }

    pub(crate) fn save(&mut self) -> Result<(), String> {
        write_kicad_schematic(&self.path, &self.schematic)
            .inspect(|_| {
                self.dirty = false;
            })
            .map_err(|error| error.to_string())
    }

    fn apply_edit(&mut self, edit: KicadSchematicEdit) -> Result<KicadEditSummary, String> {
        self.schematic
            .apply_edit(edit)
            .inspect(|_| {
                self.dirty = true;
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
    use osl_kicad::parse_kicad_symbol_library;
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
                SymbolPlacementConfig::default(),
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
    fn document_places_symbol_with_selected_pin_alternate() {
        let temp = crate::test_support::temp_schematic_copy("gui_place_alt");
        let temp_path = temp.path();

        let mut document = KicadGuiDocument::load(temp_path.to_path_buf()).unwrap();
        let definition = test_symbol_with_alternate("NekoSpice:Alt");
        let mut config = SymbolPlacementConfig::default();
        config
            .pin_alternates
            .insert("2".to_string(), "ALT2".to_string());

        let placement = document
            .place_symbol_from_definition(
                definition,
                Vec::new(),
                KicadAt {
                    x: 101.6,
                    y: 50.8,
                    rotation: 0.0,
                },
                config,
            )
            .unwrap();

        assert_eq!(placement.reference, "U1");
        let placed = document
            .schematic
            .symbols
            .iter()
            .find(|symbol| symbol.reference() == Some("U1"))
            .unwrap();
        assert_eq!(placed.pins[1].alternate.as_deref(), Some("ALT2"));
    }

    #[test]
    fn document_sets_selected_symbol_properties_for_gui_editor() {
        let temp = crate::test_support::temp_schematic_copy("gui_properties");
        let temp_path = temp.path();

        let mut document = KicadGuiDocument::load(temp_path.to_path_buf()).unwrap();
        document
            .set_symbol_property(
                "R1".to_string(),
                "Reference".to_string(),
                "RLOAD".to_string(),
            )
            .unwrap();
        document
            .set_symbol_property("RLOAD".to_string(), "Value".to_string(), "2k".to_string())
            .unwrap();
        document
            .configure_symbol_mirror("RLOAD".to_string(), Some("x y".to_string()))
            .unwrap();

        let scene = document.scene();
        let symbol = scene
            .symbols
            .iter()
            .find(|symbol| symbol.reference == "RLOAD")
            .unwrap();
        assert_eq!(symbol.value, "2k");
        assert_eq!(symbol.mirror.as_deref(), Some("x y"));
        assert!(document.is_dirty());
    }

    #[test]
    fn document_adds_basic_schematic_items_for_gui_tools() {
        let temp = crate::test_support::temp_schematic_copy("gui_tools");
        let temp_path = temp.path();

        let mut document = KicadGuiDocument::load(temp_path.to_path_buf()).unwrap();
        document
            .add_wire(vec![
                KicadPoint { x: 101.6, y: 50.8 },
                KicadPoint { x: 111.76, y: 50.8 },
            ])
            .unwrap();
        document
            .add_bus(vec![
                KicadPoint { x: 101.6, y: 38.1 },
                KicadPoint { x: 111.76, y: 38.1 },
            ])
            .unwrap();
        document
            .add_bus_entry(
                KicadPoint { x: 111.76, y: 38.1 },
                osl_kicad::KicadSize {
                    width: 2.54,
                    height: -2.54,
                },
            )
            .unwrap();
        document
            .add_label(
                "sense".to_string(),
                osl_kicad::KicadLabelKind::Global,
                KicadAt {
                    x: 111.76,
                    y: 50.8,
                    rotation: 0.0,
                },
            )
            .unwrap();
        document
            .add_label(
                "sheet_in".to_string(),
                osl_kicad::KicadLabelKind::Hierarchical,
                KicadAt {
                    x: 111.76,
                    y: 55.88,
                    rotation: 0.0,
                },
            )
            .unwrap();
        document
            .add_text(
                ".save v(out)".to_string(),
                KicadAt {
                    x: 45.72,
                    y: 35.56,
                    rotation: 0.0,
                },
            )
            .unwrap();
        document
            .set_simulation_directive(
                osl_kicad::KicadSimulationDirectiveKind::Tran,
                "2u 2m".to_string(),
                Some(KicadAt {
                    x: 45.72,
                    y: 40.64,
                    rotation: 0.0,
                }),
            )
            .unwrap();
        document
            .add_junction(KicadPoint { x: 101.6, y: 50.8 })
            .unwrap();
        document
            .add_no_connect(KicadPoint { x: 111.76, y: 50.8 })
            .unwrap();
        document
            .add_sheet(
                "gain_stage".to_string(),
                "gain_stage.kicad_sch".to_string(),
                KicadAt {
                    x: 120.0,
                    y: 40.0,
                    rotation: 0.0,
                },
                osl_kicad::KicadSize {
                    width: 25.4,
                    height: 12.7,
                },
                vec![
                    osl_kicad::KicadSheetPin {
                        name: "in".to_string(),
                        pin_type: "input".to_string(),
                        at: Some(KicadAt {
                            x: 120.0,
                            y: 46.35,
                            rotation: 180.0,
                        }),
                        uuid: None,
                        effects: None,
                    },
                    osl_kicad::KicadSheetPin {
                        name: "out".to_string(),
                        pin_type: "output".to_string(),
                        at: Some(KicadAt {
                            x: 145.4,
                            y: 46.35,
                            rotation: 0.0,
                        }),
                        uuid: None,
                        effects: None,
                    },
                ],
            )
            .unwrap();

        let scene = document.scene();
        assert!(document.is_dirty());
        assert_eq!(scene.wires.len(), 4);
        assert_eq!(scene.buses.len(), 1);
        assert_eq!(scene.bus_entries.len(), 1);
        assert_eq!(scene.sheets.len(), 1);
        assert_eq!(scene.sheets[0].name, "gain_stage");
        assert_eq!(scene.sheets[0].pins.len(), 2);
        assert!(
            scene
                .labels
                .iter()
                .any(|label| label.text == "sense"
                    && label.kind == osl_kicad::KicadLabelKind::Global)
        );
        assert!(scene.labels.iter().any(|label| label.text == "sheet_in"
            && label.kind == osl_kicad::KicadLabelKind::Hierarchical));
        assert!(
            scene
                .text_items
                .iter()
                .any(|text| text.text == ".save v(out)")
        );
        assert!(
            scene
                .text_items
                .iter()
                .any(|text| text.text == ".tran 2u 2m")
        );
        assert!(scene.junctions.iter().any(|junction| {
            (junction.at.x - 101.6).abs() < 1e-6 && (junction.at.y - 50.8).abs() < 1e-6
        }));
        assert!(scene.no_connects.iter().any(|marker| {
            (marker.at.x - 111.76).abs() < 1e-6 && (marker.at.y - 50.8).abs() < 1e-6
        }));
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

    fn test_symbol_with_alternate(name: &str) -> KicadSymbolDef {
        parse_kicad_symbol_library(
            &format!(
                r#"(kicad_symbol_lib
  (version 20230121)
  (symbol "{name}"
    (property "Reference" "U" (at 0 0 0))
    (property "Value" "Alt" (at 0 -2.54 0))
    (symbol "Alt_0_1"
      (pin passive line (at -2.54 0 0) (length 2.54) (name "~") (number "1"))
      (pin passive line (at 2.54 0 180) (length 2.54) (name "~") (number "2") (alternate "ALT2" output line))
    )
  )
)"#
            ),
            "gui_alt_symbol.kicad_sym",
        )
        .unwrap()
        .symbol(name)
        .unwrap()
        .clone()
    }
}
