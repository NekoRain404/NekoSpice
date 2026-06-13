//! Domain-focused tests for nsp-schema.

use super::assert_close;
use crate::{
    NspAt, NspLabelKind, NspPoint, NspSchematicEdit, NspSheetPin, NspSize, parse_schematic,
    parse_symbol_library, read_schematic, read_symbol_library,
};
use std::collections::BTreeMap;
use std::path::Path;

#[test]
fn roundtrips_schema_schematic_fixture_through_writer() {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap();
    let schematic =
        read_schematic(&workspace_root.join("examples/schema_schematic/rc.nsp_sch")).unwrap();

    let exported = schematic.to_schematic_sexpr();
    assert!(exported.contains("(nsp_sch"));
    assert!(exported.contains("(lib_symbols"));
    assert!(exported.contains("(lib_id \"NekoSpice:R\")"));
    let reparsed = parse_schematic(&exported, "roundtrip.nsp_sch").unwrap();

    assert_eq!(reparsed.symbols.len(), 3);
    assert_eq!(reparsed.paper.as_deref(), Some("A4"));
    assert_eq!(reparsed.library_symbols.len(), 3);
    assert_eq!(reparsed.wires.len(), 3);
    assert_eq!(
        reparsed.wires[0].uuid.as_deref(),
        Some("22222222-2222-2222-2222-222222222222")
    );
    assert_eq!(reparsed.labels.len(), 3);
    assert_eq!(
        reparsed.labels[1].uuid.as_deref(),
        Some("66666666-6666-6666-6666-666666666666")
    );
    assert_eq!(reparsed.spice_directives()[0].text, ".tran 1u 1m");
    assert_eq!(
        reparsed.spice_directives()[0].uuid.as_deref(),
        Some("77777777-7777-7777-7777-777777777777")
    );
    assert_eq!(reparsed.symbols[0].pins[0].number.as_deref(), Some("1"));
    assert_eq!(
        reparsed.symbols[0].pins[0].uuid.as_deref(),
        Some("99999999-9999-9999-9999-999999999991")
    );
    assert_eq!(
        reparsed
            .library_symbols
            .iter()
            .map(|symbol| symbol.graphics.len())
            .sum::<usize>(),
        6
    );
    assert!(reparsed.canvas_scene().bounds.is_some());
    let netlist = reparsed.to_spice_netlist().unwrap();
    assert!(netlist.contains("R1 in out 1k"));
    assert!(netlist.contains("C1 out 0 100n"));
}

#[test]
fn edits_schema_schematic_in_rust_ir_and_roundtrips() {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap();
    let mut schematic =
        read_schematic(&workspace_root.join("examples/schema_schematic/rc.nsp_sch")).unwrap();

    schematic
        .apply_edit(NspSchematicEdit::MoveSymbol {
            reference: "R1".to_string(),
            to: NspPoint { x: 73.66, y: 50.8 },
            rotation: Some(0.0),
        })
        .unwrap();
    schematic
        .apply_edit(NspSchematicEdit::SetSymbolProperty {
            reference: "R1".to_string(),
            name: "Value".to_string(),
            value: "2k".to_string(),
            at: None,
        })
        .unwrap();
    schematic
        .apply_edit(NspSchematicEdit::AddWire {
            points: vec![
                NspPoint { x: 73.66, y: 45.72 },
                NspPoint { x: 88.9, y: 45.72 },
            ],
            uuid: Some("eeeeeeee-eeee-eeee-eeee-eeeeeeeeeeee".to_string()),
        })
        .unwrap();
    schematic
        .apply_edit(NspSchematicEdit::AddBus {
            points: vec![
                NspPoint { x: 88.9, y: 38.1 },
                NspPoint { x: 101.6, y: 38.1 },
            ],
            uuid: Some("33333333-aaaa-bbbb-cccc-333333333333".to_string()),
        })
        .unwrap();
    schematic
        .apply_edit(NspSchematicEdit::AddBusEntry {
            at: NspPoint { x: 101.6, y: 38.1 },
            size: NspSize {
                width: 2.54,
                height: -2.54,
            },
            uuid: Some("44444444-aaaa-bbbb-cccc-444444444444".to_string()),
        })
        .unwrap();
    schematic
        .apply_edit(NspSchematicEdit::AddJunction {
            at: NspPoint { x: 88.9, y: 45.72 },
            uuid: Some("11111111-aaaa-bbbb-cccc-111111111111".to_string()),
        })
        .unwrap();
    schematic
        .apply_edit(NspSchematicEdit::AddNoConnect {
            at: NspPoint { x: 101.6, y: 45.72 },
            uuid: Some("22222222-aaaa-bbbb-cccc-222222222222".to_string()),
        })
        .unwrap();
    schematic
        .apply_edit(NspSchematicEdit::AddLabel {
            text: "sense".to_string(),
            kind: NspLabelKind::Global,
            at: NspAt {
                x: 88.9,
                y: 45.72,
                rotation: 0.0,
            },
            uuid: Some("ffffffff-ffff-ffff-ffff-ffffffffffff".to_string()),
        })
        .unwrap();
    schematic
        .apply_edit(NspSchematicEdit::AddText {
            text: ".save v(sense)".to_string(),
            at: NspAt {
                x: 45.72,
                y: 35.56,
                rotation: 0.0,
            },
            uuid: Some("abababab-abab-abab-abab-abababababab".to_string()),
        })
        .unwrap();
    schematic
        .apply_edit(NspSchematicEdit::AddSheet {
            name: "gain_stage".to_string(),
            file: "gain_stage.nsp_sch".to_string(),
            at: NspAt {
                x: 101.6,
                y: 43.18,
                rotation: 0.0,
            },
            size: NspSize {
                width: 25.4,
                height: 12.7,
            },
            pins: vec![
                NspSheetPin {
                    name: "in".to_string(),
                    pin_type: "input".to_string(),
                    at: Some(NspAt {
                        x: 101.6,
                        y: 48.26,
                        rotation: 180.0,
                    }),
                    uuid: None,
                    effects: None,
                },
                NspSheetPin {
                    name: "out".to_string(),
                    pin_type: "output".to_string(),
                    at: Some(NspAt {
                        x: 127.0,
                        y: 48.26,
                        rotation: 0.0,
                    }),
                    uuid: None,
                    effects: None,
                },
            ],
            uuid: Some("cdcdcdcd-cdcd-cdcd-cdcd-cdcdcdcdcdcd".to_string()),
        })
        .unwrap();

    let resistor = schematic
        .symbols
        .iter()
        .find(|symbol| symbol.reference() == Some("R1"))
        .unwrap();
    assert_close(resistor.at.unwrap().x, 73.66);
    assert_close(
        resistor
            .properties
            .iter()
            .find(|property| property.name == "Reference")
            .unwrap()
            .at
            .unwrap()
            .x,
        73.66,
    );
    assert_eq!(resistor.value(), Some("2k"));
    assert_eq!(schematic.wires.len(), 4);
    assert_eq!(schematic.buses.len(), 1);
    assert_eq!(schematic.bus_entries.len(), 1);
    assert_eq!(schematic.junctions.len(), 1);
    assert_eq!(schematic.no_connects.len(), 1);
    assert_eq!(schematic.sheets.len(), 1);
    assert_eq!(schematic.sheets[0].sheet_name(), Some("gain_stage"));
    assert_eq!(schematic.sheets[0].pins.len(), 2);
    assert!(schematic.labels.iter().any(|label| {
        label.text == "sense"
            && label.kind == NspLabelKind::Global
            && label.uuid.as_deref() == Some("ffffffff-ffff-ffff-ffff-ffffffffffff")
    }));
    assert!(
        schematic
            .spice_directives()
            .iter()
            .any(|directive| directive.text == ".save v(sense)")
    );

    let exported = schematic.to_schematic_sexpr();
    assert!(exported.contains("(bus"));
    assert!(exported.contains("(uuid \"33333333-aaaa-bbbb-cccc-333333333333\")"));
    assert!(exported.contains("(bus_entry"));
    assert!(exported.contains("(uuid \"44444444-aaaa-bbbb-cccc-444444444444\")"));
    assert!(exported.contains("(junction"));
    assert!(exported.contains("(uuid \"11111111-aaaa-bbbb-cccc-111111111111\")"));
    assert!(exported.contains("(no_connect"));
    assert!(exported.contains("(uuid \"22222222-aaaa-bbbb-cccc-222222222222\")"));
    assert!(exported.contains("(global_label \"sense\""));
    assert!(exported.contains("(sheet"));
    assert!(exported.contains("(property \"Sheetname\" \"gain_stage\""));
    assert!(exported.contains("(pin \"in\" input"));
    assert!(exported.contains("(text \".save v(sense)\""));
    let reparsed = parse_schematic(&exported, "edited.nsp_sch").unwrap();
    assert_eq!(reparsed.wires.len(), 4);
    assert_eq!(reparsed.buses.len(), 1);
    assert_eq!(
        reparsed.buses[0].uuid.as_deref(),
        Some("33333333-aaaa-bbbb-cccc-333333333333")
    );
    assert_eq!(reparsed.bus_entries.len(), 1);
    assert_eq!(
        reparsed.bus_entries[0].uuid.as_deref(),
        Some("44444444-aaaa-bbbb-cccc-444444444444")
    );
    assert_eq!(reparsed.junctions.len(), 1);
    assert_eq!(
        reparsed.junctions[0].uuid.as_deref(),
        Some("11111111-aaaa-bbbb-cccc-111111111111")
    );
    assert_eq!(reparsed.no_connects.len(), 1);
    assert_eq!(
        reparsed.no_connects[0].uuid.as_deref(),
        Some("22222222-aaaa-bbbb-cccc-222222222222")
    );
    assert_eq!(reparsed.sheets.len(), 1);
    assert_eq!(reparsed.sheets[0].pins.len(), 2);
    assert_eq!(reparsed.canvas_scene().buses.len(), 1);
    assert_eq!(reparsed.canvas_scene().bus_entries.len(), 1);
    assert_eq!(reparsed.canvas_scene().junctions.len(), 1);
    assert_eq!(reparsed.canvas_scene().no_connects.len(), 1);
    assert_eq!(
        reparsed
            .symbols
            .iter()
            .find(|symbol| symbol.reference() == Some("R1"))
            .unwrap()
            .value(),
        Some("2k")
    );
    assert!(
        reparsed
            .spice_directives()
            .iter()
            .any(|directive| directive.text == ".save v(sense)")
    );
}

#[test]
fn deletes_schema_schematic_items_by_uuid_and_roundtrips() {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap();
    let mut schematic =
        read_schematic(&workspace_root.join("examples/schema_schematic/rc.nsp_sch")).unwrap();

    schematic
        .apply_edit(NspSchematicEdit::DeleteItem {
            uuid: "22222222-2222-2222-2222-222222222222".to_string(),
        })
        .unwrap();
    schematic
        .apply_edit(NspSchematicEdit::DeleteItem {
            uuid: "66666666-6666-6666-6666-666666666666".to_string(),
        })
        .unwrap();
    schematic
        .apply_edit(NspSchematicEdit::DeleteItem {
            uuid: "77777777-7777-7777-7777-777777777777".to_string(),
        })
        .unwrap();

    assert_eq!(schematic.wires.len(), 2);
    assert_eq!(schematic.labels.len(), 2);
    assert!(schematic.spice_directives().is_empty());

    let exported = schematic.to_schematic_sexpr();
    assert!(!exported.contains("22222222-2222-2222-2222-222222222222"));
    assert!(!exported.contains("66666666-6666-6666-6666-666666666666"));
    assert!(!exported.contains("77777777-7777-7777-7777-777777777777"));
    let reparsed = parse_schematic(&exported, "deleted_items.nsp_sch").unwrap();
    assert_eq!(reparsed.wires.len(), 2);
    assert_eq!(reparsed.labels.len(), 2);
    assert!(reparsed.canvas_scene().text_items.is_empty());

    let error = schematic
        .apply_edit(NspSchematicEdit::DeleteItem {
            uuid: "00000000-0000-4000-8000-000000000000".to_string(),
        })
        .unwrap_err();
    assert!(error.to_string().contains("was not found"));
}

#[test]
fn edits_schema_table_cells_by_uuid_and_roundtrips() {
    let mut schematic = parse_schematic(
        r#"(nsp_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols)
  (table
    (column_count 2)
    (uuid "67676767-6767-4767-8767-676767676767")
    (column_widths 20 20)
    (row_heights 5)
    (cells
      (table_cell "Move me"
        (at 10 10 45)
        (size 20 5)
        (uuid "68686868-6868-4868-8868-686868686868")
      )
      (table_cell "Delete me"
        (at 30 10 0)
        (size 20 5)
        (uuid "69696969-6969-4969-8969-696969696969")
      )
    )
  )
)"#,
        "table_cell_edits.nsp_sch",
    )
    .unwrap();

    let move_summary = schematic
        .apply_edit(NspSchematicEdit::MoveItem {
            uuid: "68686868-6868-4868-8868-686868686868".to_string(),
            delta: NspPoint { x: 2.54, y: -1.27 },
        })
        .unwrap();
    assert_eq!(move_summary.operation, "move-table-cell");
    assert_close(schematic.tables[0].cells[0].at.unwrap().x, 12.54);
    assert_close(schematic.tables[0].cells[0].at.unwrap().y, 8.73);
    assert_close(schematic.tables[0].cells[1].at.unwrap().x, 30.0);

    let delete_summary = schematic
        .apply_edit(NspSchematicEdit::DeleteItem {
            uuid: "69696969-6969-4969-8969-696969696969".to_string(),
        })
        .unwrap();
    assert_eq!(delete_summary.operation, "delete-table-cell");
    assert_eq!(schematic.tables.len(), 1);
    assert_eq!(schematic.tables[0].cells.len(), 1);
    assert_eq!(schematic.tables[0].cells[0].text, "Move me");

    let exported = schematic.to_schematic_sexpr();
    assert!(exported.contains("(table"));
    assert!(exported.contains("(table_cell \"Move me\""));
    assert!(exported.contains("(at 12.54 8.73 45)"));
    assert!(!exported.contains("Delete me"));
    assert!(!exported.contains("69696969-6969-4969-8969-696969696969"));
    let reparsed = parse_schematic(&exported, "table_cell_edits_roundtrip.nsp_sch").unwrap();
    assert_eq!(reparsed.tables.len(), 1);
    assert_eq!(reparsed.tables[0].cells.len(), 1);
    assert_eq!(
        reparsed.tables[0].cells[0].uuid.as_deref(),
        Some("68686868-6868-4868-8868-686868686868")
    );
    assert_close(reparsed.tables[0].cells[0].at.unwrap().x, 12.54);
    assert_eq!(reparsed.canvas_scene().tables[0].cells.len(), 1);
}

#[test]
fn edits_schema_sheet_pins_by_uuid_and_roundtrips() {
    let mut schematic = parse_schematic(
        r#"(nsp_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols)
  (sheet
    (at 50 40)
    (size 30 20)
    (property "Sheetname" "gain_stage" (id 0) (at 50 38 0))
    (property "Sheetfile" "gain_stage.nsp_sch" (id 1) (at 50 62 0))
    (pin "in" input (at 50 45 180) (uuid "11111111-1111-4111-8111-111111111111"))
    (pin "out" output (at 80 45 0) (uuid "22222222-2222-4222-8222-222222222222"))
    (uuid "33333333-3333-4333-8333-333333333333")
  )
)"#,
        "sheet_pin_edits.nsp_sch",
    )
    .unwrap();

    let move_summary = schematic
        .apply_edit(NspSchematicEdit::MoveItem {
            uuid: "11111111-1111-4111-8111-111111111111".to_string(),
            delta: NspPoint { x: 2.54, y: -1.27 },
        })
        .unwrap();
    assert_eq!(move_summary.operation, "move-sheet-pin");
    assert_close(schematic.sheets[0].pins[0].at.unwrap().x, 52.54);
    assert_close(schematic.sheets[0].pins[0].at.unwrap().y, 43.73);
    assert_close(schematic.sheets[0].at.unwrap().x, 50.0);

    let delete_summary = schematic
        .apply_edit(NspSchematicEdit::DeleteItem {
            uuid: "22222222-2222-4222-8222-222222222222".to_string(),
        })
        .unwrap();
    assert_eq!(delete_summary.operation, "delete-sheet-pin");
    assert_eq!(schematic.sheets.len(), 1);
    assert_eq!(schematic.sheets[0].pins.len(), 1);
    assert_eq!(schematic.sheets[0].pins[0].name, "in");

    let exported = schematic.to_schematic_sexpr();
    assert!(exported.contains("(sheet"));
    assert!(exported.contains("(pin \"in\" input (at 52.54 43.73 180)"));
    assert!(!exported.contains("pin \"out\""));
    assert!(!exported.contains("22222222-2222-4222-8222-222222222222"));
    let reparsed = parse_schematic(&exported, "sheet_pin_edits_roundtrip.nsp_sch").unwrap();
    assert_eq!(reparsed.sheets.len(), 1);
    assert_eq!(reparsed.sheets[0].pins.len(), 1);
    assert_eq!(
        reparsed.sheets[0].pins[0].uuid.as_deref(),
        Some("11111111-1111-4111-8111-111111111111")
    );
    assert_close(reparsed.sheets[0].pins[0].at.unwrap().x, 52.54);
    assert_eq!(reparsed.canvas_scene().sheets[0].pins.len(), 1);
}

#[test]
fn moves_schema_schematic_items_by_uuid_and_roundtrips() {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap();
    let mut schematic =
        read_schematic(&workspace_root.join("examples/schema_schematic/rc.nsp_sch")).unwrap();

    schematic
        .apply_edit(NspSchematicEdit::AddSheet {
            name: "gain_stage".to_string(),
            file: "gain_stage.nsp_sch".to_string(),
            at: NspAt {
                x: 101.6,
                y: 43.18,
                rotation: 0.0,
            },
            size: NspSize {
                width: 25.4,
                height: 12.7,
            },
            pins: vec![NspSheetPin {
                name: "in".to_string(),
                pin_type: "input".to_string(),
                at: Some(NspAt {
                    x: 101.6,
                    y: 48.26,
                    rotation: 180.0,
                }),
                uuid: None,
                effects: None,
            }],
            uuid: Some("cdcdcdcd-cdcd-cdcd-cdcd-cdcdcdcdcdcd".to_string()),
        })
        .unwrap();

    schematic
        .apply_edit(NspSchematicEdit::MoveItem {
            uuid: "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa".to_string(),
            delta: NspPoint { x: 2.54, y: -1.27 },
        })
        .unwrap();
    schematic
        .apply_edit(NspSchematicEdit::MoveItem {
            uuid: "22222222-2222-2222-2222-222222222222".to_string(),
            delta: NspPoint { x: 1.27, y: 2.54 },
        })
        .unwrap();
    schematic
        .apply_edit(NspSchematicEdit::MoveItem {
            uuid: "66666666-6666-6666-6666-666666666666".to_string(),
            delta: NspPoint { x: -2.54, y: 1.27 },
        })
        .unwrap();
    schematic
        .apply_edit(NspSchematicEdit::MoveItem {
            uuid: "cdcdcdcd-cdcd-cdcd-cdcd-cdcdcdcdcdcd".to_string(),
            delta: NspPoint { x: 5.08, y: 2.54 },
        })
        .unwrap();

    let resistor = schematic
        .symbols
        .iter()
        .find(|symbol| symbol.uuid.as_deref() == Some("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa"))
        .unwrap();
    assert_close(resistor.at.unwrap().x, 72.39);
    assert_close(resistor.at.unwrap().y, 49.53);
    assert_close(
        resistor
            .properties
            .iter()
            .find(|property| property.name == "Reference")
            .unwrap()
            .at
            .unwrap()
            .x,
        72.39,
    );
    assert_close(schematic.wires[0].points[0].x, 52.07);
    assert_close(schematic.wires[0].points[0].y, 53.34);
    assert_close(
        schematic
            .labels
            .iter()
            .find(|label| label.uuid.as_deref() == Some("66666666-6666-6666-6666-666666666666"))
            .unwrap()
            .at
            .unwrap()
            .x,
        86.36,
    );
    assert_close(schematic.sheets[0].at.unwrap().x, 106.68);
    assert_close(schematic.sheets[0].pins[0].at.unwrap().x, 106.68);

    let exported = schematic.to_schematic_sexpr();
    assert!(exported.contains("(at 72.39 49.53 0)"));
    assert!(exported.contains("(xy 52.07 53.34)"));
    assert!(exported.contains("(at 86.36 52.07 0)"));
    assert!(exported.contains("(at 106.68 45.72)"));
    let reparsed = parse_schematic(&exported, "moved_items.nsp_sch").unwrap();
    let scene = reparsed.canvas_scene();
    assert_close(scene.symbols[1].at.x, 72.39);
    assert_close(scene.wires[0].points[0].x, 52.07);
    assert_close(scene.labels[1].at.unwrap().x, 86.36);
    assert_close(scene.sheets[0].at.unwrap().x, 106.68);
    assert_close(scene.sheets[0].pins[0].at.unwrap().x, 106.68);

    let error = schematic
        .apply_edit(NspSchematicEdit::MoveItem {
            uuid: "00000000-0000-4000-8000-000000000000".to_string(),
            delta: NspPoint { x: 1.0, y: 1.0 },
        })
        .unwrap_err();
    assert!(error.to_string().contains("was not found"));
}

#[test]
fn places_symbol_from_schema_library_into_schematic_ir() {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap();
    let mut schematic =
        read_schematic(&workspace_root.join("examples/schema_schematic/rc.nsp_sch")).unwrap();
    let library =
        read_symbol_library(&workspace_root.join("examples/schema_schematic/neko_spice.nsp_sym"))
            .unwrap();
    let capacitor = library.symbol("NekoSpice:C").unwrap().clone();

    schematic
        .apply_edit(NspSchematicEdit::PlaceSymbol {
            definition: Box::new(capacitor),
            library_symbols: library.symbols.clone(),
            reference: "C2".to_string(),
            value: "47n".to_string(),
            at: NspAt {
                x: 101.6,
                y: 53.34,
                rotation: 0.0,
            },
            unit: Some(1),
            body_style: None,
            pin_alternates: BTreeMap::new(),
            uuid: Some("eeeeeeee-eeee-eeee-eeee-eeeeeeeeeeee".to_string()),
        })
        .unwrap();

    let placed = schematic
        .symbols
        .iter()
        .find(|symbol| symbol.reference() == Some("C2"))
        .unwrap();
    assert_eq!(placed.lib_id, "NekoSpice:C");
    assert_eq!(placed.value(), Some("47n"));
    assert_eq!(placed.pins.len(), 2);
    assert!(placed.pins.iter().all(|pin| pin.uuid.is_some()));
    assert!(
        schematic
            .library_symbols
            .iter()
            .any(|symbol| symbol.name == "NekoSpice:C")
    );

    let exported = schematic.to_schematic_sexpr();
    assert!(exported.contains("(property \"Reference\" \"C2\""));
    assert!(exported.contains("(property \"Value\" \"47n\""));
    let reparsed = parse_schematic(&exported, "placed.nsp_sch").unwrap();
    assert!(
        reparsed
            .canvas_scene()
            .symbols
            .iter()
            .any(|symbol| symbol.reference == "C2" && symbol.pins.len() == 2)
    );
}

#[test]
fn places_derived_symbol_with_parent_library_context() {
    let mut schematic = parse_schematic(
        r#"(nsp_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols)
)"#,
        "empty_derived_placement.nsp_sch",
    )
    .unwrap();
    let library = parse_symbol_library(
        r#"(nsp_symbol_lib
  (version 20230121)
  (symbol "NekoSpice:ParentR"
    (property "Reference" "R" (at 0 0 0))
    (property "Value" "1k" (at 0 -2.54 0))
    (symbol "ParentR_0_1"
      (rectangle (start -1 -1) (end 1 1) (stroke (width 0.127) (type default)) (fill (type none)))
      (pin passive line (at -2.54 0 0) (length 2.54) (name "~") (number "1"))
      (pin passive line (at 2.54 0 180) (length 2.54) (name "~") (number "2"))
    )
  )
  (symbol "NekoSpice:DerivedR"
    (extends "NekoSpice:ParentR")
    (property "Reference" "R" (at 0 0 0))
    (property "Value" "2.2k" (at 0 -2.54 0))
  )
)"#,
        "derived_placement.nsp_sym",
    )
    .unwrap();

    schematic
        .apply_edit(NspSchematicEdit::PlaceSymbol {
            definition: Box::new(library.symbol("NekoSpice:DerivedR").unwrap().clone()),
            library_symbols: library.symbols.clone(),
            reference: "R1".to_string(),
            value: "2.2k".to_string(),
            at: NspAt {
                x: 10.0,
                y: 10.0,
                rotation: 0.0,
            },
            unit: Some(1),
            body_style: None,
            pin_alternates: BTreeMap::new(),
            uuid: Some("bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb".to_string()),
        })
        .unwrap();

    assert!(
        schematic
            .library_symbols
            .iter()
            .any(|symbol| symbol.name == "NekoSpice:ParentR")
    );
    assert!(
        schematic
            .library_symbols
            .iter()
            .any(|symbol| symbol.name == "NekoSpice:DerivedR")
    );
    let placed = schematic
        .symbols
        .iter()
        .find(|symbol| symbol.reference() == Some("R1"))
        .unwrap();
    assert_eq!(placed.pins.len(), 2);
    assert_eq!(schematic.canvas_scene().symbols[0].graphics.len(), 1);

    let exported = schematic.to_schematic_sexpr();
    assert!(exported.contains("(symbol \"NekoSpice:ParentR\""));
    assert!(exported.contains("(symbol \"NekoSpice:DerivedR\""));
    assert!(exported.contains("(extends \"NekoSpice:ParentR\")"));
    assert!(!exported.contains("(symbol \"DerivedR_0_1\""));
}

#[test]
fn places_symbol_when_embedded_library_has_explicit_default_property_effects() {
    let mut schematic = parse_schematic(
            r#"(nsp_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols
    (symbol "NekoSpice:R"
      (property "Reference" "R" (at 0 0 0)
        (effects (font (size 1.27 1.27)))
      )
      (property "Value" "1k" (at 0 -2.54 0)
        (effects (font (size 1.27 1.27)))
      )
      (symbol "R_0_1"
        (rectangle (start -1.27 -1.27) (end 1.27 1.27) (stroke (width 0.254) (type default)) (fill (type none)))
        (pin passive line (at -2.54 0 0) (length 2.54) (name "~" (effects (font (size 1.27 1.27)))) (number "1" (effects (font (size 1.27 1.27)))))
        (pin passive line (at 2.54 0 180) (length 2.54) (name "~" (effects (font (size 1.27 1.27)))) (number "2" (effects (font (size 1.27 1.27)))))
      )
    )
  )
)"#,
            "explicit_default_effects.nsp_sch",
        )
        .unwrap();
    let library = parse_symbol_library(
            r#"(nsp_symbol_lib
  (version 20230121)
  (symbol "NekoSpice:R"
    (property "Reference" "R" (at 0 0 0))
    (property "Value" "1k" (at 0 -2.54 0))
    (symbol "R_0_1"
      (rectangle (start -1.27 -1.27) (end 1.27 1.27) (stroke (width 0.254) (type default)) (fill (type none)))
      (pin passive line (at -2.54 0 0) (length 2.54) (name "~" (effects (font (size 1.27 1.27)))) (number "1" (effects (font (size 1.27 1.27)))))
      (pin passive line (at 2.54 0 180) (length 2.54) (name "~" (effects (font (size 1.27 1.27)))) (number "2" (effects (font (size 1.27 1.27)))))
    )
  )
)"#,
            "implicit_default_effects.nsp_sym",
        )
        .unwrap();

    let summary = schematic
        .apply_edit(NspSchematicEdit::PlaceSymbol {
            definition: Box::new(library.symbol("NekoSpice:R").unwrap().clone()),
            library_symbols: library.symbols.clone(),
            reference: "R1".to_string(),
            value: "1k".to_string(),
            at: NspAt {
                x: 10.0,
                y: 10.0,
                rotation: 0.0,
            },
            unit: Some(1),
            body_style: None,
            pin_alternates: BTreeMap::new(),
            uuid: Some("aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa".to_string()),
        })
        .unwrap();

    assert_eq!(summary.operation, "place-symbol");
    assert_eq!(schematic.library_symbols.len(), 1);
    assert_eq!(schematic.symbols[0].reference(), Some("R1"));
}

#[test]
fn places_selected_schema_symbol_unit_and_body_style() {
    let mut schematic = parse_schematic(
        r#"(nsp_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols)
)"#,
        "empty.nsp_sch",
    )
    .unwrap();
    let library = parse_symbol_library(
        r#"(nsp_symbol_lib
  (version 20230121)
  (symbol "NekoSpice:Scoped"
    (property "Reference" "U" (at 0 0 0))
    (property "Value" "Scoped" (at 0 -2.54 0))
    (symbol "Scoped_1_1"
      (pin passive line (at -2.54 0 0) (length 2.54) (name "A1") (number "1"))
      (pin passive line (at 2.54 0 180) (length 2.54) (name "B1") (number "2"))
    )
    (symbol "Scoped_2_2"
      (unit_name "Analog")
      (pin passive line (at -2.54 0 0) (length 2.54) (name "A2") (number "3"))
      (pin passive line
        (at 2.54 0 180)
        (length 2.54)
        (name "B2")
        (number "4")
        (alternate "ALT4" output line)
      )
    )
  )
)"#,
        "scoped.nsp_sym",
    )
    .unwrap();
    let definition = library.symbol("NekoSpice:Scoped").unwrap().clone();

    schematic
        .apply_edit(NspSchematicEdit::PlaceSymbol {
            definition: Box::new(definition),
            library_symbols: library.symbols.clone(),
            reference: "U2".to_string(),
            value: "Scoped".to_string(),
            at: NspAt {
                x: 20.0,
                y: 10.0,
                rotation: 0.0,
            },
            unit: Some(2),
            body_style: Some(2),
            pin_alternates: BTreeMap::from([("4".to_string(), "ALT4".to_string())]),
            uuid: Some("aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa".to_string()),
        })
        .unwrap();

    let placed = schematic
        .symbols
        .iter()
        .find(|symbol| symbol.reference() == Some("U2"))
        .unwrap();
    assert_eq!(placed.unit, Some(2));
    assert_eq!(placed.body_style, Some(2));
    assert_eq!(placed.pins[1].alternate.as_deref(), Some("ALT4"));
    assert_eq!(
        placed
            .pins
            .iter()
            .filter_map(|pin| pin.number.as_deref())
            .collect::<Vec<_>>(),
        vec!["3", "4"]
    );

    let scene = schematic.canvas_scene();
    assert_eq!(scene.symbols[0].unit_name.as_deref(), Some("Analog"));
    assert_eq!(
        scene.symbols[0]
            .pins
            .iter()
            .map(|pin| pin.number.as_str())
            .collect::<Vec<_>>(),
        vec!["3", "4"]
    );

    let exported = schematic.to_schematic_sexpr();
    assert!(exported.contains("(unit 2)"));
    assert!(exported.contains("(body_style 2)"));
    assert!(exported.contains("(pin \"3\""));
    assert!(exported.contains("(alternate \"ALT4\")"));
    assert!(!exported.contains("(pin \"1\""));
    let reparsed = parse_schematic(&exported, "placed_scoped.nsp_sch").unwrap();
    assert_eq!(reparsed.symbols[0].unit, Some(2));
    assert_eq!(reparsed.symbols[0].body_style, Some(2));
    assert_eq!(
        reparsed.symbols[0].pins[1].alternate.as_deref(),
        Some("ALT4")
    );
    assert_eq!(reparsed.canvas_scene().symbols[0].pins.len(), 2);

    let definition = schematic
        .symbol_definition("NekoSpice:Scoped")
        .unwrap()
        .clone();
    let error = schematic
        .apply_edit(NspSchematicEdit::PlaceSymbol {
            definition: Box::new(definition),
            library_symbols: Vec::new(),
            reference: "U3".to_string(),
            value: "Scoped".to_string(),
            at: NspAt {
                x: 30.0,
                y: 10.0,
                rotation: 0.0,
            },
            unit: Some(2),
            body_style: Some(2),
            pin_alternates: BTreeMap::from([("4".to_string(), "MISSING".to_string())]),
            uuid: None,
        })
        .unwrap_err();
    assert!(error.to_string().contains("has no alternate 'MISSING'"));
}

#[test]
fn configures_existing_symbol_scope_mirror_and_pin_alternates() {
    let mut schematic = parse_schematic(
        r#"(nsp_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols
    (symbol "NekoSpice:Scoped"
      (property "Reference" "U" (at 0 0 0))
      (property "Value" "Scoped" (at 0 -2.54 0))
      (symbol "Scoped_1_1"
        (pin passive line (at -2.54 0 0) (length 2.54) (name "A1") (number "1"))
        (pin passive line (at 2.54 0 180) (length 2.54) (name "B1") (number "2"))
      )
      (symbol "Scoped_2_2"
        (unit_name "Analog")
        (pin passive line (at -2.54 0 0) (length 2.54) (name "A2") (number "3"))
        (pin passive line
          (at 2.54 0 180)
          (length 2.54)
          (name "B2")
          (number "4")
          (alternate "ALT4" output line)
        )
      )
    )
  )
  (symbol
    (lib_id "NekoSpice:Scoped")
    (at 20 10 0)
    (unit 1)
    (uuid "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa")
    (property "Reference" "U2" (at 20 7.46 0))
    (property "Value" "Scoped" (at 20 12.54 0))
    (pin "1" (uuid "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbb1"))
    (pin "2" (uuid "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbb2"))
  )
)"#,
        "configure_symbol.nsp_sch",
    )
    .unwrap();

    schematic
        .apply_edit(NspSchematicEdit::ConfigureSymbol {
            reference: "U2".to_string(),
            unit: Some(2),
            body_style: Some(Some(2)),
            mirror: Some(Some("x y".to_string())),
            pin_alternates: Some(BTreeMap::from([("4".to_string(), "ALT4".to_string())])),
        })
        .unwrap();

    let symbol = schematic.symbols[0].clone();
    assert_eq!(symbol.unit, Some(2));
    assert_eq!(symbol.body_style, Some(2));
    assert_eq!(symbol.mirror.as_deref(), Some("x y"));
    assert_eq!(
        symbol
            .pins
            .iter()
            .filter_map(|pin| pin.number.as_deref())
            .collect::<Vec<_>>(),
        vec!["3", "4"]
    );
    assert_eq!(symbol.pins[1].alternate.as_deref(), Some("ALT4"));

    let scene = schematic.canvas_scene();
    assert_eq!(scene.symbols[0].unit_name.as_deref(), Some("Analog"));
    assert_eq!(scene.symbols[0].mirror.as_deref(), Some("x y"));
    let pin3 = scene.symbols[0]
        .pins
        .iter()
        .find(|pin| pin.number == "3")
        .unwrap();
    assert_close(pin3.start.x, 22.54);
    assert_close(pin3.end.x, 20.0);

    let exported = schematic.to_schematic_sexpr();
    assert!(exported.contains("(mirror x y)"));
    assert!(exported.contains("(unit 2)"));
    assert!(exported.contains("(body_style 2)"));
    assert!(exported.contains("(alternate \"ALT4\")"));
    let reparsed = parse_schematic(&exported, "configure_symbol_roundtrip.nsp_sch").unwrap();
    assert_eq!(reparsed.symbols[0].mirror.as_deref(), Some("x y"));
    assert_eq!(
        reparsed.symbols[0].pins[1].alternate.as_deref(),
        Some("ALT4")
    );

    schematic
        .apply_edit(NspSchematicEdit::ConfigureSymbol {
            reference: "U2".to_string(),
            unit: None,
            body_style: Some(None),
            mirror: Some(None),
            pin_alternates: Some(BTreeMap::new()),
        })
        .unwrap();
    assert_eq!(schematic.symbols[0].body_style, None);
    assert_eq!(schematic.symbols[0].mirror, None);
    assert!(
        schematic.symbols[0]
            .pins
            .iter()
            .all(|pin| pin.alternate.is_none())
    );

    let error = schematic
        .apply_edit(NspSchematicEdit::ConfigureSymbol {
            reference: "U2".to_string(),
            unit: Some(2),
            body_style: Some(Some(2)),
            mirror: None,
            pin_alternates: Some(BTreeMap::from([("4".to_string(), "MISSING".to_string())])),
        })
        .unwrap_err();
    assert!(error.to_string().contains("has no alternate 'MISSING'"));
}

#[test]
fn rejects_edit_that_reuses_existing_schema_uuid() {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap();
    let mut schematic =
        read_schematic(&workspace_root.join("examples/schema_schematic/rc.nsp_sch")).unwrap();

    let error = schematic
        .apply_edit(NspSchematicEdit::AddWire {
            points: vec![NspPoint { x: 10.0, y: 10.0 }, NspPoint { x: 20.0, y: 10.0 }],
            uuid: Some("22222222-2222-2222-2222-222222222222".to_string()),
        })
        .unwrap_err();

    assert!(error.to_string().contains("already used"));
}
