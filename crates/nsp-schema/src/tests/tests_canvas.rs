//! Domain-focused tests for nsp-schema.

use super::assert_close;
use crate::{NspPoint, parse_schematic, parse_symbol_library, read_schematic};
use std::path::Path;

#[test]
fn builds_canvas_scene_from_schema_schematic_fixture() {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap();
    let schematic =
        read_schematic(&workspace_root.join("examples/schema_schematic/rc.nsp_sch")).unwrap();

    let scene = schematic.canvas_scene();
    assert_eq!(scene.symbols.len(), 3);
    assert_eq!(
        scene
            .symbols
            .iter()
            .map(|symbol| symbol.graphics.len())
            .sum::<usize>(),
        6
    );
    assert_eq!(
        scene
            .symbols
            .iter()
            .map(|symbol| symbol.pins.len())
            .sum::<usize>(),
        6
    );
    assert_eq!(scene.wires.len(), 3);
    assert_eq!(scene.labels.len(), 3);
    assert_eq!(scene.text_items.len(), 1);
    assert!(scene.text_items[0].is_spice_directive);
    assert!(scene.bounds.unwrap().width() > 20.0);

    let resistor = scene
        .symbols
        .iter()
        .find(|symbol| symbol.reference == "R1")
        .unwrap();
    assert_eq!(resistor.lib_id, "NekoSpice:R");
    assert_eq!(resistor.graphics.len(), 1);
    assert_close(resistor.pins[0].start.x, 67.31);
    assert_close(resistor.pins[0].end.x, 69.85);
    assert!(scene.to_summary_json().contains("\"graphic_count\": 6"));
    assert!(scene.to_summary_json().contains("\"pin_count\": 6"));
    assert!(scene.to_summary_json().contains("\"text_count\": 1"));
    assert!(
        scene
            .to_summary_json()
            .contains("\"spice_directive_count\": 1")
    );
}

#[test]
fn selects_schema_symbol_unit_scope_for_canvas_and_netlist() {
    let schematic = parse_schematic(
        r#"(nsp_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols
    (symbol "NekoSpice:Multi"
      (property "Reference" "U" (at 0 0 0))
      (property "Value" "Multi" (at 0 -2.54 0))
      (property "Sim.Device" "R" (at 0 0 0))
      (symbol "Multi_0_1"
        (rectangle
          (start -1 -1)
          (end 1 1)
          (stroke (width 0) (type default))
          (fill (type none))
        )
      )
      (symbol "Multi_1_1"
        (polyline
          (pts (xy -1 0) (xy 1 0))
          (stroke (width 0.127) (type default))
          (fill (type none))
        )
        (pin passive line (at -2.54 0 0) (length 2.54) (name "A1") (number "1"))
        (pin passive line (at 2.54 0 180) (length 2.54) (name "B1") (number "2"))
      )
      (symbol "Multi_2_1"
        (circle
          (center 0 0)
          (radius 1)
          (stroke (width 0.127) (type default))
          (fill (type none))
        )
        (pin passive line (at -2.54 0 0) (length 2.54) (name "A2") (number "3"))
        (pin passive line (at 2.54 0 180) (length 2.54) (name "B2") (number "4"))
      )
    )
  )
  (wire (pts (xy 17.46 10) (xy 10 10)))
  (wire (pts (xy 22.54 10) (xy 30 10)))
  (label "in" (at 10 10 0))
  (label "0" (at 30 10 0))
  (text ".op" (at 5 5 0))
  (symbol
    (lib_id "NekoSpice:Multi")
    (at 20 10 0)
    (unit 2)
    (body_style 1)
    (property "Reference" "R1" (at 20 8 0))
    (property "Value" "10k" (at 20 12 0))
  )
)"#,
        "multi_unit.nsp_sch",
    )
    .unwrap();

    let definition = schematic.symbol_definition("NekoSpice:Multi").unwrap();
    assert_eq!(definition.graphics[0].unit, 0);
    assert_eq!(definition.graphics[1].unit, 1);
    assert_eq!(definition.graphics[2].unit, 2);
    assert_eq!(definition.pins[0].unit, 1);
    assert_eq!(definition.pins[2].unit, 2);
    assert_eq!(
        definition
            .graphics
            .iter()
            .filter(|graphic| graphic.unit != 0)
            .count(),
        2
    );
    assert_eq!(
        definition.pins.iter().filter(|pin| pin.unit != 0).count(),
        4
    );
    assert!(
        schematic
            .to_summary_json()
            .contains("\"symbol_body_style_count\": 1")
    );

    let scene = schematic.canvas_scene();
    let symbol = scene
        .symbols
        .iter()
        .find(|symbol| symbol.reference == "R1")
        .unwrap();
    assert_eq!(symbol.graphics.len(), 2);
    assert_eq!(symbol.pins.len(), 2);
    assert_eq!(symbol.pins[0].number, "3");
    assert_eq!(symbol.pins[1].number, "4");
    assert!(!symbol.pins.iter().any(|pin| pin.number == "1"));

    let netlist = schematic.to_spice_netlist().unwrap();
    assert!(netlist.contains("R1 in 0 10k"));

    let exported = schematic.to_schematic_sexpr();
    assert!(exported.contains("(body_style 1)"));
    assert!(exported.contains("(symbol \"Multi_0_1\""));
    assert!(exported.contains("(symbol \"Multi_1_1\""));
    assert!(exported.contains("(symbol \"Multi_2_1\""));
    let reparsed = parse_schematic(&exported, "multi_unit_roundtrip.nsp_sch").unwrap();
    assert_eq!(
        reparsed
            .symbols
            .iter()
            .find(|symbol| symbol.reference() == Some("R1"))
            .unwrap()
            .body_style,
        Some(1)
    );
    assert_eq!(
        reparsed
            .canvas_scene()
            .symbols
            .iter()
            .find(|symbol| symbol.reference == "R1")
            .unwrap()
            .pins
            .len(),
        2
    );
}

#[test]
fn preserves_schema_symbol_unit_display_names() {
    let schematic = parse_schematic(
        r#"(nsp_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols
    (symbol "NekoSpice:NamedUnits"
      (property "Reference" "U" (at 0 0 0))
      (property "Value" "NamedUnits" (at 0 -2.54 0))
      (symbol "NamedUnits_1_1"
        (unit_name "Power")
        (pin passive line (at -2.54 0 0) (length 2.54) (name "VIN") (number "1"))
      )
      (symbol "NamedUnits_2_1"
        (unit_name "Logic")
        (pin passive line (at 2.54 0 180) (length 2.54) (name "IO") (number "2"))
      )
    )
  )
  (symbol
    (lib_id "NekoSpice:NamedUnits")
    (at 20 10 0)
    (unit 2)
    (property "Reference" "U1" (at 20 8 0))
    (property "Value" "NamedUnits" (at 20 12 0))
  )
)"#,
        "named_units.nsp_sch",
    )
    .unwrap();

    let definition = schematic.symbol_definition("NekoSpice:NamedUnits").unwrap();
    assert_eq!(
        definition.unit_names.get(&1).map(String::as_str),
        Some("Power")
    );
    assert_eq!(
        definition.unit_names.get(&2).map(String::as_str),
        Some("Logic")
    );
    assert_eq!(
        schematic.canvas_scene().symbols[0].unit_name.as_deref(),
        Some("Logic")
    );
    assert!(
        schematic
            .to_summary_json()
            .contains("\"library_unit_name_count\": 2")
    );

    let exported = schematic.to_schematic_sexpr();
    assert!(exported.contains("(unit_name \"Power\")"));
    assert!(exported.contains("(unit_name \"Logic\")"));
    let reparsed = parse_schematic(&exported, "named_units_roundtrip.nsp_sch").unwrap();
    assert_eq!(
        reparsed
            .symbol_definition("NekoSpice:NamedUnits")
            .unwrap()
            .unit_names
            .get(&2)
            .map(String::as_str),
        Some("Logic")
    );

    let library = parse_symbol_library(
        r#"(nsp_symbol_lib
  (version 20230121)
  (generator "NekoSpice")
  (symbol "NekoSpice:NamedUnits"
    (property "Reference" "U" (at 0 0 0))
    (property "Value" "NamedUnits" (at 0 -2.54 0))
    (symbol "NamedUnits_1_1"
      (unit_name "Power")
      (pin passive line (at -2.54 0 0) (length 2.54) (name "VIN") (number "1"))
    )
    (symbol "NamedUnits_2_1"
      (unit_name "Logic")
      (pin passive line (at 2.54 0 180) (length 2.54) (name "IO") (number "2"))
    )
  )
)"#,
        "named_units.nsp_sym",
    )
    .unwrap();

    assert!(library.to_summary_json().contains("\"unit_name_count\": 2"));
    let exported_library = library.to_symbol_library_sexpr();
    assert!(exported_library.contains("(unit_name \"Power\")"));
    assert!(exported_library.contains("(unit_name \"Logic\")"));
}

#[test]
fn exposes_schema_canvas_item_uuids_for_editor_selection() {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap();
    let schematic =
        read_schematic(&workspace_root.join("examples/schema_schematic/rc.nsp_sch")).unwrap();

    let scene = schematic.canvas_scene();
    assert_eq!(
        scene.symbols[0].uuid.as_deref(),
        Some("88888888-8888-8888-8888-888888888888")
    );
    assert_eq!(
        scene.wires[0].uuid.as_deref(),
        Some("22222222-2222-2222-2222-222222222222")
    );
    assert_eq!(
        scene.labels[1].uuid.as_deref(),
        Some("66666666-6666-6666-6666-666666666666")
    );
    assert_eq!(
        scene.text_items[0].uuid.as_deref(),
        Some("77777777-7777-7777-7777-777777777777")
    );

    let scene_json: serde_json::Value = serde_json::from_str(&scene.to_json()).unwrap();
    assert_eq!(
        scene_json["symbols"][0]["uuid"],
        "88888888-8888-8888-8888-888888888888"
    );
    assert!(
        scene_json["symbols"][0]["bounds"]["width"]
            .as_f64()
            .unwrap()
            > 0.0
    );
    assert_eq!(
        scene_json["wires"][0]["uuid"],
        "22222222-2222-2222-2222-222222222222"
    );
    assert!(scene_json["wires"][0]["bounds"]["width"].as_f64().unwrap() > 16.0);
    assert_eq!(
        scene_json["labels"][1]["uuid"],
        "66666666-6666-6666-6666-666666666666"
    );
    assert!(
        scene_json["labels"][1]["bounds"]["width"].as_f64().unwrap()
            >= crate::geometry::SCHEMA_CANVAS_POINT_BOUNDS_RADIUS * 2.0
    );
    assert_eq!(
        scene_json["text_items"][0]["uuid"],
        "77777777-7777-7777-7777-777777777777"
    );
    assert!(
        scene_json["text_items"][0]["bounds"]["height"]
            .as_f64()
            .unwrap()
            >= crate::geometry::SCHEMA_CANVAS_POINT_BOUNDS_RADIUS * 2.0
    );
}

#[test]
fn finds_schema_canvas_items_by_uuid_for_editor_state() {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap();
    let schematic =
        read_schematic(&workspace_root.join("examples/schema_schematic/rc.nsp_sch")).unwrap();
    let scene = schematic.canvas_scene();

    let wire_hit = scene
        .item_hit_by_uuid("22222222-2222-2222-2222-222222222222")
        .unwrap();
    assert_eq!(wire_hit.kind, "wire");
    assert_eq!(wire_hit.label, "wire");
    assert!(wire_hit.bounds.width() > 16.0);

    let source_hit = scene
        .item_hit_by_uuid("88888888-8888-8888-8888-888888888888")
        .unwrap();
    assert_eq!(source_hit.kind, "symbol");
    assert_eq!(source_hit.label, "V1");

    let resistor_hit = scene
        .item_hit_by_uuid("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa")
        .unwrap();
    assert_eq!(resistor_hit.kind, "symbol");
    assert_eq!(resistor_hit.label, "R1");

    assert!(
        scene
            .item_hit_by_uuid("00000000-0000-4000-8000-000000000000")
            .is_none()
    );
}

#[test]
fn hit_tests_schema_canvas_items_by_bounds() {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap();
    let schematic =
        read_schematic(&workspace_root.join("examples/schema_schematic/rc.nsp_sch")).unwrap();

    let hit_report = schematic
        .canvas_scene()
        .hit_test(NspPoint { x: 88.9, y: 50.8 });

    assert!(hit_report.hit_count >= 2);
    assert_eq!(hit_report.hits[0].kind, "label");
    assert_eq!(
        hit_report.hits[0].uuid.as_deref(),
        Some("66666666-6666-6666-6666-666666666666")
    );
    assert!(hit_report.hits.iter().any(|hit| hit.kind == "wire"
        && hit.uuid.as_deref() == Some("33333333-3333-3333-3333-333333333333")));
    let json: serde_json::Value = serde_json::from_str(&hit_report.to_json()).unwrap();
    assert_eq!(
        json["hit_count"].as_u64().unwrap(),
        hit_report.hit_count as u64
    );
    assert_eq!(json["hits"][0]["kind"], "label");
    assert_eq!(
        json["hits"][0]["uuid"],
        "66666666-6666-6666-6666-666666666666"
    );
    assert!(json["hits"][0]["bounds"]["width"].as_f64().unwrap() > 0.0);

    let empty_report = schematic
        .canvas_scene()
        .hit_test(NspPoint { x: 10.0, y: 10.0 });
    assert_eq!(empty_report.hit_count, 0);
    assert!(empty_report.hits.is_empty());
}

#[test]
fn hit_tests_symbols_by_body_and_pin_geometry() {
    let schematic = parse_schematic(
        r#"(nsp_sch
  (version 20230121)
  (generator "NekoSpice")
  (uuid "11111111-1111-4111-8111-111111111111")
  (paper "A4")
  (lib_symbols
    (symbol "NekoSpice:Sparse"
      (property "Reference" "U" (at 0 0 0))
      (property "Value" "Sparse" (at 0 -2.54 0))
      (symbol "Sparse_0_1"
        (polyline (pts (xy -2.54 0) (xy 2.54 0)))
        (pin passive line (at -5.08 0 0) (length 2.54) (name "A") (number "1"))
      )
    )
  )
  (symbol
    (lib_id "NekoSpice:Sparse")
    (at 20 20 0)
    (property "Reference" "U1" (at 20 17 0))
    (property "Value" "Sparse" (at 20 23 0))
    (uuid "22222222-2222-4222-8222-222222222222")
    (pin "1" (uuid "33333333-3333-4333-8333-333333333333"))
  )
)"#,
        "symbol_hit_test.nsp_sch",
    )
    .unwrap();
    let scene = schematic.canvas_scene();
    let symbol = &scene.symbols[0];
    assert!(
        symbol.bounds.unwrap().height() >= crate::geometry::SCHEMA_CANVAS_LINE_BOUNDS_PADDING * 2.0
    );

    let body_hit = scene.hit_test(NspPoint { x: 20.0, y: 20.4 });
    assert!(body_hit.hits.iter().any(|hit| hit.kind == "symbol"
        && hit.uuid.as_deref() == Some("22222222-2222-4222-8222-222222222222")));

    let pin_hit = scene.hit_test(NspPoint { x: 16.2, y: 20.4 });
    assert!(pin_hit.hits.iter().any(|hit| hit.kind == "symbol"
        && hit.uuid.as_deref() == Some("22222222-2222-4222-8222-222222222222")));

    let bounds_only_miss = scene.hit_test(NspPoint { x: 17.0, y: 20.7 });
    assert!(!bounds_only_miss.hits.iter().any(|hit| hit.kind == "symbol"
        && hit.uuid.as_deref() == Some("22222222-2222-4222-8222-222222222222")));
}

#[test]
fn hit_tests_line_items_by_segment_distance() {
    let schematic = parse_schematic(
            r#"(nsp_sch
  (version 20230121)
  (generator "NekoSpice")
  (uuid "11111111-1111-1111-1111-111111111111")
  (paper "A4")
  (wire (pts (xy 10 10) (xy 30 10)) (stroke (width 0) (type default)) (uuid "22222222-2222-2222-2222-222222222222"))
  (bus (pts (xy 10 20) (xy 30 20)) (stroke (width 0) (type default)) (uuid "33333333-3333-3333-3333-333333333333"))
  (bus_entry (at 30 20) (size 2.54 -2.54) (stroke (width 0) (type default)) (uuid "44444444-4444-4444-4444-444444444444"))
)"#,
            "line_hit_test.nsp_sch",
        )
        .unwrap();
    let scene = schematic.canvas_scene();

    let wire_hit = scene.hit_test(NspPoint { x: 20.0, y: 10.4 });
    assert!(wire_hit.hits.iter().any(|hit| hit.kind == "wire"
        && hit.uuid.as_deref() == Some("22222222-2222-2222-2222-222222222222")));

    let wire_miss_inside_bounds = scene.hit_test(NspPoint { x: 20.0, y: 10.7 });
    assert!(
        !wire_miss_inside_bounds
            .hits
            .iter()
            .any(|hit| hit.kind == "wire"
                && hit.uuid.as_deref() == Some("22222222-2222-2222-2222-222222222222"))
    );

    let bus_hit = scene.hit_test(NspPoint { x: 20.0, y: 20.4 });
    assert!(bus_hit.hits.iter().any(|hit| hit.kind == "bus"
        && hit.uuid.as_deref() == Some("33333333-3333-3333-3333-333333333333")));

    let entry_hit = scene.hit_test(NspPoint { x: 31.27, y: 18.73 });
    assert!(entry_hit.hits.iter().any(|hit| hit.kind == "bus-entry"
        && hit.uuid.as_deref() == Some("44444444-4444-4444-4444-444444444444")));
}

#[test]
fn hit_tests_junctions_and_no_connects_by_shape() {
    let schematic = parse_schematic(
        r#"(nsp_sch
  (version 20230121)
  (generator "NekoSpice")
  (uuid "11111111-1111-4111-8111-111111111111")
  (paper "A4")
  (lib_symbols)
  (junction (at 10 10) (diameter 2.54) (uuid "22222222-2222-4222-8222-222222222222"))
  (no_connect (at 20 10) (uuid "33333333-3333-4333-8333-333333333333"))
)"#,
        "point_shape_hit_test.nsp_sch",
    )
    .unwrap();
    let scene = schematic.canvas_scene();
    assert!(scene.junctions[0].bounds.width() > 2.5);
    assert!(
        scene.no_connects[0].bounds.width() > crate::geometry::SCHEMA_CANVAS_POINT_BOUNDS_RADIUS
    );

    let junction_hit = scene.hit_test(NspPoint { x: 11.0, y: 10.0 });
    assert!(junction_hit.hits.iter().any(|hit| hit.kind == "junction"
        && hit.uuid.as_deref() == Some("22222222-2222-4222-8222-222222222222")));

    let junction_corner_miss = scene.hit_test(NspPoint { x: 10.95, y: 10.95 });
    assert!(
        !junction_corner_miss
            .hits
            .iter()
            .any(|hit| hit.kind == "junction"
                && hit.uuid.as_deref() == Some("22222222-2222-4222-8222-222222222222"))
    );

    let no_connect_hit = scene.hit_test(NspPoint { x: 20.9, y: 10.9 });
    assert!(
        no_connect_hit
            .hits
            .iter()
            .any(|hit| hit.kind == "no-connect"
                && hit.uuid.as_deref() == Some("33333333-3333-4333-8333-333333333333"))
    );

    let no_connect_corner_miss = scene.hit_test(NspPoint { x: 20.95, y: 10.0 });
    assert!(
        !no_connect_corner_miss
            .hits
            .iter()
            .any(|hit| hit.kind == "no-connect"
                && hit.uuid.as_deref() == Some("33333333-3333-4333-8333-333333333333"))
    );
}

#[test]
fn hit_tests_sheet_pins_by_segment_distance() {
    let schematic = parse_schematic(
        r#"(nsp_sch
  (version 20230121)
  (generator "NekoSpice")
  (uuid "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa")
  (paper "A4")
  (lib_symbols)
  (sheet
    (at 50 40)
    (size 30 20)
    (property "Sheetname" "gain_stage" (id 0) (at 50 38 0))
    (property "Sheetfile" "gain_stage.nsp_sch" (id 1) (at 50 62 0))
    (pin "in" input (at 50 45 180) (uuid "11111111-1111-4111-8111-111111111111"))
    (uuid "33333333-3333-4333-8333-333333333333")
  )
)"#,
        "sheet_pin_hit_test.nsp_sch",
    )
    .unwrap();
    let scene = schematic.canvas_scene();
    let pin = &scene.sheets[0].pins[0];
    let pin_bounds = pin.bounds.unwrap();
    assert!(pin_bounds.width() > 2.54);
    assert!(pin_bounds.min.x < 48.0);

    let pin_hit = scene.hit_test(NspPoint { x: 48.73, y: 45.0 });
    assert!(pin_hit.hits.iter().any(|hit| hit.kind == "sheet-pin"
        && hit.uuid.as_deref() == Some("11111111-1111-4111-8111-111111111111")));
    assert!(!pin_hit.hits.iter().any(|hit| hit.kind == "sheet"
        && hit.uuid.as_deref() == Some("33333333-3333-4333-8333-333333333333")));

    let anchor_box_miss = scene.hit_test(NspPoint { x: 50.0, y: 46.2 });
    assert!(
        !anchor_box_miss
            .hits
            .iter()
            .any(|hit| hit.kind == "sheet-pin"
                && hit.uuid.as_deref() == Some("11111111-1111-4111-8111-111111111111"))
    );
    assert!(anchor_box_miss.hits.iter().any(|hit| hit.kind == "sheet"
        && hit.uuid.as_deref() == Some("33333333-3333-4333-8333-333333333333")));

    let corner_miss = scene.hit_test(NspPoint { x: 46.86, y: 45.62 });
    assert!(!corner_miss.hits.iter().any(|hit| hit.kind == "sheet-pin"
        && hit.uuid.as_deref() == Some("11111111-1111-4111-8111-111111111111")));
    assert!(!corner_miss.hits.iter().any(|hit| hit.kind == "sheet"
        && hit.uuid.as_deref() == Some("33333333-3333-4333-8333-333333333333")));
}

#[test]
fn hit_tests_directive_labels_by_segment_and_text_bounds() {
    let schematic = parse_schematic(
        r#"(nsp_sch
  (version 20230121)
  (generator "NekoSpice")
  (uuid "11111111-1111-4111-8111-111111111111")
  (paper "A4")
  (lib_symbols)
  (netclass_flag ""
    (length 3.81)
    (shape dot)
    (at 50 40 0)
    (effects (font (size 1.27 1.27)))
    (uuid "22222222-2222-4222-8222-222222222222")
    (property "Net Class" "HV" (at 50 38 0))
  )
)"#,
        "directive_label_hit_test.nsp_sch",
    )
    .unwrap();
    let scene = schematic.canvas_scene();
    let label = &scene.directive_labels[0];
    let bounds = label.bounds.unwrap();
    assert!(bounds.width() > 4.0);
    assert!(bounds.height() > 2.0);

    let segment_hit = scene.hit_test(NspPoint { x: 52.0, y: 40.4 });
    assert!(
        segment_hit
            .hits
            .iter()
            .any(|hit| hit.kind == "directive-label"
                && hit.uuid.as_deref() == Some("22222222-2222-4222-8222-222222222222"))
    );

    let text_hit = scene.hit_test(NspPoint { x: 51.0, y: 41.0 });
    assert!(text_hit.hits.iter().any(|hit| hit.kind == "directive-label"
        && hit.uuid.as_deref() == Some("22222222-2222-4222-8222-222222222222")));

    let bounds_only_miss = scene.hit_test(NspPoint { x: 54.0, y: 41.5 });
    assert!(
        !bounds_only_miss
            .hits
            .iter()
            .any(|hit| hit.kind == "directive-label"
                && hit.uuid.as_deref() == Some("22222222-2222-4222-8222-222222222222"))
    );
}

#[test]
fn hit_tests_text_items_by_estimated_text_bounds() {
    let schematic = parse_schematic(
            r#"(nsp_sch
  (version 20230121)
  (generator "NekoSpice")
  (uuid "11111111-1111-1111-1111-111111111111")
  (paper "A4")
  (label "LONG_LABEL" (at 10 10 0) (effects (font (size 1.27 1.27))) (uuid "22222222-2222-2222-2222-222222222222"))
  (text "First line\nSecond line" (at 10 20 0) (effects (font (size 1.27 1.27))) (uuid "33333333-3333-3333-3333-333333333333"))
  (text "RIGHT" (at 40 10 0) (effects (font (size 1.27 1.27)) (justify right)) (uuid "44444444-4444-4444-4444-444444444444"))
)"#,
            "text_hit_test.nsp_sch",
        )
        .unwrap();
    let scene = schematic.canvas_scene();

    let label_hit = scene.hit_test(NspPoint { x: 16.0, y: 10.7 });
    assert!(label_hit.hits.iter().any(|hit| hit.kind == "label"
        && hit.uuid.as_deref() == Some("22222222-2222-2222-2222-222222222222")));

    let label_miss = scene.hit_test(NspPoint { x: 21.0, y: 10.7 });
    assert!(!label_miss.hits.iter().any(|hit| hit.kind == "label"
        && hit.uuid.as_deref() == Some("22222222-2222-2222-2222-222222222222")));

    let multiline_hit = scene.hit_test(NspPoint { x: 13.0, y: 22.5 });
    assert!(multiline_hit.hits.iter().any(|hit| hit.kind == "text"
        && hit.uuid.as_deref() == Some("33333333-3333-3333-3333-333333333333")));

    let right_justified_hit = scene.hit_test(NspPoint { x: 37.0, y: 10.7 });
    assert!(right_justified_hit.hits.iter().any(|hit| hit.kind == "text"
        && hit.uuid.as_deref() == Some("44444444-4444-4444-4444-444444444444")));

    let right_justified_miss = scene.hit_test(NspPoint { x: 42.0, y: 10.7 });
    assert!(
        !right_justified_miss
            .hits
            .iter()
            .any(|hit| hit.kind == "text"
                && hit.uuid.as_deref() == Some("44444444-4444-4444-4444-444444444444"))
    );
}

#[test]
fn hit_tests_schematic_graphics_by_shape() {
    let schematic = parse_schematic(
            r#"(nsp_sch
  (version 20230121)
  (generator "NekoSpice")
  (uuid "11111111-1111-1111-1111-111111111111")
  (paper "A4")
  (rectangle (start 10 10) (end 20 20) (stroke (width 0) (type default)) (fill (type none)) (uuid "22222222-2222-2222-2222-222222222222"))
  (rectangle (start 30 10) (end 40 20) (stroke (width 0) (type default)) (fill (type color) (color 255 228 206 0.5)) (uuid "33333333-3333-3333-3333-333333333333"))
  (circle (center 55 15) (radius 5) (stroke (width 0) (type default)) (fill (type none)) (uuid "44444444-4444-4444-4444-444444444444"))
  (polyline (pts (xy 10 30) (xy 20 30) (xy 20 40)) (stroke (width 0) (type default)) (fill (type none)) (uuid "55555555-5555-5555-5555-555555555555"))
)"#,
            "graphic_hit_test.nsp_sch",
        )
        .unwrap();
    let scene = schematic.canvas_scene();

    let hollow_rectangle_center = scene.hit_test(NspPoint { x: 15.0, y: 15.0 });
    assert!(
        !hollow_rectangle_center
            .hits
            .iter()
            .any(|hit| hit.kind == "graphic"
                && hit.uuid.as_deref() == Some("22222222-2222-2222-2222-222222222222"))
    );

    let hollow_rectangle_edge = scene.hit_test(NspPoint { x: 15.0, y: 10.4 });
    assert!(
        hollow_rectangle_edge
            .hits
            .iter()
            .any(|hit| hit.kind == "graphic"
                && hit.label == "rectangle"
                && hit.uuid.as_deref() == Some("22222222-2222-2222-2222-222222222222"))
    );

    let filled_rectangle_center = scene.hit_test(NspPoint { x: 35.0, y: 15.0 });
    assert!(
        filled_rectangle_center
            .hits
            .iter()
            .any(|hit| hit.kind == "graphic"
                && hit.label == "rectangle"
                && hit.uuid.as_deref() == Some("33333333-3333-3333-3333-333333333333"))
    );

    let hollow_circle_center = scene.hit_test(NspPoint { x: 55.0, y: 15.0 });
    assert!(
        !hollow_circle_center
            .hits
            .iter()
            .any(|hit| hit.kind == "graphic"
                && hit.uuid.as_deref() == Some("44444444-4444-4444-4444-444444444444"))
    );

    let hollow_circle_edge = scene.hit_test(NspPoint { x: 60.0, y: 15.0 });
    assert!(
        hollow_circle_edge
            .hits
            .iter()
            .any(|hit| hit.kind == "graphic"
                && hit.label == "circle"
                && hit.uuid.as_deref() == Some("44444444-4444-4444-4444-444444444444"))
    );

    let polyline_hit = scene.hit_test(NspPoint { x: 20.0, y: 35.0 });
    assert!(polyline_hit.hits.iter().any(|hit| hit.kind == "graphic"
        && hit.label == "polyline"
        && hit.uuid.as_deref() == Some("55555555-5555-5555-5555-555555555555")));
}

#[test]
fn hit_tests_bezier_graphics_by_sampled_curve() {
    let schematic = parse_schematic(
        r#"(nsp_sch
  (version 20230121)
  (generator "NekoSpice")
  (uuid "11111111-1111-1111-1111-111111111111")
  (paper "A4")
  (bezier
    (pts (xy 10 20) (xy 10 10) (xy 30 10) (xy 30 20))
    (stroke (width 0) (type default))
    (fill (type none))
    (uuid "22222222-2222-2222-2222-222222222222")
  )
)"#,
        "bezier_hit_test.nsp_sch",
    )
    .unwrap();
    let scene = schematic.canvas_scene();

    let curve_hit = scene.hit_test(NspPoint { x: 20.0, y: 12.5 });
    assert!(curve_hit.hits.iter().any(|hit| hit.kind == "graphic"
        && hit.label == "bezier"
        && hit.uuid.as_deref() == Some("22222222-2222-2222-2222-222222222222")));

    let control_polygon_miss = scene.hit_test(NspPoint { x: 20.0, y: 10.0 });
    assert!(
        !control_polygon_miss
            .hits
            .iter()
            .any(|hit| hit.kind == "graphic"
                && hit.uuid.as_deref() == Some("22222222-2222-2222-2222-222222222222"))
    );
}

#[test]
fn hit_tests_arc_graphics_by_sampled_curve() {
    let schematic = parse_schematic(
        r#"(nsp_sch
  (version 20230121)
  (generator "NekoSpice")
  (uuid "11111111-1111-1111-1111-111111111111")
  (paper "A4")
  (arc
    (start 10 20)
    (mid 20 10)
    (end 30 20)
    (stroke (width 0) (type default))
    (fill (type none))
    (uuid "22222222-2222-2222-2222-222222222222")
  )
)"#,
        "arc_hit_test.nsp_sch",
    )
    .unwrap();
    let scene = schematic.canvas_scene();

    let curve_hit = scene.hit_test(NspPoint { x: 20.0, y: 10.0 });
    assert!(curve_hit.hits.iter().any(|hit| hit.kind == "graphic"
        && hit.label == "arc"
        && hit.uuid.as_deref() == Some("22222222-2222-2222-2222-222222222222")));

    let chord_miss = scene.hit_test(NspPoint { x: 15.0, y: 15.0 });
    assert!(!chord_miss.hits.iter().any(|hit| hit.kind == "graphic"
        && hit.uuid.as_deref() == Some("22222222-2222-2222-2222-222222222222")));
}

#[test]
fn hit_tests_rule_areas_by_polygon_shape() {
    let schematic = parse_schematic(
        r#"(nsp_sch
  (version 20230121)
  (generator "NekoSpice")
  (uuid "11111111-1111-1111-1111-111111111111")
  (paper "A4")
  (rule_area
    (polyline
      (pts (xy 10 10) (xy 20 10) (xy 20 20) (xy 10 20))
      (stroke (width 0) (type default))
      (fill (type none))
      (uuid "22222222-2222-2222-2222-222222222222")
    )
  )
  (rule_area
    (polyline
      (pts (xy 30 10) (xy 40 10) (xy 40 20) (xy 30 20))
      (stroke (width 0) (type default))
      (fill (type color) (color 20 200 170 0.25))
      (uuid "33333333-3333-3333-3333-333333333333")
    )
  )
)"#,
        "rule_area_hit_test.nsp_sch",
    )
    .unwrap();
    let scene = schematic.canvas_scene();

    let hollow_center = scene.hit_test(NspPoint { x: 15.0, y: 15.0 });
    assert!(!hollow_center.hits.iter().any(|hit| hit.kind == "rule-area"
        && hit.uuid.as_deref() == Some("22222222-2222-2222-2222-222222222222")));

    let hollow_edge = scene.hit_test(NspPoint { x: 15.0, y: 10.4 });
    assert!(hollow_edge.hits.iter().any(|hit| hit.kind == "rule-area"
        && hit.uuid.as_deref() == Some("22222222-2222-2222-2222-222222222222")));

    let filled_center = scene.hit_test(NspPoint { x: 35.0, y: 15.0 });
    assert!(filled_center.hits.iter().any(|hit| hit.kind == "rule-area"
        && hit.uuid.as_deref() == Some("33333333-3333-3333-3333-333333333333")));
}
