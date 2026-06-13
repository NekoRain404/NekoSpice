//! Domain-focused tests for nsp-schema.

use super::assert_close;
use crate::{NspColor, parse_schematic};

#[test]
fn honors_no_connect_markers_on_unconnected_symbol_pins() {
    let schematic = parse_schematic(
        r#"(nsp_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols
    (symbol "NekoSpice:R"
      (property "Reference" "R" (at 0 0 0))
      (property "Value" "1k" (at 0 -2.54 0))
      (property "Sim.Device" "R" (at 0 0 0))
      (symbol "R_0_1"
        (pin passive line (at -2.54 0 0) (length 2.54) (name "~") (number "1"))
        (pin passive line (at 2.54 0 180) (length 2.54) (name "~") (number "2"))
      )
    )
  )
  (label "0" (at 15.08 10 0))
  (no_connect (at 10 10) (uuid "12121212-1212-1212-1212-121212121212"))
  (text ".op" (at 5 5 0))
  (symbol
    (lib_id "NekoSpice:R")
    (at 12.54 10 0)
    (property "Reference" "R1" (at 12.54 8 0))
    (property "Value" "1k" (at 12.54 12 0))
    (pin "1" (uuid "abababab-0000-0000-0000-000000000001"))
    (pin "2" (uuid "abababab-0000-0000-0000-000000000002"))
  )
)"#,
        "no_connect.nsp_sch",
    )
    .unwrap();

    assert_eq!(schematic.no_connects.len(), 1);
    assert_eq!(
        schematic.no_connects[0].uuid.as_deref(),
        Some("12121212-1212-1212-1212-121212121212")
    );
    assert!(
        schematic
            .to_summary_json()
            .contains("\"no_connect_count\": 1")
    );

    let report = schematic.check_report();
    assert!(
        report.error_count() <= 3,
        "lib resolution warnings OK in test env"
    );
    assert!(!report.diagnostics.iter().any(|diagnostic| {
        matches!(
            diagnostic.code.as_str(),
            "unconnected-pin" | "generated-net-name" | "floating-no-connect"
        )
    }));

    let roundtrip = schematic.to_schematic_sexpr();
    assert!(roundtrip.contains("(no_connect"));
    assert!(roundtrip.contains("(uuid \"12121212-1212-1212-1212-121212121212\")"));
    let reparsed = parse_schematic(&roundtrip, "roundtrip.nsp_sch").unwrap();
    assert_eq!(reparsed.no_connects.len(), 1);
    assert_eq!(reparsed.canvas_scene().no_connects.len(), 1);
}

#[test]
fn preserves_schematic_file_metadata_and_instances() {
    let schematic = parse_schematic(
        r#"(nsp_sch
  (version 20230121)
  (generator "eeschema")
  (generator_version "9.99")
  (uuid "10101010-1010-4010-8010-101010101010")
  (paper "A4")
  (title_block
    (title "Control Board")
    (date "2026-06-09")
    (rev "A")
    (company "NekoSpice")
    (comment 1 "simulation front-end")
    (comment 4 "${APPROVER}")
  )
  (lib_symbols)
  (symbol
    (lib_id "Device:R")
    (at 10 20 0)
    (unit 1)
    (uuid "20202020-2020-4020-8020-202020202020")
    (property "Reference" "R1" (at 10 17.46 0))
    (property "Value" "1k" (at 10 22.54 0))
    (pin "1" (uuid "30303030-3030-4030-8030-303030303030"))
    (pin "2" (uuid "40404040-4040-4040-8040-404040404040"))
  )
  (sheet_instances
    (path "/" (page "1"))
    (path "/aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee" (page "2"))
  )
  (symbol_instances
    (path "/20202020-2020-4020-8020-202020202020"
      (reference "R1")
      (unit 1)
      (value "1k")
      (footprint "")
    )
  )
  (embedded_fonts no)
)"#,
        "metadata.nsp_sch",
    )
    .unwrap();

    assert_eq!(schematic.generator_version.as_deref(), Some("9.99"));
    assert_eq!(schematic.embedded_fonts, Some(false));
    let title_block = schematic.title_block.as_ref().unwrap();
    assert_eq!(title_block.title.as_deref(), Some("Control Board"));
    assert_eq!(title_block.revision.as_deref(), Some("A"));
    assert_eq!(title_block.comments.len(), 2);
    assert_eq!(title_block.comments[1].index, 4);
    assert_eq!(title_block.comments[1].text, "${APPROVER}");
    assert_eq!(schematic.sheet_instances.len(), 2);
    assert_eq!(schematic.sheet_instances[1].page.as_deref(), Some("2"));
    assert_eq!(schematic.symbol_instances.len(), 1);
    assert_eq!(
        schematic.symbol_instances[0].path,
        "/20202020-2020-4020-8020-202020202020"
    );
    assert_eq!(
        schematic.symbol_instances[0].reference.as_deref(),
        Some("R1")
    );
    assert_eq!(schematic.symbol_instances[0].unit, Some(1));
    assert!(
        schematic
            .to_summary_json()
            .contains("\"has_title_block\": true")
    );
    assert!(
        schematic
            .to_summary_json()
            .contains("\"title_comment_count\": 2")
    );
    assert!(
        schematic
            .to_summary_json()
            .contains("\"sheet_instance_count\": 2")
    );
    assert!(
        schematic
            .to_summary_json()
            .contains("\"symbol_instance_count\": 1")
    );
    assert!(
        schematic
            .to_summary_json()
            .contains("\"embedded_fonts\": false")
    );

    let roundtrip = schematic.to_schematic_sexpr();
    assert!(roundtrip.contains("(generator_version \"9.99\")"));
    assert!(roundtrip.contains("(title \"Control Board\")"));
    assert!(roundtrip.contains("(comment 4 \"${APPROVER}\")"));
    assert!(roundtrip.contains("(sheet_instances"));
    assert!(roundtrip.contains("(path \"/\" (page \"1\"))"));
    assert!(roundtrip.contains("(symbol_instances"));
    assert!(roundtrip.contains("(reference \"R1\")"));
    assert!(roundtrip.contains("(embedded_fonts no)"));

    let reparsed = parse_schematic(&roundtrip, "metadata_roundtrip.nsp_sch").unwrap();
    assert_eq!(reparsed.generator_version.as_deref(), Some("9.99"));
    assert_eq!(reparsed.title_block.unwrap().comments.len(), 2);
    assert_eq!(reparsed.sheet_instances.len(), 2);
    assert_eq!(reparsed.symbol_instances.len(), 1);
    assert_eq!(reparsed.embedded_fonts, Some(false));
}

#[test]
fn preserves_symbol_instance_pin_alternates_and_roundtrips() {
    let schematic = parse_schematic(
        r#"(nsp_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols)
  (symbol
    (lib_id "NekoSpice:AltPin")
    (at 10 20 0)
    (unit 1)
    (uuid "20202020-2020-4020-8020-202020202020")
    (property "Reference" "U1" (at 10 17.46 0))
    (property "Value" "AltPin" (at 10 22.54 0))
    (pin "G39"
      (uuid "30303030-3030-4030-8030-303030303030")
      (alternate "CAN0_DIN")
    )
    (pin "G38"
      (uuid "40404040-4040-4040-8040-404040404040")
      (alternate "CAN0_DOUT")
    )
  )
)"#,
        "symbol_pin_alternates.nsp_sch",
    )
    .unwrap();

    assert_eq!(schematic.symbols.len(), 1);
    assert_eq!(schematic.symbols[0].pins.len(), 2);
    assert_eq!(
        schematic.symbols[0].pins[0].alternate.as_deref(),
        Some("CAN0_DIN")
    );
    assert!(
        schematic
            .to_summary_json()
            .contains("\"symbol_pin_alternate_count\": 2")
    );

    let exported = schematic.to_schematic_sexpr();
    assert!(exported.contains("(alternate \"CAN0_DIN\")"));
    assert!(exported.contains("(alternate \"CAN0_DOUT\")"));

    let reparsed = parse_schematic(&exported, "symbol_pin_alternates_roundtrip.nsp_sch").unwrap();
    assert_eq!(
        reparsed.symbols[0].pins[1].alternate.as_deref(),
        Some("CAN0_DOUT")
    );
}

#[test]
fn preserves_embedded_project_instances_and_variants() {
    let schematic = parse_schematic(
        r#"(nsp_sch
  (version 20251028)
  (generator "eeschema")
  (paper "A4")
  (lib_symbols)
  (symbol
    (lib_id "Connector:J")
    (at 10 20 0)
    (unit 1)
    (uuid "11111111-1111-4111-8111-111111111111")
    (property "Reference" "J1" (at 10 17.46 0))
    (property "Value" "Conn" (at 10 22.54 0))
    (pin "1" (uuid "22222222-2222-4222-8222-222222222222"))
    (instances
      (project "variants"
        (path "/aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee"
          (reference "J1")
          (unit 1)
          (variant
            (name "Variant 1")
            (dnp yes)
          )
        )
      )
    )
  )
  (sheet
    (at 40 20)
    (size 20 10)
    (uuid "33333333-3333-4333-8333-333333333333")
    (property "Sheetname" "Sub" (at 40 17.46 0))
    (property "Sheetfile" "sub.nsp_sch" (at 40 32.54 0))
    (instances
      (project "variants"
        (path "/33333333-3333-4333-8333-333333333333"
          (page "2")
        )
      )
    )
  )
)"#,
        "embedded_instances.nsp_sch",
    )
    .unwrap();

    assert_eq!(schematic.symbols[0].instances.len(), 1);
    assert_eq!(schematic.symbols[0].instances[0].name, "variants");
    assert_eq!(schematic.symbols[0].instances[0].paths.len(), 1);
    let symbol_path = &schematic.symbols[0].instances[0].paths[0];
    assert_eq!(symbol_path.reference.as_deref(), Some("J1"));
    assert_eq!(symbol_path.unit, Some(1));
    assert_eq!(symbol_path.variants.len(), 1);
    assert_eq!(symbol_path.variants[0].name.as_deref(), Some("Variant 1"));
    assert_eq!(symbol_path.variants[0].dnp, Some(true));
    assert_eq!(schematic.sheets[0].instances.len(), 1);
    assert_eq!(
        schematic.sheets[0].instances[0].paths[0].page.as_deref(),
        Some("2")
    );
    assert!(
        schematic
            .to_summary_json()
            .contains("\"embedded_project_instance_count\": 2")
    );
    assert!(
        schematic
            .to_summary_json()
            .contains("\"embedded_instance_path_count\": 2")
    );
    assert!(
        schematic
            .to_summary_json()
            .contains("\"variant_instance_count\": 1")
    );

    let roundtrip = schematic.to_schematic_sexpr();
    assert!(roundtrip.contains("(instances"));
    assert!(roundtrip.contains("(project \"variants\""));
    assert!(roundtrip.contains("(reference \"J1\")"));
    assert!(roundtrip.contains("(name \"Variant 1\")"));
    assert!(roundtrip.contains("(dnp yes)"));
    assert!(roundtrip.contains("(page \"2\")"));
    let reparsed = parse_schematic(&roundtrip, "embedded_instances_roundtrip.nsp_sch").unwrap();
    assert_eq!(reparsed.symbols[0].instances[0].paths[0].variants.len(), 1);
    assert_eq!(
        reparsed.sheets[0].instances[0].paths[0].page.as_deref(),
        Some("2")
    );
}

#[test]
fn preserves_symbol_and_sheet_assembly_flags() {
    let schematic = parse_schematic(
        r#"(nsp_sch
  (version 20251028)
  (generator "eeschema")
  (paper "A4")
  (lib_symbols)
  (symbol
    (lib_id "Device:R")
    (at 10 20 0)
    (mirror x y)
    (unit 1)
    (exclude_from_sim no)
    (in_bom no)
    (on_board yes)
    (dnp yes)
    (fields_autoplaced yes)
    (uuid "11111111-1111-4111-8111-111111111111")
    (property "Reference" "Rskip" (at 10 17.46 0))
    (property "Value" "DNP" (at 10 22.54 0))
    (pin "1" (uuid "22222222-2222-4222-8222-222222222222"))
  )
  (sheet
    (at 40 20)
    (size 20 10)
    (exclude_from_sim no)
    (in_bom yes)
    (on_board no)
    (dnp no)
    (fields_autoplaced yes)
    (uuid "33333333-3333-4333-8333-333333333333")
    (property "Sheetname" "Sub" (at 40 17.46 0))
    (property "Sheetfile" "sub.nsp_sch" (at 40 32.54 0))
  )
)"#,
        "assembly_flags.nsp_sch",
    )
    .unwrap();

    assert_eq!(schematic.symbols[0].mirror.as_deref(), Some("x y"));
    assert_eq!(schematic.symbols[0].in_bom, Some(false));
    assert_eq!(schematic.symbols[0].on_board, Some(true));
    assert_eq!(schematic.symbols[0].dnp, Some(true));
    assert_eq!(schematic.symbols[0].fields_autoplaced, Some(true));
    assert_eq!(schematic.sheets[0].in_bom, Some(true));
    assert_eq!(schematic.sheets[0].on_board, Some(false));
    assert_eq!(schematic.sheets[0].dnp, Some(false));
    assert_eq!(schematic.sheets[0].fields_autoplaced, Some(true));
    assert!(
        schematic
            .to_summary_json()
            .contains("\"dnp_item_count\": 1")
    );
    assert!(
        schematic
            .to_summary_json()
            .contains("\"bom_excluded_count\": 1")
    );
    assert!(
        schematic
            .to_summary_json()
            .contains("\"board_excluded_count\": 1")
    );
    assert!(
        schematic
            .to_summary_json()
            .contains("\"mirrored_symbol_count\": 1")
    );
    assert!(
        schematic
            .to_summary_json()
            .contains("\"fields_autoplaced_count\": 2")
    );

    let roundtrip = schematic.to_schematic_sexpr();
    assert!(roundtrip.contains("(mirror x y)"));
    assert!(roundtrip.contains("(in_bom no)"));
    assert!(roundtrip.contains("(on_board yes)"));
    assert!(roundtrip.contains("(dnp yes)"));
    assert!(roundtrip.contains("(fields_autoplaced yes)"));
    assert!(roundtrip.contains("(on_board no)"));
    let reparsed = parse_schematic(&roundtrip, "assembly_flags_roundtrip.nsp_sch").unwrap();
    assert_eq!(reparsed.symbols[0].mirror.as_deref(), Some("x y"));
    assert_eq!(reparsed.symbols[0].dnp, Some(true));
    assert_eq!(reparsed.sheets[0].on_board, Some(false));
    assert_eq!(reparsed.sheets[0].fields_autoplaced, Some(true));
}

#[test]
fn preserves_property_display_flags_and_effects() {
    let schematic = parse_schematic(
        r#"(nsp_sch
  (version 20251028)
  (generator "eeschema")
  (paper "A4")
  (lib_symbols)
  (symbol
    (lib_id "Device:R")
    (at 10 20 0)
    (unit 1)
    (uuid "11111111-1111-4111-8111-111111111111")
    (property "Reference" "R1"
      (id 0)
      (at 10 17.46 0)
      (hide yes)
      (show_name no)
      (do_not_autoplace no)
      (effects
        (font
          (size 1.524 1.016)
          (thickness 0.254)
          (bold yes)
          (italic yes)
          (color 10 9 37 1)
        )
        (justify left bottom)
        (href "https://schema.org")
      )
    )
    (property "Value" "1k"
      (at 10 22.54 0)
      (effects
        (font
          (size 1.27 1.27)
        )
      )
    )
    (pin "1" (uuid "22222222-2222-4222-8222-222222222222"))
  )
)"#,
        "property_effects.nsp_sch",
    )
    .unwrap();

    let property = &schematic.symbols[0].properties[0];
    assert_eq!(property.id, Some(0));
    assert_eq!(property.hide, Some(true));
    assert_eq!(property.show_name, Some(false));
    assert_eq!(property.do_not_autoplace, Some(false));
    let effects = property.effects.as_ref().unwrap();
    assert_close(effects.font_size.unwrap().width, 1.524);
    assert_close(effects.font_size.unwrap().height, 1.016);
    assert_close(effects.font_thickness.unwrap(), 0.254);
    assert_eq!(effects.font_bold, Some(true));
    assert_eq!(effects.font_italic, Some(true));
    assert_eq!(
        effects.font_color,
        Some(NspColor {
            red: 10.0,
            green: 9.0,
            blue: 37.0,
            alpha: 1.0,
        })
    );
    assert_eq!(
        effects.justify,
        vec!["left".to_string(), "bottom".to_string()]
    );
    assert_eq!(effects.href.as_deref(), Some("https://schema.org"));
    assert!(
        schematic
            .to_summary_json()
            .contains("\"hidden_property_count\": 1")
    );
    assert!(
        schematic
            .to_summary_json()
            .contains("\"property_effect_count\": 2")
    );

    let roundtrip = schematic.to_schematic_sexpr();
    assert!(roundtrip.contains("(hide yes)"));
    assert!(roundtrip.contains("(id 0)"));
    assert!(roundtrip.contains("(show_name no)"));
    assert!(roundtrip.contains("(do_not_autoplace no)"));
    assert!(roundtrip.contains("(font (size 1.524 1.016)"));
    assert!(roundtrip.contains("(thickness 0.254)"));
    assert!(roundtrip.contains("(bold yes)"));
    assert!(roundtrip.contains("(italic yes)"));
    assert!(roundtrip.contains("(color 10 9 37 1)"));
    assert!(roundtrip.contains("(justify left bottom)"));
    assert!(roundtrip.contains("(href \"https://schema.org\")"));
    let reparsed = parse_schematic(&roundtrip, "property_effects_roundtrip.nsp_sch").unwrap();
    let property = &reparsed.symbols[0].properties[0];
    assert_eq!(property.id, Some(0));
    assert_eq!(property.hide, Some(true));
    assert_eq!(property.show_name, Some(false));
    assert_eq!(property.do_not_autoplace, Some(false));
    assert_eq!(property.effects.as_ref().unwrap().font_bold, Some(true));
    assert_eq!(
        property.effects.as_ref().unwrap().justify,
        vec!["left".to_string(), "bottom".to_string()]
    );
}

#[test]
fn preserves_canvas_text_effects() {
    let schematic = parse_schematic(
        r#"(nsp_sch
  (version 20251028)
  (generator "eeschema")
  (paper "A4")
  (lib_symbols)
  (label "OUT"
    (at 10 5 0)
    (effects (font (size 1.27 1.27) italic) (justify left bottom) hide)
    (uuid "11111111-1111-4111-8111-111111111111")
  )
  (text "note"
    (at 20 5 0)
    (effects
      (font
        (size 1.905 1.905)
        (thickness 0.254)
        (bold yes)
        (color 10 9 37 1)
      )
      (justify right)
      (href "https://schema.org")
    )
    (uuid "22222222-2222-4222-8222-222222222222")
  )
  (text_box "box"
    (at 30 5 0)
    (size 10 5)
    (effects (font (size 1.27 1.27) (italic yes)) (justify center))
    (uuid "33333333-3333-4333-8333-333333333333")
  )
  (sheet
    (at 40 5)
    (size 15 10)
    (uuid "44444444-4444-4444-8444-444444444444")
    (property "Sheetname" "Sub" (at 40 4 0))
    (property "Sheetfile" "sub.nsp_sch" (at 40 16 0))
    (pin "BUS{0}" bidirectional
      (at 55 10 0)
      (effects (font (size 1.27 1.27)) (justify right))
      (uuid "55555555-5555-4555-8555-555555555555")
    )
  )
)"#,
        "canvas_text_effects.nsp_sch",
    )
    .unwrap();

    let label_effects = schematic.labels[0].effects.as_ref().unwrap();
    assert_eq!(label_effects.font_italic, Some(true));
    assert_eq!(
        label_effects.justify,
        vec!["left".to_string(), "bottom".to_string()]
    );
    assert!(label_effects.hide);

    let text_effects = schematic.text_items[0].effects.as_ref().unwrap();
    assert_close(text_effects.font_size.unwrap().width, 1.905);
    assert_close(text_effects.font_thickness.unwrap(), 0.254);
    assert_eq!(text_effects.font_bold, Some(true));
    assert_eq!(
        text_effects.font_color,
        Some(NspColor {
            red: 10.0,
            green: 9.0,
            blue: 37.0,
            alpha: 1.0,
        })
    );
    assert_eq!(text_effects.href.as_deref(), Some("https://schema.org"));
    assert_eq!(
        schematic.text_boxes[0]
            .effects
            .as_ref()
            .unwrap()
            .font_italic,
        Some(true)
    );
    assert_eq!(
        schematic.sheets[0].pins[0]
            .effects
            .as_ref()
            .unwrap()
            .justify,
        vec!["right".to_string()]
    );

    let scene = schematic.canvas_scene();
    assert!(scene.labels[0].effects.as_ref().unwrap().hide);
    assert_eq!(
        scene.text_items[0]
            .effects
            .as_ref()
            .unwrap()
            .href
            .as_deref(),
        Some("https://schema.org")
    );
    assert_eq!(
        scene.text_boxes[0].effects.as_ref().unwrap().font_italic,
        Some(true)
    );
    assert_eq!(
        scene.sheets[0].pins[0].effects.as_ref().unwrap().justify,
        vec!["right".to_string()]
    );
    let scene_json: serde_json::Value = serde_json::from_str(&scene.to_json()).unwrap();
    assert_eq!(scene_json["sheet_count"], 1);
    assert_eq!(scene_json["sheet_pin_count"], 1);
    assert_eq!(scene_json["label_count"], 1);
    assert_eq!(scene_json["text_box_count"], 1);
    assert_eq!(scene_json["sheets"][0]["name"], "Sub");
    assert_eq!(
        scene_json["sheets"][0]["pins"][0]["effects"]["justify"][0],
        "right"
    );
    assert_eq!(scene_json["labels"][0]["effects"]["hide"], true);
    assert_eq!(
        scene_json["text_items"][0]["effects"]["href"],
        "https://schema.org"
    );
    assert_eq!(scene_json["text_boxes"][0]["effects"]["font_italic"], true);

    let roundtrip = schematic.to_schematic_sexpr();
    assert!(roundtrip.contains("(justify left bottom) hide"));
    assert!(roundtrip.contains("(thickness 0.254)"));
    assert!(roundtrip.contains("(bold yes)"));
    assert!(roundtrip.contains("(color 10 9 37 1)"));
    assert!(roundtrip.contains("(href \"https://schema.org\")"));
    assert!(roundtrip.contains("(justify right)"));
    let reparsed = parse_schematic(&roundtrip, "canvas_text_effects_roundtrip.nsp_sch").unwrap();
    assert_eq!(
        reparsed.labels[0].effects.as_ref().unwrap().font_italic,
        Some(true)
    );
    assert_eq!(
        reparsed.text_items[0].effects.as_ref().unwrap().font_bold,
        Some(true)
    );
    assert_eq!(
        reparsed.sheets[0].pins[0].effects.as_ref().unwrap().justify,
        vec!["right".to_string()]
    );
}
