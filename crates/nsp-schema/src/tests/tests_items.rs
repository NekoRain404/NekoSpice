//! Domain-focused tests for nsp-schema.

use super::assert_close;
use crate::{NspColor, NspGraphic, NspPoint, parse_schematic};

#[test]
fn parses_schematic_junction_styles_and_roundtrips() {
    let schematic = parse_schematic(
        r#"(nsp_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols)
  (junction
    (at 58.42 19.05)
    (diameter 0.8128)
    (color 255 0 239 1)
    (uuid "8fabedd0-c306-4e64-a286-1d33eb9a2adf")
  )
)"#,
        "junction.nsp_sch",
    )
    .unwrap();

    assert_eq!(schematic.junctions.len(), 1);
    assert_close(schematic.junctions[0].diameter.unwrap(), 0.8128);
    assert_eq!(
        schematic.junctions[0].color,
        Some(NspColor {
            red: 255.0,
            green: 0.0,
            blue: 239.0,
            alpha: 1.0,
        })
    );
    assert!(
        schematic
            .to_summary_json()
            .contains("\"styled_junction_count\": 1")
    );

    let scene = schematic.canvas_scene();
    assert_eq!(scene.junctions.len(), 1);
    assert_close(scene.junctions[0].diameter.unwrap(), 0.8128);
    assert_eq!(
        scene.junctions[0].color,
        Some(NspColor {
            red: 255.0,
            green: 0.0,
            blue: 239.0,
            alpha: 1.0,
        })
    );

    let roundtrip = schematic.to_schematic_sexpr();
    assert!(roundtrip.contains("(junction"));
    assert!(roundtrip.contains("(diameter 0.8128)"));
    assert!(roundtrip.contains("(color 255 0 239 1)"));
    let reparsed = parse_schematic(&roundtrip, "junction_roundtrip.nsp_sch").unwrap();
    assert_eq!(reparsed.junctions.len(), 1);
    assert_close(reparsed.junctions[0].diameter.unwrap(), 0.8128);
    assert_eq!(
        reparsed.junctions[0].uuid.as_deref(),
        Some("8fabedd0-c306-4e64-a286-1d33eb9a2adf")
    );
}

#[test]
fn parses_schema_bus_items_and_roundtrips() {
    let schematic = parse_schematic(
        r#"(nsp_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols)
  (bus_alias "DATA" (members "D0" "D1" "D2" "D3"))
  (bus_entry
    (at 30 10)
    (size 2.54 -2.54)
    (stroke (width 0.127) (type dot) (color 255 89 101 1))
    (uuid "31313131-3131-4131-8131-313131313131")
  )
  (bus
    (pts (xy 30 10) (xy 30 30) (xy 60 30))
    (stroke (width 0.254) (type dash) (color 58 104 255 1))
    (uuid "32323232-3232-4232-8232-323232323232")
  )
  (wire
    (pts (xy 60 30) (xy 70 30))
    (stroke (width 0.1778) (type dash_dot) (color 255 176 0 1))
    (uuid "33333333-3333-4333-8333-333333333333")
  )
)"#,
        "bus.nsp_sch",
    )
    .unwrap();

    assert_eq!(schematic.bus_aliases.len(), 1);
    assert_eq!(schematic.bus_aliases[0].name, "DATA");
    assert_eq!(
        schematic.bus_aliases[0].members,
        vec![
            "D0".to_string(),
            "D1".to_string(),
            "D2".to_string(),
            "D3".to_string()
        ]
    );
    assert_eq!(schematic.buses.len(), 1);
    assert_eq!(schematic.bus_entries.len(), 1);
    assert_eq!(schematic.wires.len(), 1);
    assert_eq!(
        schematic.bus_entries[0]
            .stroke
            .as_ref()
            .unwrap()
            .stroke_type
            .as_deref(),
        Some("dot")
    );
    assert_close(
        schematic.buses[0].stroke.as_ref().unwrap().width.unwrap(),
        0.254,
    );
    assert_eq!(
        schematic.wires[0].stroke.as_ref().unwrap().color,
        Some(NspColor {
            red: 255.0,
            green: 176.0,
            blue: 0.0,
            alpha: 1.0,
        })
    );
    assert_close(schematic.bus_entries[0].end().x, 32.54);
    assert_close(schematic.bus_entries[0].end().y, 7.46);
    assert!(
        schematic
            .to_summary_json()
            .contains("\"bus_alias_count\": 1")
    );
    assert!(schematic.to_summary_json().contains("\"bus_count\": 1"));
    assert!(
        schematic
            .to_summary_json()
            .contains("\"bus_entry_count\": 1")
    );
    assert!(
        schematic
            .to_summary_json()
            .contains("\"styled_wire_count\": 1")
    );
    assert!(
        schematic
            .to_summary_json()
            .contains("\"styled_bus_count\": 1")
    );
    assert!(
        schematic
            .to_summary_json()
            .contains("\"styled_bus_entry_count\": 1")
    );

    let scene = schematic.canvas_scene();
    assert_eq!(scene.wires.len(), 1);
    assert_eq!(scene.buses.len(), 1);
    assert_eq!(scene.bus_entries.len(), 1);
    assert_eq!(
        scene.wires[0]
            .stroke
            .as_ref()
            .unwrap()
            .stroke_type
            .as_deref(),
        Some("dash_dot")
    );
    assert!(scene.to_summary_json().contains("\"bus_count\": 1"));
    assert!(scene.to_summary_json().contains("\"bus_entry_count\": 1"));

    let roundtrip = schematic.to_schematic_sexpr();
    assert!(roundtrip.contains("(bus_alias \"DATA\" (members \"D0\" \"D1\" \"D2\" \"D3\"))"));
    assert!(roundtrip.contains("(bus"));
    assert!(roundtrip.contains("(bus_entry"));
    assert!(roundtrip.contains("(stroke (width 0.127) (type dot) (color 255 89 101 1))"));
    assert!(roundtrip.contains("(stroke (width 0.254) (type dash) (color 58 104 255 1))"));
    assert!(roundtrip.contains("(stroke (width 0.1778) (type dash_dot) (color 255 176 0 1))"));
    assert!(roundtrip.contains("(uuid \"31313131-3131-4131-8131-313131313131\")"));
    assert!(roundtrip.contains("(uuid \"32323232-3232-4232-8232-323232323232\")"));
    let reparsed = parse_schematic(&roundtrip, "bus_roundtrip.nsp_sch").unwrap();
    assert_eq!(reparsed.bus_aliases.len(), 1);
    assert_eq!(reparsed.buses.len(), 1);
    assert_eq!(reparsed.bus_entries.len(), 1);
    assert_eq!(reparsed.wires.len(), 1);
    assert_eq!(
        reparsed.buses[0]
            .stroke
            .as_ref()
            .unwrap()
            .stroke_type
            .as_deref(),
        Some("dash")
    );
    assert_eq!(
        reparsed.bus_entries[0].uuid.as_deref(),
        Some("31313131-3131-4131-8131-313131313131")
    );
    assert_eq!(
        reparsed.buses[0].uuid.as_deref(),
        Some("32323232-3232-4232-8232-323232323232")
    );
}

#[test]
fn parses_net_chains_and_roundtrips() {
    let schematic = parse_schematic(
        r#"(nsp_sch
  (version 20251028)
  (generator "eeschema")
  (paper "A4")
  (lib_symbols)
  (net_chain "Signal1"
    (from "U1" "A1")
    (to "J1" "2")
    (net_class "USB3")
    (color 58 104 255 0.75)
    (nets "SS_TX+" "SS_TX-")
    (uuid "605e5401-cbcc-4f20-9148-b7b3bd8eecbe")
    (uuid "a878e86a-9b21-4559-9e74-a7a0e383034e")
  )
)"#,
        "net_chain.nsp_sch",
    )
    .unwrap();

    assert_eq!(schematic.net_chains.len(), 1);
    let net_chain = &schematic.net_chains[0];
    assert_eq!(net_chain.name, "Signal1");
    assert_eq!(net_chain.from.as_ref().unwrap().reference, "U1");
    assert_eq!(net_chain.from.as_ref().unwrap().pin, "A1");
    assert_eq!(net_chain.to.as_ref().unwrap().reference, "J1");
    assert_eq!(net_chain.to.as_ref().unwrap().pin, "2");
    assert_eq!(net_chain.net_class.as_deref(), Some("USB3"));
    assert_eq!(
        net_chain.color,
        Some(NspColor {
            red: 58.0,
            green: 104.0,
            blue: 255.0,
            alpha: 0.75,
        })
    );
    assert_eq!(
        net_chain.member_nets,
        vec!["SS_TX+".to_string(), "SS_TX-".to_string()]
    );
    assert_eq!(net_chain.extra.len(), 2);
    assert!(
        schematic
            .to_summary_json()
            .contains("\"net_chain_count\": 1")
    );
    assert!(
        schematic
            .to_summary_json()
            .contains("\"net_chain_member_net_count\": 2")
    );

    let roundtrip = schematic.to_schematic_sexpr();
    assert!(roundtrip.contains("(net_chain \"Signal1\""));
    assert!(roundtrip.contains("(from \"U1\" \"A1\")"));
    assert!(roundtrip.contains("(to \"J1\" \"2\")"));
    assert!(roundtrip.contains("(net_class \"USB3\")"));
    assert!(roundtrip.contains("(color 58 104 255 0.75)"));
    assert!(roundtrip.contains("(nets \"SS_TX+\" \"SS_TX-\")"));
    assert!(roundtrip.contains("(uuid \"605e5401-cbcc-4f20-9148-b7b3bd8eecbe\")"));
    assert!(roundtrip.contains("(uuid \"a878e86a-9b21-4559-9e74-a7a0e383034e\")"));

    let reparsed = parse_schematic(&roundtrip, "net_chain_roundtrip.nsp_sch").unwrap();
    assert_eq!(reparsed.net_chains.len(), 1);
    assert_eq!(reparsed.net_chains[0].member_nets.len(), 2);
    assert_eq!(reparsed.net_chains[0].extra.len(), 2);
    assert_eq!(reparsed.net_chains[0].net_class.as_deref(), Some("USB3"));
}

#[test]
fn parses_schematic_graphics_and_roundtrips() {
    let schematic = parse_schematic(
        r#"(nsp_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols)
  (polyline
    (pts (xy 10 10) (xy 20 10) (xy 20 15))
    (stroke (width 0.3556) (type dot) (color 255 89 101 1))
    (uuid "41414141-4141-4141-8141-414141414141")
  )
  (bezier
    (pts (xy 12 16) (xy 16 8) (xy 24 8) (xy 28 16))
    (stroke (width 0.2032) (type dash) (color 58 104 255 1))
    (fill (type none))
    (uuid "45454545-4545-4545-8545-454545454545")
  )
  (rectangle
    (start 30 10)
    (end 45 20)
    (stroke (width 0) (type default))
    (fill (type hatch) (color 255 64 87 1))
    (uuid "42424242-4242-4242-8242-424242424242")
    (locked yes)
  )
  (circle
    (center 60 15)
    (radius 5)
    (stroke (width 0) (type default))
    (fill (type none))
    (uuid "43434343-4343-4343-8343-434343434343")
  )
  (arc
    (start 70 20)
    (mid 75 10)
    (end 80 20)
    (stroke (width 0) (type default))
    (fill (type none))
    (uuid "44444444-4444-4444-8444-444444444444")
  )
)"#,
        "graphics.nsp_sch",
    )
    .unwrap();

    assert_eq!(schematic.graphics.len(), 5);
    assert!(matches!(
        &schematic.graphics[0].graphic,
        NspGraphic::Polyline { .. }
    ));
    assert!(matches!(
        &schematic.graphics[1].graphic,
        NspGraphic::Bezier { .. }
    ));
    assert!(matches!(
        &schematic.graphics[2].graphic,
        NspGraphic::Rectangle { .. }
    ));
    assert!(matches!(
        &schematic.graphics[3].graphic,
        NspGraphic::Circle { .. }
    ));
    assert!(matches!(
        &schematic.graphics[4].graphic,
        NspGraphic::Arc { .. }
    ));
    assert_eq!(
        schematic.graphics[0].uuid.as_deref(),
        Some("41414141-4141-4141-8141-414141414141")
    );
    assert_close(
        schematic.graphics[0]
            .stroke
            .as_ref()
            .unwrap()
            .width
            .unwrap(),
        0.3556,
    );
    assert_eq!(
        schematic.graphics[0]
            .stroke
            .as_ref()
            .unwrap()
            .stroke_type
            .as_deref(),
        Some("dot")
    );
    assert_eq!(
        schematic.graphics[0].stroke.as_ref().unwrap().color,
        Some(NspColor {
            red: 255.0,
            green: 89.0,
            blue: 101.0,
            alpha: 1.0,
        })
    );
    if let NspGraphic::Bezier { points } = &schematic.graphics[1].graphic {
        assert_eq!(points.len(), 4);
        assert_close(points[1].x, 16.0);
        assert_close(points[2].y, 8.0);
    } else {
        panic!("expected bezier schematic graphic");
    }
    assert_eq!(
        schematic.graphics[1]
            .stroke
            .as_ref()
            .unwrap()
            .stroke_type
            .as_deref(),
        Some("dash")
    );
    assert_eq!(
        schematic.graphics[2]
            .fill
            .as_ref()
            .unwrap()
            .fill_type
            .as_deref(),
        Some("hatch")
    );
    assert_eq!(schematic.graphics[2].locked, Some(true));
    assert!(
        schematic
            .to_summary_json()
            .contains("\"schematic_graphic_count\": 5")
    );
    assert!(
        schematic
            .to_summary_json()
            .contains("\"styled_schematic_graphic_count\": 5")
    );
    assert!(
        schematic
            .to_summary_json()
            .contains("\"locked_schematic_graphic_count\": 1")
    );

    let scene = schematic.canvas_scene();
    assert_eq!(scene.graphics.len(), 5);
    assert!(matches!(
        &scene.graphics[1],
        crate::NspCanvasGraphic::Bezier {
            points,
            stroke: Some(stroke),
            ..
        } if points.len() == 4 && stroke.stroke_type.as_deref() == Some("dash")
    ));
    assert!(matches!(
        &scene.graphics[2],
        crate::NspCanvasGraphic::Rectangle {
            fill: Some(fill),
            ..
        } if fill.fill_type.as_deref() == Some("hatch")
    ));
    assert!(scene.to_summary_json().contains("\"graphic_count\": 5"));
    assert!(
        scene
            .to_summary_json()
            .contains("\"schematic_graphic_count\": 5")
    );

    let roundtrip = schematic.to_schematic_sexpr();
    assert!(roundtrip.contains("(polyline"));
    assert!(roundtrip.contains("(stroke (width 0.3556) (type dot) (color 255 89 101 1))"));
    assert!(roundtrip.contains("(bezier"));
    assert!(roundtrip.contains("(pts (xy 12 16) (xy 16 8) (xy 24 8) (xy 28 16))"));
    assert!(roundtrip.contains("(stroke (width 0.2032) (type dash) (color 58 104 255 1))"));
    assert!(roundtrip.contains("(rectangle"));
    assert!(roundtrip.contains("(fill (type hatch) (color 255 64 87 1))"));
    assert!(roundtrip.contains("(locked yes)"));
    assert!(roundtrip.contains("(circle"));
    assert!(roundtrip.contains("(arc"));
    assert!(roundtrip.contains("(uuid \"44444444-4444-4444-8444-444444444444\")"));
    let reparsed = parse_schematic(&roundtrip, "graphics_roundtrip.nsp_sch").unwrap();
    assert_eq!(reparsed.graphics.len(), 5);
    assert_eq!(
        reparsed.graphics[1].uuid.as_deref(),
        Some("45454545-4545-4545-8545-454545454545")
    );
    assert!(matches!(
        &reparsed.graphics[1].graphic,
        NspGraphic::Bezier { points } if points.len() == 4
    ));
    assert_eq!(
        reparsed.graphics[4].uuid.as_deref(),
        Some("44444444-4444-4444-8444-444444444444")
    );
    assert_eq!(reparsed.graphics[2].locked, Some(true));
    assert_eq!(
        reparsed.graphics[2]
            .fill
            .as_ref()
            .unwrap()
            .fill_type
            .as_deref(),
        Some("hatch")
    );
    assert_eq!(reparsed.canvas_scene().graphics.len(), 5);
}

#[test]
fn parses_rule_areas_and_roundtrips() {
    let schematic = parse_schematic(
        r#"(nsp_sch
  (version 20251028)
  (generator "eeschema")
  (paper "A4")
  (lib_symbols)
  (rule_area
    (locked yes)
    (exclude_from_sim no)
    (in_bom no)
    (on_board no)
    (dnp yes)
    (polyline
      (pts
        (xy 120.65 30.48) (xy 100.33 30.48) (xy 100.33 53.34) (xy 104.14 57.15)
      )
      (stroke (width 0.127) (type dash) (color 10 20 30 1))
      (fill (type color) (color 20 200 170 0.25))
      (uuid "c41fc141-ff73-4a8e-9714-30fcb0d8076b")
    )
  )
)"#,
        "rule_area.nsp_sch",
    )
    .unwrap();

    assert_eq!(schematic.rule_areas.len(), 1);
    let rule_area = &schematic.rule_areas[0];
    assert_eq!(rule_area.points.len(), 4);
    assert_close(rule_area.stroke.as_ref().unwrap().width.unwrap(), 0.127);
    assert_eq!(
        rule_area.stroke.as_ref().unwrap().stroke_type.as_deref(),
        Some("dash")
    );
    assert_eq!(
        rule_area.fill.as_ref().unwrap().fill_type.as_deref(),
        Some("color")
    );
    assert_eq!(rule_area.locked, Some(true));
    assert_eq!(rule_area.exclude_from_sim, Some(false));
    assert_eq!(rule_area.in_bom, Some(false));
    assert_eq!(rule_area.on_board, Some(false));
    assert_eq!(rule_area.dnp, Some(true));
    assert!(
        schematic
            .to_summary_json()
            .contains("\"rule_area_count\": 1")
    );
    assert!(
        schematic
            .to_summary_json()
            .contains("\"styled_rule_area_count\": 1")
    );
    assert!(
        schematic
            .to_summary_json()
            .contains("\"locked_rule_area_count\": 1")
    );
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

    let scene = schematic.canvas_scene();
    assert_eq!(scene.rule_areas.len(), 1);
    assert_eq!(scene.rule_areas[0].points.len(), 4);
    assert!(scene.to_summary_json().contains("\"rule_area_count\": 1"));
    let scene_json: serde_json::Value = serde_json::from_str(&scene.to_json()).unwrap();
    assert_eq!(scene_json["rule_area_count"], 1);
    assert_eq!(scene_json["rule_areas"][0]["points"][0]["x"], 120.65);
    assert_eq!(scene_json["rule_areas"][0]["stroke"]["type"], "dash");
    assert_eq!(scene_json["rule_areas"][0]["fill"]["color"]["alpha"], 0.25);

    let roundtrip = schematic.to_schematic_sexpr();
    assert!(roundtrip.contains("(rule_area"));
    assert!(roundtrip.contains("(locked yes)"));
    assert!(roundtrip.contains("(exclude_from_sim no)"));
    assert!(roundtrip.contains("(in_bom no)"));
    assert!(roundtrip.contains("(on_board no)"));
    assert!(roundtrip.contains("(dnp yes)"));
    assert!(roundtrip.contains("(stroke (width 0.127) (type dash) (color 10 20 30 1))"));
    assert!(roundtrip.contains("(fill (type color) (color 20 200 170 0.25))"));
    assert!(roundtrip.contains("(uuid \"c41fc141-ff73-4a8e-9714-30fcb0d8076b\")"));
    let reparsed = parse_schematic(&roundtrip, "rule_area_roundtrip.nsp_sch").unwrap();
    assert_eq!(reparsed.rule_areas.len(), 1);
    assert_eq!(
        reparsed.rule_areas[0].uuid.as_deref(),
        Some("c41fc141-ff73-4a8e-9714-30fcb0d8076b")
    );
    assert_eq!(
        reparsed.rule_areas[0]
            .stroke
            .as_ref()
            .unwrap()
            .stroke_type
            .as_deref(),
        Some("dash")
    );
}

#[test]
fn parses_schematic_text_boxes_and_roundtrips() {
    let schematic = parse_schematic(
        r#"(nsp_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols)
  (text_box "Bigger\nMultiline\nText"
    (exclude_from_sim no)
    (at 10 20 0)
    (size 17.78 12.7)
    (margins 0.9525 0.9525 0.9525 0.9525)
    (stroke (width 0.0508) (type dash_dot) (color 255 50 55 1))
    (fill (type color) (color 255 228 206 0.7490196078))
    (effects (font (size 1.27 1.27) (color 10 9 37 1)))
    (uuid "45454545-4545-4545-8545-454545454545")
    (locked)
  )
)"#,
        "text_box.nsp_sch",
    )
    .unwrap();

    assert_eq!(schematic.text_boxes.len(), 1);
    assert_eq!(schematic.text_boxes[0].text, "Bigger\nMultiline\nText");
    assert_eq!(schematic.text_boxes[0].exclude_from_sim, Some(false));
    assert_close(schematic.text_boxes[0].size.unwrap().width, 17.78);
    assert_close(schematic.text_boxes[0].margins.unwrap().left, 0.9525);
    assert_close(
        schematic.text_boxes[0]
            .stroke
            .as_ref()
            .unwrap()
            .width
            .unwrap(),
        0.0508,
    );
    assert_eq!(
        schematic.text_boxes[0]
            .stroke
            .as_ref()
            .unwrap()
            .stroke_type
            .as_deref(),
        Some("dash_dot")
    );
    assert_eq!(
        schematic.text_boxes[0].stroke.as_ref().unwrap().color,
        Some(NspColor {
            red: 255.0,
            green: 50.0,
            blue: 55.0,
            alpha: 1.0,
        })
    );
    assert_eq!(
        schematic.text_boxes[0]
            .fill
            .as_ref()
            .unwrap()
            .fill_type
            .as_deref(),
        Some("color")
    );
    assert_eq!(schematic.text_boxes[0].locked, Some(true));
    assert_eq!(
        schematic.text_boxes[0].uuid.as_deref(),
        Some("45454545-4545-4545-8545-454545454545")
    );
    assert!(
        schematic
            .to_summary_json()
            .contains("\"text_box_count\": 1")
    );
    assert!(
        schematic
            .to_summary_json()
            .contains("\"styled_text_box_count\": 1")
    );
    assert!(
        schematic
            .to_summary_json()
            .contains("\"locked_text_box_count\": 1")
    );

    let scene = schematic.canvas_scene();
    assert_eq!(scene.text_boxes.len(), 1);
    assert_eq!(
        scene.text_boxes[0]
            .stroke
            .as_ref()
            .unwrap()
            .stroke_type
            .as_deref(),
        Some("dash_dot")
    );
    assert!(scene.bounds.unwrap().width() >= 17.78);
    assert!(scene.to_summary_json().contains("\"text_box_count\": 1"));
    let scene_json: serde_json::Value = serde_json::from_str(&scene.to_json()).unwrap();
    assert_eq!(scene_json["text_box_count"], 1);
    assert_eq!(
        scene_json["text_boxes"][0]["text"],
        "Bigger\nMultiline\nText"
    );
    assert_eq!(scene_json["text_boxes"][0]["margins"]["left"], 0.9525);
    assert_eq!(scene_json["text_boxes"][0]["stroke"]["type"], "dash_dot");
    assert_eq!(
        scene_json["text_boxes"][0]["effects"]["font_color"]["blue"],
        37.0
    );

    let roundtrip = schematic.to_schematic_sexpr();
    assert!(roundtrip.contains("(text_box \"Bigger\\nMultiline\\nText\""));
    assert!(roundtrip.contains("(size 17.78 12.7)"));
    assert!(roundtrip.contains("(margins 0.9525 0.9525 0.9525 0.9525)"));
    assert!(roundtrip.contains("(stroke (width 0.0508) (type dash_dot) (color 255 50 55 1))"));
    assert!(roundtrip.contains("(fill (type color) (color 255 228 206 0.7490196078))"));
    assert!(roundtrip.contains("(uuid \"45454545-4545-4545-8545-454545454545\")"));
    assert!(roundtrip.contains("(locked yes)"));
    let reparsed = parse_schematic(&roundtrip, "text_box_roundtrip.nsp_sch").unwrap();
    assert_eq!(reparsed.text_boxes.len(), 1);
    assert_eq!(reparsed.text_boxes[0].text, "Bigger\nMultiline\nText");
    assert_eq!(
        reparsed.text_boxes[0]
            .fill
            .as_ref()
            .unwrap()
            .fill_type
            .as_deref(),
        Some("color")
    );
    assert_eq!(reparsed.text_boxes[0].locked, Some(true));
    assert_eq!(reparsed.canvas_scene().text_boxes.len(), 1);
}

#[test]
fn hit_tests_rotated_text_boxes_by_shape() {
    let schematic = parse_schematic(
        r#"(nsp_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols)
  (text_box "Rotated note"
    (at 20 10 45)
    (size 10 4)
    (uuid "45454545-4545-4545-8545-454545454545")
  )
)"#,
        "rotated_text_box_hit.nsp_sch",
    )
    .unwrap();
    let scene = schematic.canvas_scene();
    let text_box = &scene.text_boxes[0];
    assert!(text_box.bounds.unwrap().width() > 9.0);
    assert!(text_box.bounds.unwrap().height() > 9.0);

    let hit = scene.hit_test(NspPoint { x: 22.12, y: 14.95 });
    assert!(hit.hits.iter().any(|hit| hit.kind == "text-box"
        && hit.uuid.as_deref() == Some("45454545-4545-4545-8545-454545454545")));

    let aabb_corner_miss = scene.hit_test(NspPoint { x: 26.8, y: 10.3 });
    assert!(
        !aabb_corner_miss
            .hits
            .iter()
            .any(|hit| hit.kind == "text-box"
                && hit.uuid.as_deref() == Some("45454545-4545-4545-8545-454545454545"))
    );
}

#[test]
fn parses_schematic_images_and_roundtrips() {
    let schematic = parse_schematic(
        r#"(nsp_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols)
  (image
    (at 36.83 39.37)
    (scale 1.5)
    (uuid "56565656-5656-4656-8656-565656565656")
    (data
      "iVBORw0KGgoAAAANSUhEUgAAADAAAAAwCAYAAABXAvmH"
    )
  )
)"#,
        "image.nsp_sch",
    )
    .unwrap();

    assert_eq!(schematic.images.len(), 1);
    assert_eq!(
        schematic.images[0].uuid.as_deref(),
        Some("56565656-5656-4656-8656-565656565656")
    );
    assert_close(schematic.images[0].scale, 1.5);
    assert_eq!(schematic.images[0].mime_type(), "image/png");
    assert_close(schematic.images[0].image_size_mm().unwrap().width, 6.096);
    assert!(schematic.to_summary_json().contains("\"image_count\": 1"));

    let scene = schematic.canvas_scene();
    assert_eq!(scene.images.len(), 1);
    assert_eq!(scene.images[0].mime_type, "image/png");
    assert_close(scene.images[0].image_size.unwrap().height, 6.096);
    let bounds = scene.bounds.unwrap();
    assert_close(bounds.width(), 6.096);
    assert_close(bounds.height(), 6.096);
    assert!(scene.to_summary_json().contains("\"image_count\": 1"));
    let scene_json: serde_json::Value = serde_json::from_str(&scene.to_json()).unwrap();
    assert_eq!(scene_json["image_count"], 1);
    assert_eq!(
        scene_json["images"][0]["uuid"],
        "56565656-5656-4656-8656-565656565656"
    );
    assert_eq!(scene_json["images"][0]["mime_type"], "image/png");
    assert_eq!(scene_json["images"][0]["scale"], 1.5);
    assert_close(
        scene_json["images"][0]["bounds"]["width"].as_f64().unwrap(),
        6.096,
    );
    assert_close(
        scene_json["images"][0]["bounds"]["height"]
            .as_f64()
            .unwrap(),
        6.096,
    );
    assert_eq!(
        scene_json["images"][0]["data_base64"],
        "iVBORw0KGgoAAAANSUhEUgAAADAAAAAwCAYAAABXAvmH"
    );

    let roundtrip = schematic.to_schematic_sexpr();
    assert!(roundtrip.contains("(image (at 36.83 39.37) (scale 1.5)"));
    assert!(roundtrip.contains("(data"));
    assert!(roundtrip.contains("iVBORw0KGgoAAAANSUhEUgAAADAAAAAwCAYAAABXAvmH"));
    assert!(roundtrip.contains("(uuid \"56565656-5656-4656-8656-565656565656\")"));
    let reparsed = parse_schematic(&roundtrip, "image_roundtrip.nsp_sch").unwrap();
    assert_eq!(reparsed.images.len(), 1);
    assert_eq!(reparsed.images[0].mime_type(), "image/png");
    assert_eq!(reparsed.canvas_scene().images.len(), 1);
}

#[test]
fn parses_schematic_tables_and_roundtrips() {
    let schematic = parse_schematic(
        r#"(nsp_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols)
  (table
    (column_count 2)
    (border (external yes) (header yes) (stroke (width 0.127) (type dash) (color 10 20 30 1)))
    (separators (rows yes) (cols no) (stroke (width 0.0508) (type dot) (color 40 50 60 0.5)))
    (column_widths 26.67 21.59)
    (row_heights 2.54 2.54)
    (uuid "67676767-6767-4767-8767-676767676767")
    (cells
      (table_cell "LED pin"
        (exclude_from_sim no)
        (at 122.555 29.21 0)
        (size 26.67 2.54)
        (margins 0.9525 0.9525 0.9525 0.9525)
        (span 1 1)
        (fill (type color) (color 255 228 206 0.5))
        (effects (font (size 1.27 1.27) (color 10 9 37 1)) (justify left top))
        (uuid "68686868-6868-4868-8868-686868686868")
      )
      (table_cell "Expected net"
        (exclude_from_sim no)
        (at 149.225 29.21 0)
        (size 21.59 2.54)
        (margins 0.9525 0.9525 0.9525 0.9525)
        (span 1 1)
        (fill (type none))
        (effects (font (size 1.27 1.27)) (justify left top))
        (uuid "69696969-6969-4969-8969-696969696969")
        (locked)
      )
    )
  )
)"#,
        "table.nsp_sch",
    )
    .unwrap();

    assert_eq!(schematic.tables.len(), 1);
    assert_eq!(schematic.tables[0].column_count, 2);
    assert_eq!(schematic.tables[0].cells.len(), 2);
    assert_eq!(schematic.tables[0].cells[0].text, "LED pin");
    assert_close(
        schematic.tables[0]
            .border
            .as_ref()
            .unwrap()
            .stroke
            .as_ref()
            .unwrap()
            .width
            .unwrap(),
        0.127,
    );
    assert_eq!(
        schematic.tables[0].separators.as_ref().unwrap().cols,
        Some(false)
    );
    assert_eq!(
        schematic.tables[0].cells[0]
            .fill
            .as_ref()
            .unwrap()
            .fill_type
            .as_deref(),
        Some("color")
    );
    assert_eq!(
        schematic.tables[0].cells[0]
            .effects
            .as_ref()
            .unwrap()
            .justify,
        vec!["left".to_string(), "top".to_string()]
    );
    assert_eq!(schematic.tables[0].cells[1].locked, Some(true));
    assert_close(schematic.tables[0].column_widths[0], 26.67);
    assert_close(schematic.tables[0].row_heights[0], 2.54);
    assert_eq!(
        schematic.tables[0].uuid.as_deref(),
        Some("67676767-6767-4767-8767-676767676767")
    );
    assert_eq!(
        schematic.tables[0].cells[0].uuid.as_deref(),
        Some("68686868-6868-4868-8868-686868686868")
    );
    assert!(schematic.to_summary_json().contains("\"table_count\": 1"));
    assert!(
        schematic
            .to_summary_json()
            .contains("\"table_cell_count\": 2")
    );
    assert!(
        schematic
            .to_summary_json()
            .contains("\"styled_table_count\": 1")
    );
    assert!(
        schematic
            .to_summary_json()
            .contains("\"styled_table_cell_count\": 2")
    );
    assert!(
        schematic
            .to_summary_json()
            .contains("\"locked_table_cell_count\": 1")
    );

    let scene = schematic.canvas_scene();
    assert_eq!(scene.tables.len(), 1);
    assert_eq!(scene.tables[0].cells.len(), 2);
    assert_eq!(
        scene.tables[0].cells[0]
            .fill
            .as_ref()
            .unwrap()
            .fill_type
            .as_deref(),
        Some("color")
    );
    assert!(scene.to_summary_json().contains("\"table_count\": 1"));
    assert!(scene.to_summary_json().contains("\"table_cell_count\": 2"));
    assert_close(scene.bounds.unwrap().width(), 48.26);
    let scene_json: serde_json::Value = serde_json::from_str(&scene.to_json()).unwrap();
    assert_eq!(scene_json["table_count"], 1);
    assert_eq!(scene_json["table_cell_count"], 2);
    assert_eq!(
        scene_json["tables"][0]["uuid"],
        "67676767-6767-4767-8767-676767676767"
    );
    assert_close(
        scene_json["tables"][0]["bounds"]["width"].as_f64().unwrap(),
        48.26,
    );
    assert_eq!(scene_json["tables"][0]["column_count"], 2);
    assert_eq!(scene_json["tables"][0]["cell_count"], 2);
    assert_eq!(
        scene_json["tables"][0]["cells"][0]["uuid"],
        "68686868-6868-4868-8868-686868686868"
    );
    assert_close(
        scene_json["tables"][0]["cells"][0]["bounds"]["width"]
            .as_f64()
            .unwrap(),
        26.67,
    );
    assert_eq!(scene_json["tables"][0]["cells"][0]["text"], "LED pin");
    assert_eq!(
        scene_json["tables"][0]["cells"][0]["effects"]["justify"][1],
        "top"
    );

    let roundtrip = schematic.to_schematic_sexpr();
    assert!(roundtrip.contains("(table"));
    assert!(roundtrip.contains("(column_count 2)"));
    assert!(roundtrip.contains(
        "(border (external yes) (header yes) (stroke (width 0.127) (type dash) (color 10 20 30 1)))"
    ));
    assert!(roundtrip.contains(
        "(separators (rows yes) (cols no) (stroke (width 0.0508) (type dot) (color 40 50 60 0.5)))"
    ));
    assert!(roundtrip.contains("(column_widths 26.67 21.59)"));
    assert!(roundtrip.contains("(fill (type color) (color 255 228 206 0.5))"));
    assert!(
        roundtrip
            .contains("(effects (font (size 1.27 1.27) (color 10 9 37 1)) (justify left top))")
    );
    assert!(roundtrip.contains("(locked yes)"));
    assert!(roundtrip.contains("(table_cell \"LED pin\""));
    assert!(roundtrip.contains("(uuid \"67676767-6767-4767-8767-676767676767\")"));
    assert!(roundtrip.contains("(uuid \"68686868-6868-4868-8868-686868686868\")"));
    let reparsed = parse_schematic(&roundtrip, "table_roundtrip.nsp_sch").unwrap();
    assert_eq!(reparsed.tables.len(), 1);
    assert_eq!(reparsed.tables[0].cells.len(), 2);
    assert_eq!(reparsed.tables[0].cells[1].locked, Some(true));
    assert_eq!(
        reparsed.tables[0].cells[0]
            .effects
            .as_ref()
            .unwrap()
            .font_color,
        Some(NspColor {
            red: 10.0,
            green: 9.0,
            blue: 37.0,
            alpha: 1.0,
        })
    );
    assert_eq!(reparsed.canvas_scene().tables.len(), 1);
}

#[test]
fn hit_tests_rotated_table_cells_by_shape() {
    let schematic = parse_schematic(
        r#"(nsp_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols)
  (table
    (column_count 1)
    (column_widths 10)
    (row_heights 4)
    (uuid "67676767-6767-4767-8767-676767676767")
    (cells
      (table_cell "Rotated cell"
        (at 40 10 45)
        (size 10 4)
        (uuid "68686868-6868-4868-8868-686868686868")
      )
    )
  )
)"#,
        "rotated_table_hit.nsp_sch",
    )
    .unwrap();
    let scene = schematic.canvas_scene();
    let cell = &scene.tables[0].cells[0];
    assert!(cell.bounds.unwrap().width() > 9.0);
    assert!(cell.bounds.unwrap().height() > 9.0);

    let hit = scene.hit_test(NspPoint { x: 42.12, y: 14.95 });
    assert!(hit.hits.iter().any(|hit| hit.kind == "table-cell"
        && hit.uuid.as_deref() == Some("68686868-6868-4868-8868-686868686868")));
    assert!(hit.hits.iter().any(|hit| hit.kind == "table"
        && hit.uuid.as_deref() == Some("67676767-6767-4767-8767-676767676767")));

    let aabb_corner_miss = scene.hit_test(NspPoint { x: 46.8, y: 10.3 });
    assert!(
        !aabb_corner_miss
            .hits
            .iter()
            .any(|hit| (hit.kind == "table-cell"
                && hit.uuid.as_deref() == Some("68686868-6868-4868-8868-686868686868"))
                || (hit.kind == "table"
                    && hit.uuid.as_deref() == Some("67676767-6767-4767-8767-676767676767")))
    );
}

#[test]
fn parses_schematic_groups_and_roundtrips() {
    let schematic = parse_schematic(
        r#"(nsp_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols)
  (wire (pts (xy 5 5) (xy 10 5)) (uuid "7e1da7e2-473f-48bf-b7bf-2eb79e1b1372"))
  (label "OUT" (at 10 5 0) (uuid "d26fc350-11e5-4917-ba78-4e25070d7aa8"))
  (group "GroupName"
    (uuid "7267eac2-0eb2-494a-bc81-61295bcdf08c")
    (locked yes)
    (members "7e1da7e2-473f-48bf-b7bf-2eb79e1b1372" "d26fc350-11e5-4917-ba78-4e25070d7aa8")
  )
)"#,
        "group.nsp_sch",
    )
    .unwrap();

    assert_eq!(schematic.groups.len(), 1);
    assert_eq!(schematic.groups[0].name, "GroupName");
    assert_eq!(
        schematic.groups[0].uuid.as_deref(),
        Some("7267eac2-0eb2-494a-bc81-61295bcdf08c")
    );
    assert_eq!(schematic.groups[0].locked, Some(true));
    assert_eq!(schematic.groups[0].members.len(), 2);
    assert_eq!(
        schematic.groups[0].members[0],
        "7e1da7e2-473f-48bf-b7bf-2eb79e1b1372"
    );
    assert!(schematic.to_summary_json().contains("\"group_count\": 1"));
    assert!(
        schematic
            .to_summary_json()
            .contains("\"group_member_count\": 2")
    );

    let scene = schematic.canvas_scene();
    assert_eq!(scene.wires.len(), 1);
    assert_eq!(scene.groups.len(), 1);
    assert_eq!(
        scene.groups[0].uuid.as_deref(),
        Some("7267eac2-0eb2-494a-bc81-61295bcdf08c")
    );
    assert_eq!(scene.groups[0].members.len(), 2);
    assert!(scene.to_summary_json().contains("\"wire_count\": 1"));
    assert!(scene.to_summary_json().contains("\"group_count\": 1"));
    let scene_json: serde_json::Value = serde_json::from_str(&scene.to_json()).unwrap();
    assert_eq!(scene_json["group_count"], 1);
    assert_eq!(scene_json["group_member_count"], 2);
    assert_eq!(
        scene_json["groups"][0]["uuid"],
        "7267eac2-0eb2-494a-bc81-61295bcdf08c"
    );
    assert_eq!(scene_json["groups"][0]["member_count"], 2);
    assert!(scene_json["groups"][0]["bounds"]["width"].as_f64().unwrap() > 5.0);

    let roundtrip = schematic.to_schematic_sexpr();
    assert!(roundtrip.contains("(group \"GroupName\""));
    assert!(roundtrip.contains("(uuid \"7267eac2-0eb2-494a-bc81-61295bcdf08c\")"));
    assert!(roundtrip.contains("(locked yes)"));
    assert!(roundtrip.contains(
            "(members \"7e1da7e2-473f-48bf-b7bf-2eb79e1b1372\" \"d26fc350-11e5-4917-ba78-4e25070d7aa8\")"
        ));
    let reparsed = parse_schematic(&roundtrip, "group_roundtrip.nsp_sch").unwrap();
    assert_eq!(reparsed.groups.len(), 1);
    assert_eq!(reparsed.groups[0].members.len(), 2);
    assert_eq!(reparsed.groups[0].locked, Some(true));
}
