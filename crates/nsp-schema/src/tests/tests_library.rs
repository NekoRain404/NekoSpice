//! Domain-focused tests for nsp-schema.

use super::assert_close;
use crate::{
    NspAt, NspColor, NspGraphic, NspSymbolBodyStyles, NspSymbolPlacement, NspSymbolPower,
    parse_schematic, parse_symbol_library, read_symbol_library, read_symbol_library_table,
};
use std::collections::BTreeMap;
use std::path::Path;

#[test]
fn parses_schema_symbol_library_fixture() {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap();
    let library =
        read_symbol_library(&workspace_root.join("examples/schema_schematic/neko_spice.nsp_sym"))
            .unwrap();

    let resistor = library.symbol("NekoSpice:R").unwrap();
    assert_eq!(resistor.property("Reference"), Some("R"));
    assert_eq!(resistor.graphics.len(), 1);
    assert_eq!(resistor.pins.len(), 2);
    assert_eq!(resistor.pins[0].number(), "1");
    assert_eq!(resistor.pins[0].electrical_type, "passive");
    let bounds = resistor.bounding_box().unwrap();
    assert_eq!(bounds.min.x, -2.54);
    assert_eq!(bounds.max.x, 2.54);
    assert!(bounds.width() > 5.0);
    assert!(library.to_summary_json().contains("\"symbol_count\": 3"));
    assert!(library.to_summary_json().contains("\"graphic_count\": 6"));
}

#[test]
fn roundtrips_schema_symbol_library_fixture_through_writer() {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap();
    let library =
        read_symbol_library(&workspace_root.join("examples/schema_schematic/neko_spice.nsp_sym"))
            .unwrap();

    let exported = library.to_symbol_library_sexpr();
    assert!(exported.contains("(nsp_symbol_lib"));
    assert!(exported.contains("(symbol \"NekoSpice:R\""));
    let reparsed = parse_symbol_library(&exported, "roundtrip.nsp_sym").unwrap();

    assert_eq!(reparsed.symbols.len(), library.symbols.len());
    assert_eq!(
        reparsed
            .symbols
            .iter()
            .map(|symbol| symbol.graphics.len())
            .sum::<usize>(),
        6
    );
    let resistor = reparsed.symbol("NekoSpice:R").unwrap();
    assert_eq!(resistor.pins.len(), 2);
    assert_eq!(resistor.property("Reference"), Some("R"));
    assert_eq!(resistor.graphics.len(), 1);
    let bounds = resistor.bounding_box().unwrap();
    assert_close(bounds.min.x, -2.54);
    assert_close(bounds.max.x, 2.54);
}

#[test]
fn preserves_schema_symbol_library_file_metadata() {
    let library = parse_symbol_library(
        r#"(nsp_symbol_lib
  (version 20230121)
  (generator "nsp_symbol_editor")
  (generator_version "9.0")
  (symbol "NekoSpice:Fonted"
    (embedded_fonts no)
    (property "Reference" "U" (at 0 0 0))
    (property "Value" "Fonted" (at 0 -2.54 0))
  )
  (symbol "NekoSpice:Embedded"
    (embedded_fonts yes)
    (property "Reference" "U" (at 0 0 0))
    (property "Value" "Embedded" (at 0 -2.54 0))
  )
)"#,
        "metadata.nsp_sym",
    )
    .unwrap();

    assert_eq!(library.generator_version.as_deref(), Some("9.0"));
    assert_eq!(
        library.symbol("NekoSpice:Fonted").unwrap().embedded_fonts,
        Some(false)
    );
    assert_eq!(
        library.symbol("NekoSpice:Embedded").unwrap().embedded_fonts,
        Some(true)
    );
    let summary = library.to_summary_json();
    assert!(summary.contains("\"generator_version\": \"9.0\""));
    assert!(summary.contains("\"embedded_font_symbol_count\": 2"));

    let exported = library.to_symbol_library_sexpr();
    assert!(exported.contains("(generator_version \"9.0\")"));
    assert!(exported.contains("(embedded_fonts no)"));
    assert!(exported.contains("(embedded_fonts yes)"));

    let reparsed = parse_symbol_library(&exported, "metadata_roundtrip.nsp_sym").unwrap();
    assert_eq!(reparsed.generator_version.as_deref(), Some("9.0"));
    assert_eq!(
        reparsed.symbol("NekoSpice:Fonted").unwrap().embedded_fonts,
        Some(false)
    );
}

#[test]
fn parses_schema_symbol_library_bezier_graphics_and_roundtrips() {
    let library = parse_symbol_library(
        r#"(nsp_symbol_lib
  (version 20230121)
  (generator "NekoSpice")
  (symbol "NekoSpice:Curve"
    (property "Reference" "U" (at 0 0 0))
    (property "Value" "Curve" (at 0 -2.54 0))
    (symbol "Curve_0_1"
      (bezier
        (pts (xy -2.54 0) (xy -1.27 -2.54) (xy 1.27 2.54) (xy 2.54 0))
        (stroke (width 0.254) (type default))
        (fill (type none))
      )
    )
  )
)"#,
        "curve.nsp_sym",
    )
    .unwrap();

    let symbol = library.symbol("NekoSpice:Curve").unwrap();
    assert_eq!(symbol.graphics.len(), 1);
    if let NspGraphic::Bezier { points } = &symbol.graphics[0].graphic {
        assert_eq!(points.len(), 4);
        assert_close(points[0].x, -2.54);
        assert_close(points[3].x, 2.54);
    } else {
        panic!("expected bezier symbol graphic");
    }
    let bounds = symbol.bounding_box().unwrap();
    assert_close(bounds.min.x, -2.54);
    assert_close(bounds.max.y, 2.54);

    let exported = library.to_symbol_library_sexpr();
    assert!(exported.contains("(bezier"));
    assert!(exported.contains("(pts (xy -2.54 0) (xy -1.27 -2.54) (xy 1.27 2.54) (xy 2.54 0))"));
    let reparsed = parse_symbol_library(&exported, "curve_roundtrip.nsp_sym").unwrap();
    let reparsed_symbol = reparsed.symbol("NekoSpice:Curve").unwrap();
    assert!(matches!(
        &reparsed_symbol.graphics[0].graphic,
        NspGraphic::Bezier { points } if points.len() == 4
    ));
}

#[test]
fn preserves_schema_symbol_pin_display_and_text_effects() {
    let library = parse_symbol_library(
        r#"(nsp_symbol_lib
  (version 20230121)
  (generator "NekoSpice")
  (symbol "NekoSpice:StyledPin"
    (pin_numbers
      (hide yes)
    )
    (pin_names
      (offset 2.54)
      (hide no)
    )
    (property "Reference" "U" (at 0 0 0))
    (property "Value" "StyledPin" (at 0 -2.54 0))
    (symbol "StyledPin_0_1"
      (pin input clock
        (at -5.08 0 0)
        (length 5.08)
        (name "CLK"
          (effects
            (font (size 1.524 1.016) (thickness 0.1524) bold italic (color 58 104 255 0.5))
            (justify left bottom)
            (hide yes)
          )
        )
        (number "1"
          (effects
            (font (size 1.27 1.27) (color 255 89 101 1))
            (justify right)
          )
        )
      )
    )
  )
)"#,
        "styled_pin.nsp_sym",
    )
    .unwrap();

    let symbol = library.symbol("NekoSpice:StyledPin").unwrap();
    assert_eq!(symbol.pin_numbers.as_ref().unwrap().hide, Some(true));
    assert_close(symbol.pin_names.as_ref().unwrap().offset.unwrap(), 2.54);
    assert_eq!(symbol.pin_names.as_ref().unwrap().hide, Some(false));
    assert_eq!(symbol.pins.len(), 1);
    let pin = &symbol.pins[0];
    assert_eq!(pin.number(), "1");
    assert_eq!(pin.name(), "CLK");
    assert_eq!(pin.electrical_type, "input");
    assert_eq!(pin.shape, "clock");
    assert_close(pin.name_effects().unwrap().font_size.unwrap().width, 1.524);
    assert_close(pin.name_effects().unwrap().font_size.unwrap().height, 1.016);
    assert_eq!(pin.name_effects().unwrap().font_bold, Some(true));
    assert_eq!(pin.name_effects().unwrap().font_italic, Some(true));
    assert!(pin.name_effects().unwrap().hide);
    assert_eq!(
        pin.number_effects().unwrap().font_color,
        Some(NspColor {
            red: 255.0,
            green: 89.0,
            blue: 101.0,
            alpha: 1.0,
        })
    );
    let summary = library.to_summary_json();
    assert!(summary.contains("\"pin_count\": 1"));
    assert!(summary.contains("\"pin_display_setting_count\": 2"));
    assert!(summary.contains("\"pin_text_effect_count\": 2"));

    let schematic = parse_schematic(
        r#"(nsp_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols
    (symbol "NekoSpice:StyledPin"
      (pin_numbers (hide yes))
      (pin_names (offset 2.54) (hide no))
      (property "Reference" "U" (at 0 0 0))
      (property "Value" "StyledPin" (at 0 -2.54 0))
      (symbol "StyledPin_0_1"
        (pin input clock
          (at -5.08 0 0)
          (length 5.08)
          (name "CLK" (effects (font (size 1.524 1.016) bold italic) (hide yes)))
          (number "1" (effects (font (size 1.27 1.27) (color 255 89 101 1))))
        )
      )
    )
  )
  (symbol
    (lib_id "NekoSpice:StyledPin")
    (at 10 10 0)
    (property "Reference" "U1" (at 10 7 0))
    (property "Value" "StyledPin" (at 10 13 0))
  )
)"#,
        "styled_pin_canvas.nsp_sch",
    )
    .unwrap();
    let scene = schematic.canvas_scene();
    assert_eq!(scene.symbols.len(), 1);
    assert_eq!(
        scene.symbols[0].pin_numbers.as_ref().unwrap().hide,
        Some(true)
    );
    assert_close(
        scene.symbols[0].pin_names.as_ref().unwrap().offset.unwrap(),
        2.54,
    );
    assert!(scene.symbols[0].pins[0].name_effects.as_ref().unwrap().hide);
    assert_eq!(
        scene.symbols[0].pins[0]
            .number_effects
            .as_ref()
            .unwrap()
            .font_color,
        Some(NspColor {
            red: 255.0,
            green: 89.0,
            blue: 101.0,
            alpha: 1.0,
        })
    );

    let exported = library.to_symbol_library_sexpr();
    assert!(exported.contains("(pin_numbers"));
    assert!(exported.contains("(hide yes)"));
    assert!(exported.contains("(pin_names"));
    assert!(exported.contains("(offset 2.54)"));
    assert!(exported.contains("(hide no)"));
    assert!(exported.contains("(font (size 1.524 1.016) (thickness 0.1524) (bold yes) (italic yes) (color 58 104 255 0.5))"));
    assert!(exported.contains("(justify left bottom)"));
    assert!(exported.contains("(font (size 1.27 1.27) (color 255 89 101 1))"));

    let reparsed = parse_symbol_library(&exported, "styled_pin_roundtrip.nsp_sym").unwrap();
    let reparsed_symbol = reparsed.symbol("NekoSpice:StyledPin").unwrap();
    assert_eq!(
        reparsed_symbol.pin_numbers.as_ref().unwrap().hide,
        Some(true)
    );
    assert_close(
        reparsed_symbol.pin_names.as_ref().unwrap().offset.unwrap(),
        2.54,
    );
    assert_eq!(
        reparsed_symbol.pins[0].name_effects().unwrap().font_bold,
        Some(true)
    );
    assert_eq!(
        reparsed_symbol.pins[0].number_effects().unwrap().justify,
        vec!["right"]
    );
}

#[test]
fn preserves_schema_symbol_pin_alternates_and_canvas_metadata() {
    let library = parse_symbol_library(
        r#"(nsp_symbol_lib
  (version 20230121)
  (generator "NekoSpice")
  (symbol "NekoSpice:AltPin"
    (property "Reference" "U" (at 0 0 0))
    (property "Value" "AltPin" (at 0 -2.54 0))
    (symbol "AltPin_0_1"
      (pin input line
        (at -5.08 0 0)
        (length 5.08)
        (name "SDI" (effects (font (size 1.27 1.27))))
        (number "6" (effects (font (size 1.27 1.27))))
        (alternate "SDA" bidirectional line)
        (alternate "SDO" output clock)
      )
    )
  )
)"#,
        "alt_pin.nsp_sym",
    )
    .unwrap();

    let symbol = library.symbol("NekoSpice:AltPin").unwrap();
    assert_eq!(symbol.pins.len(), 1);
    assert_eq!(symbol.pins[0].alternates.len(), 2);
    assert_eq!(symbol.pins[0].alternates[0].name, "SDA");
    assert_eq!(
        symbol.pins[0].alternates[0].electrical_type,
        "bidirectional"
    );
    assert_eq!(symbol.pins[0].alternates[1].name, "SDO");
    assert_eq!(symbol.pins[0].alternates[1].shape, "clock");
    assert!(
        library
            .to_summary_json()
            .contains("\"pin_alternate_count\": 2")
    );

    let schematic = parse_schematic(
        r#"(nsp_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols
    (symbol "NekoSpice:AltPin"
      (property "Reference" "U" (at 0 0 0))
      (property "Value" "AltPin" (at 0 -2.54 0))
      (symbol "AltPin_0_1"
        (pin input line
          (at -5.08 0 0)
          (length 5.08)
          (name "SDI" (effects (font (size 1.27 1.27))))
          (number "6" (effects (font (size 1.27 1.27))))
          (alternate "SDA" bidirectional line)
          (alternate "SDO" output clock)
        )
      )
    )
  )
  (symbol
    (lib_id "NekoSpice:AltPin")
    (at 10 10 0)
    (property "Reference" "U1" (at 10 7 0))
    (property "Value" "AltPin" (at 10 13 0))
  )
)"#,
        "alt_pin_canvas.nsp_sch",
    )
    .unwrap();
    let scene = schematic.canvas_scene();
    assert_eq!(scene.symbols[0].pins[0].alternates.len(), 2);
    assert_eq!(scene.symbols[0].pins[0].alternates[0].name, "SDA");
    assert_eq!(
        scene.symbols[0].pins[0].alternates[1].electrical_type,
        "output"
    );

    let exported = library.to_symbol_library_sexpr();
    assert!(exported.contains("(alternate \"SDA\" bidirectional line)"));
    assert!(exported.contains("(alternate \"SDO\" output clock)"));

    let reparsed = parse_symbol_library(&exported, "alt_pin_roundtrip.nsp_sym").unwrap();
    assert_eq!(
        reparsed.symbol("NekoSpice:AltPin").unwrap().pins[0].alternates,
        symbol.pins[0].alternates
    );
}

#[test]
fn preserves_schema_symbol_definition_flags_and_roundtrips() {
    let library = parse_symbol_library(
        r##"(nsp_symbol_lib
  (version 20230121)
  (generator "NekoSpice")
  (symbol "NekoSpice:PowerBare"
    (power)
    (exclude_from_sim no)
    (in_bom no)
    (on_board yes)
    (in_pos_files no)
    (duplicate_pin_numbers_are_jumpers yes)
    (property "Reference" "#PWR" (at 0 0 0))
    (property "Value" "PowerBare" (at 0 -2.54 0))
    (symbol "PowerBare_0_1"
      (pin power_in line (at 0 0 0) (length 0) (name "VCC") (number "1"))
    )
  )
  (symbol "NekoSpice:PowerGlobal"
    (power global)
    (in_bom yes)
    (on_board no)
    (in_pos_files yes)
    (property "Reference" "#PWR" (at 0 0 0))
    (property "Value" "PowerGlobal" (at 0 -2.54 0))
  )
  (symbol "NekoSpice:PowerLocal"
    (power local)
    (property "Reference" "#PWR" (at 0 0 0))
    (property "Value" "PowerLocal" (at 0 -2.54 0))
  )
)"##,
        "symbol_flags.nsp_sym",
    )
    .unwrap();

    let bare = library.symbol("NekoSpice:PowerBare").unwrap();
    assert_eq!(bare.power, Some(NspSymbolPower::Bare));
    assert_eq!(bare.exclude_from_sim, Some(false));
    assert_eq!(bare.in_bom, Some(false));
    assert_eq!(bare.on_board, Some(true));
    assert_eq!(bare.in_pos_files, Some(false));
    assert_eq!(bare.duplicate_pin_numbers_are_jumpers, Some(true));
    assert_eq!(
        library.symbol("NekoSpice:PowerGlobal").unwrap().power,
        Some(NspSymbolPower::Global)
    );
    assert_eq!(
        library.symbol("NekoSpice:PowerGlobal").unwrap().in_bom,
        Some(true)
    );
    assert_eq!(
        library.symbol("NekoSpice:PowerGlobal").unwrap().on_board,
        Some(false)
    );
    assert_eq!(
        library
            .symbol("NekoSpice:PowerGlobal")
            .unwrap()
            .in_pos_files,
        Some(true)
    );
    assert_eq!(
        library.symbol("NekoSpice:PowerLocal").unwrap().power,
        Some(NspSymbolPower::Local)
    );

    let summary = library.to_summary_json();
    assert!(summary.contains("\"power_symbol_count\": 3"));
    assert!(summary.contains("\"symbol_in_bom_setting_count\": 2"));
    assert!(summary.contains("\"symbol_on_board_setting_count\": 2"));
    assert!(summary.contains("\"symbol_in_pos_files_setting_count\": 2"));
    assert!(summary.contains("\"duplicate_pin_numbers_are_jumpers_count\": 1"));

    let exported = library.to_symbol_library_sexpr();
    assert!(exported.contains("(power)"));
    assert!(exported.contains("(power global)"));
    assert!(exported.contains("(power local)"));
    assert!(exported.contains("(exclude_from_sim no)"));
    assert!(exported.contains("(in_bom no)"));
    assert!(exported.contains("(in_bom yes)"));
    assert!(exported.contains("(on_board no)"));
    assert!(exported.contains("(on_board yes)"));
    assert!(exported.contains("(in_pos_files no)"));
    assert!(exported.contains("(in_pos_files yes)"));
    assert!(exported.contains("(duplicate_pin_numbers_are_jumpers yes)"));

    let reparsed = parse_symbol_library(&exported, "symbol_flags_roundtrip.nsp_sym").unwrap();
    assert_eq!(
        reparsed.symbol("NekoSpice:PowerBare").unwrap().power,
        Some(NspSymbolPower::Bare)
    );
    assert_eq!(
        reparsed
            .symbol("NekoSpice:PowerBare")
            .unwrap()
            .duplicate_pin_numbers_are_jumpers,
        Some(true)
    );
    assert_eq!(
        reparsed.symbol("NekoSpice:PowerGlobal").unwrap().power,
        Some(NspSymbolPower::Global)
    );
    assert_eq!(
        reparsed.symbol("NekoSpice:PowerLocal").unwrap().power,
        Some(NspSymbolPower::Local)
    );
}

#[test]
fn preserves_schema_symbol_inheritance_body_styles_and_jumpers() {
    let library = parse_symbol_library(
        r#"(nsp_symbol_lib
  (version 20230121)
  (generator "NekoSpice")
  (symbol "NekoSpice:Parent"
    (body_styles demorgan)
    (duplicate_pin_numbers_are_jumpers yes)
    (jumper_pin_groups
      ("A1" "A2")
      ("B1" "B2" "B3")
    )
    (property "Reference" "U" (at 0 0 0))
    (property "Value" "Parent" (at 0 -2.54 0))
  )
  (symbol "NekoSpice:Derived"
    (extends "NekoSpice:Parent")
    (body_styles "logic" "analog-front-end")
    (property "Reference" "U" (at 0 0 0))
    (property "Value" "Derived" (at 0 -2.54 0))
  )
)"#,
        "symbol_inheritance.nsp_sym",
    )
    .unwrap();

    let parent = library.symbol("NekoSpice:Parent").unwrap();
    assert_eq!(parent.body_styles, Some(NspSymbolBodyStyles::Demorgan));
    assert_eq!(parent.duplicate_pin_numbers_are_jumpers, Some(true));
    assert_eq!(
        parent.jumper_pin_groups,
        vec![
            vec!["A1".to_string(), "A2".to_string()],
            vec!["B1".to_string(), "B2".to_string(), "B3".to_string()]
        ]
    );

    let derived = library.symbol("NekoSpice:Derived").unwrap();
    assert_eq!(derived.extends.as_deref(), Some("NekoSpice:Parent"));
    assert_eq!(
        derived.body_styles,
        Some(NspSymbolBodyStyles::Names(vec![
            "logic".to_string(),
            "analog-front-end".to_string()
        ]))
    );

    let summary = library.to_summary_json();
    assert!(summary.contains("\"extended_symbol_count\": 1"));
    assert!(summary.contains("\"body_style_symbol_count\": 2"));
    assert!(summary.contains("\"jumper_pin_group_count\": 2"));

    let exported = library.to_symbol_library_sexpr();
    assert!(exported.contains("(body_styles demorgan)"));
    assert!(exported.contains("(duplicate_pin_numbers_are_jumpers yes)"));
    assert!(exported.contains("(jumper_pin_groups"));
    assert!(exported.contains("(\"A1\" \"A2\")"));
    assert!(exported.contains("(\"B1\" \"B2\" \"B3\")"));
    assert!(exported.contains("(extends \"NekoSpice:Parent\")"));
    assert!(exported.contains("(body_styles logic analog-front-end)"));

    let reparsed = parse_symbol_library(&exported, "symbol_inheritance_roundtrip.nsp_sym").unwrap();
    assert_eq!(
        reparsed
            .symbol("NekoSpice:Derived")
            .unwrap()
            .extends
            .as_deref(),
        Some("NekoSpice:Parent")
    );
    assert_eq!(
        reparsed
            .symbol("NekoSpice:Parent")
            .unwrap()
            .jumper_pin_groups
            .len(),
        2
    );
}

#[test]
fn resolves_schema_symbol_inheritance_for_canvas_netlist_and_placement() {
    let mut schematic = parse_schematic(
        r#"(nsp_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols
    (symbol "NekoSpice:ParentR"
      (pin_names (offset 0.508))
      (pin_numbers (hide yes))
      (property "Reference" "R" (at 0 0 0))
      (property "Value" "1k" (at 0 -2.54 0))
      (property "Sim.Device" "R" (at 0 0 0))
      (symbol "ParentR_0_1"
        (rectangle
          (start -1 -1)
          (end 1 1)
          (stroke (width 0.127) (type default))
          (fill (type none))
        )
        (pin passive line (at -2.54 0 0) (length 2.54) (name "~") (number "1"))
        (pin passive line (at 2.54 0 180) (length 2.54) (name "~") (number "2"))
      )
    )
    (symbol "NekoSpice:DerivedR"
      (extends "NekoSpice:ParentR")
      (pin_names (offset 1.016))
      (property "Reference" "R" (at 0 0 0))
      (property "Value" "4.7k" (at 0 -2.54 0))
    )
  )
  (wire (pts (xy 17.46 10) (xy 10 10)))
  (wire (pts (xy 22.54 10) (xy 30 10)))
  (label "in" (at 10 10 0))
  (label "0" (at 30 10 0))
  (text ".op" (at 5 5 0))
  (symbol
    (lib_id "NekoSpice:DerivedR")
    (at 20 10 0)
    (property "Reference" "R1" (at 20 8 0))
    (property "Value" "2.2k" (at 20 12 0))
  )
)"#,
        "derived_symbol.nsp_sch",
    )
    .unwrap();

    let scene = schematic.canvas_scene();
    let symbol = scene
        .symbols
        .iter()
        .find(|symbol| symbol.reference == "R1")
        .unwrap();
    assert_eq!(symbol.graphics.len(), 1);
    assert_eq!(symbol.pins.len(), 2);
    assert_close(symbol.pin_names.as_ref().unwrap().offset.unwrap(), 1.016);
    assert_eq!(symbol.pin_numbers.as_ref().unwrap().hide, Some(true));

    let netlist = schematic.to_spice_netlist().unwrap();
    assert!(netlist.contains("R1 in 0 2.2k"));

    let exported = schematic.to_schematic_sexpr();
    assert!(exported.contains("(extends \"NekoSpice:ParentR\")"));
    assert!(!exported.contains("(symbol \"DerivedR_0_1\""));

    let derived = schematic
        .symbol_definition("NekoSpice:DerivedR")
        .unwrap()
        .clone();
    schematic
        .place_symbol(NspSymbolPlacement {
            definition: derived,
            library_symbols: Vec::new(),
            reference: "R2".to_string(),
            value: "3.3k".to_string(),
            at: NspAt {
                x: 40.0,
                y: 10.0,
                rotation: 0.0,
            },
            unit: Some(1),
            body_style: None,
            pin_alternates: BTreeMap::new(),
            uuid: None,
        })
        .unwrap();
    let placed = schematic
        .symbols
        .iter()
        .find(|symbol| symbol.reference() == Some("R2"))
        .unwrap();
    assert_eq!(placed.pins.len(), 2);
}

#[test]
fn preserves_schema_symbol_graphic_styles_and_roundtrips() {
    let library = parse_symbol_library(
        r#"(nsp_symbol_lib
  (version 20230121)
  (generator "NekoSpice")
  (symbol "NekoSpice:Styled"
    (property "Reference" "U" (at 0 0 0))
    (property "Value" "Styled" (at 0 -2.54 0))
    (symbol "Styled_0_1"
      (polyline private
        (pts (xy -2.54 -1.27) (xy 0 1.27) (xy 2.54 -1.27))
        (stroke (width 0.0254) (type dash_dot) (color 58 104 255 0.5))
        (fill (type outline))
        (uuid "a5cd8da1-8f7f-4f80-bb23-0317de562222")
        (locked yes)
      )
      (rectangle
        (start -1 -1)
        (end 1 1)
        (stroke (width 0) (type default) (color 0 0 0 0))
        (fill (type background))
      )
      (text "ALT"
        (at 1.27 2.54 90)
        (effects
          (font (size 1.524 1.016) (thickness 0.1524) bold italic (color 255 89 101 0.75))
          (justify right bottom)
          (href "https://nekospice.test/symbol-text")
        )
      )
    )
  )
)"#,
        "styled_symbol.nsp_sym",
    )
    .unwrap();

    let symbol = library.symbol("NekoSpice:Styled").unwrap();
    assert_eq!(symbol.graphics.len(), 3);
    let styled = &symbol.graphics[0];
    assert!(styled.private);
    assert!(matches!(
        styled.graphic,
        NspGraphic::Polyline { ref points } if points.len() == 3
    ));
    assert_close(styled.stroke.as_ref().unwrap().width.unwrap(), 0.0254);
    assert_eq!(
        styled.stroke.as_ref().unwrap().stroke_type.as_deref(),
        Some("dash_dot")
    );
    assert_eq!(
        styled.stroke.as_ref().unwrap().color,
        Some(NspColor {
            red: 58.0,
            green: 104.0,
            blue: 255.0,
            alpha: 0.5,
        })
    );
    assert_eq!(
        styled.fill.as_ref().unwrap().fill_type.as_deref(),
        Some("outline")
    );
    assert_eq!(
        styled.uuid.as_deref(),
        Some("a5cd8da1-8f7f-4f80-bb23-0317de562222")
    );
    assert_eq!(styled.locked, Some(true));
    assert_eq!(
        symbol.graphics[1]
            .fill
            .as_ref()
            .unwrap()
            .fill_type
            .as_deref(),
        Some("background")
    );
    if let NspGraphic::Text { text, at, effects } = &symbol.graphics[2].graphic {
        assert_eq!(text, "ALT");
        assert_close(at.unwrap().x, 1.27);
        assert_close(at.unwrap().rotation, 90.0);
        let effects = effects.as_ref().unwrap();
        assert_close(effects.font_size.unwrap().width, 1.524);
        assert_close(effects.font_size.unwrap().height, 1.016);
        assert_close(effects.font_thickness.unwrap(), 0.1524);
        assert_eq!(effects.font_bold, Some(true));
        assert_eq!(effects.font_italic, Some(true));
        assert_eq!(
            effects.font_color,
            Some(NspColor {
                red: 255.0,
                green: 89.0,
                blue: 101.0,
                alpha: 0.75,
            })
        );
        assert_eq!(effects.justify, vec!["right", "bottom"]);
        assert_eq!(
            effects.href.as_deref(),
            Some("https://nekospice.test/symbol-text")
        );
    } else {
        panic!("expected styled text symbol graphic");
    }

    let schematic = parse_schematic(
        r#"(nsp_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols
    (symbol "NekoSpice:Styled"
      (property "Reference" "U" (at 0 0 0))
      (property "Value" "Styled" (at 0 -2.54 0))
      (symbol "Styled_0_1"
        (polyline private
          (pts (xy -2.54 -1.27) (xy 0 1.27) (xy 2.54 -1.27))
          (stroke (width 0.0254) (type dash_dot) (color 58 104 255 0.5))
          (fill (type outline))
        )
      )
    )
  )
  (symbol
    (lib_id "NekoSpice:Styled")
    (at 10 10 0)
    (property "Reference" "U1" (at 10 7 0))
    (property "Value" "Styled" (at 10 13 0))
  )
)"#,
        "styled_symbol_canvas.nsp_sch",
    )
    .unwrap();
    let scene = schematic.canvas_scene();
    assert_eq!(scene.symbols.len(), 1);
    assert!(matches!(
        &scene.symbols[0].graphics[0],
        crate::NspCanvasGraphic::Polyline {
            stroke: Some(stroke),
            fill: Some(fill),
            ..
        } if stroke.stroke_type.as_deref() == Some("dash_dot")
            && fill.fill_type.as_deref() == Some("outline")
    ));

    let exported = library.to_symbol_library_sexpr();
    assert!(
        library
            .to_summary_json()
            .contains("\"symbol_graphic_text_effect_count\": 1")
    );
    assert!(exported.contains("(polyline private"));
    assert!(exported.contains("(stroke (width 0.0254) (type dash_dot) (color 58 104 255 0.5))"));
    assert!(exported.contains("(fill (type outline))"));
    assert!(exported.contains("(uuid \"a5cd8da1-8f7f-4f80-bb23-0317de562222\")"));
    assert!(exported.contains("(locked yes)"));
    assert!(exported.contains("(fill (type background))"));
    assert!(exported.contains("(text \"ALT\" (at 1.27 2.54 90)"));
    assert!(
            exported.contains(
                "(effects (font (size 1.524 1.016) (thickness 0.1524) (bold yes) (italic yes) (color 255 89 101 0.75)) (justify right bottom) (href \"https://nekospice.test/symbol-text\"))"
            )
        );

    let reparsed = parse_symbol_library(&exported, "styled_symbol_roundtrip.nsp_sym").unwrap();
    let reparsed_symbol = reparsed.symbol("NekoSpice:Styled").unwrap();
    assert_eq!(reparsed_symbol.graphics.len(), 3);
    assert!(reparsed_symbol.graphics[0].private);
    assert_eq!(
        reparsed_symbol.graphics[0]
            .stroke
            .as_ref()
            .unwrap()
            .stroke_type
            .as_deref(),
        Some("dash_dot")
    );
    assert_eq!(
        reparsed_symbol.graphics[0]
            .fill
            .as_ref()
            .unwrap()
            .fill_type
            .as_deref(),
        Some("outline")
    );
    assert_eq!(reparsed_symbol.graphics[0].locked, Some(true));
    assert_eq!(
        reparsed_symbol.graphics[1]
            .fill
            .as_ref()
            .unwrap()
            .fill_type
            .as_deref(),
        Some("background")
    );
    assert!(matches!(
        &reparsed_symbol.graphics[2].graphic,
        NspGraphic::Text { effects: Some(effects), .. }
            if effects.font_italic == Some(true)
                && effects.justify == vec!["right".to_string(), "bottom".to_string()]
                && effects.href.as_deref() == Some("https://nekospice.test/symbol-text")
    ));
}

#[test]
fn parses_schema_symbol_library_table_fixture() {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap();
    let table =
        read_symbol_library_table(&workspace_root.join("examples/schema_schematic/sym-lib-table"))
            .unwrap();

    assert_eq!(table.version.as_deref(), Some("7"));
    assert_eq!(table.libraries.len(), 1);
    assert_eq!(table.libraries[0].name, "NekoSpice");
    assert_eq!(table.libraries[0].library_type, "NekoSpice");
    assert_eq!(
        table.libraries[0].description.as_deref(),
        Some("NekoSpice analog simulation symbols")
    );
    assert_eq!(table.enabled_schema_libraries().count(), 1);
    assert!(table.to_summary_json().contains("\"library_count\": 1"));
}
