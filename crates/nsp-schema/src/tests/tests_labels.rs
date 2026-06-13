//! Domain-focused tests for nsp-schema.

use super::assert_close;
use crate::{NspColor, NspLabelKind, parse_schematic};

#[test]
fn preserves_schema_directive_labels_and_roundtrips() {
    let schematic = parse_schematic(
        r#"(nsp_sch
  (version 20251028)
  (generator "eeschema")
  (paper "A4")
  (lib_symbols)
  (netclass_flag ""
    (length 3.81)
    (shape dot)
    (at 102.87 30.48 0)
    (fields_autoplaced yes)
    (effects
      (font
        (size 1.27 1.27)
        (color 236 104 255 1)
      )
      (justify left bottom)
    )
    (uuid "3c7ec402-4c06-4b52-9acd-ed760671ff85")
    (property "Net Class" "HV"
      (at 103.5685 27.94 0)
      (show_name no)
      (do_not_autoplace no)
      (effects (font (size 1.27 1.27)) (justify left))
    )
    (property "Component Class" "Classy"
      (at 99.822 24.892 0)
      (show_name no)
      (do_not_autoplace no)
      (effects (font (size 1.27 1.27) (italic yes)) (justify left))
    )
  )
  (netclass_flag ""
    (length 2.54)
    (shape dot)
    (at 110 30 0)
    (property "Net Class" "" (at 110 28 0))
    (property "Component Class" "OnlyComponent" (at 110 26 0))
  )
)"#,
        "directive_label.nsp_sch",
    )
    .unwrap();

    assert_eq!(schematic.directive_labels.len(), 2);
    let label = &schematic.directive_labels[0];
    assert_eq!(label.display_text(), "HV");
    assert_eq!(
        schematic.directive_labels[1].display_text(),
        "OnlyComponent"
    );
    assert_close(label.length.unwrap(), 3.81);
    assert_eq!(label.shape.as_deref(), Some("dot"));
    assert_eq!(label.fields_autoplaced, Some(true));
    assert_eq!(
        label.effects.as_ref().unwrap().font_color,
        Some(NspColor {
            red: 236.0,
            green: 104.0,
            blue: 255.0,
            alpha: 1.0,
        })
    );
    assert_eq!(label.properties.len(), 2);
    assert!(
        schematic
            .to_summary_json()
            .contains("\"directive_label_count\": 2")
    );
    assert!(
        schematic
            .to_summary_json()
            .contains("\"directive_label_property_count\": 4")
    );

    let scene = schematic.canvas_scene();
    assert_eq!(scene.directive_labels.len(), 2);
    assert_eq!(scene.directive_labels[0].text, "HV");
    assert_eq!(scene.directive_labels[1].text, "OnlyComponent");
    assert!(
        scene
            .to_summary_json()
            .contains("\"directive_label_count\": 2")
    );
    let scene_json: serde_json::Value = serde_json::from_str(&scene.to_json()).unwrap();
    assert_eq!(scene_json["directive_label_count"], 2);
    assert_eq!(scene_json["directive_labels"][0]["text"], "HV");
    assert_eq!(scene_json["directive_labels"][0]["shape"], "dot");
    assert_eq!(
        scene_json["directive_labels"][0]["properties"][1]["effects"]["font_italic"],
        true
    );

    let roundtrip = schematic.to_schematic_sexpr();
    assert!(roundtrip.contains("(netclass_flag \"\""));
    assert!(roundtrip.contains("(length 3.81)"));
    assert!(roundtrip.contains("(shape dot)"));
    assert!(roundtrip.contains("(fields_autoplaced yes)"));
    assert!(roundtrip.contains("(color 236 104 255 1)"));
    assert!(roundtrip.contains("(property \"Net Class\" \"HV\""));
    let reparsed = parse_schematic(&roundtrip, "directive_label_roundtrip.nsp_sch").unwrap();
    assert_eq!(reparsed.directive_labels.len(), 2);
    assert_eq!(
        reparsed.directive_labels[0].uuid.as_deref(),
        Some("3c7ec402-4c06-4b52-9acd-ed760671ff85")
    );
    assert_eq!(reparsed.directive_labels[0].display_text(), "HV");
    assert_eq!(reparsed.directive_labels[1].display_text(), "OnlyComponent");
}

#[test]
fn preserves_label_shape_autoplace_and_properties() {
    let schematic = parse_schematic(
        r#"(nsp_sch
  (version 20251028)
  (generator "eeschema")
  (paper "A4")
  (lib_symbols)
  (global_label "NET_OK" (shape input) (at 31.75 30.48 0) (fields_autoplaced)
    (effects (font (size 1.27 1.27)) (justify left))
    (uuid "11111111-1111-4111-8111-111111111111")
    (property "Intersheet References" "${INTERSHEET_REFS}" (id 0) (at 41.2993 30.4006 0)
      (effects (font (size 1.27 1.27)) (justify left) hide)
    )
  )
  (hierarchical_label "CHILD_IN"
    (shape output)
    (at 50.8 30.48 180)
    (fields_autoplaced no)
    (effects (font (size 1.27 1.27)) (justify right))
    (uuid "22222222-2222-4222-8222-222222222222")
  )
)"#,
        "label_metadata.nsp_sch",
    )
    .unwrap();

    assert_eq!(schematic.labels.len(), 2);
    let global = &schematic.labels[0];
    assert_eq!(global.kind, NspLabelKind::Global);
    assert_eq!(global.shape.as_deref(), Some("input"));
    assert_eq!(global.fields_autoplaced, Some(true));
    assert_eq!(global.properties.len(), 1);
    assert_eq!(global.properties[0].id, Some(0));
    assert!(global.properties[0].effects.as_ref().unwrap().hide);

    let hierarchical = &schematic.labels[1];
    assert_eq!(hierarchical.kind, NspLabelKind::Hierarchical);
    assert_eq!(hierarchical.shape.as_deref(), Some("output"));
    assert_eq!(hierarchical.fields_autoplaced, Some(false));
    assert!(
        schematic
            .to_summary_json()
            .contains("\"fields_autoplaced_count\": 1")
    );
    assert!(
        schematic
            .to_summary_json()
            .contains("\"shaped_label_count\": 2")
    );
    assert!(
        schematic
            .to_summary_json()
            .contains("\"label_property_count\": 1")
    );
    assert!(
        schematic
            .to_summary_json()
            .contains("\"hidden_property_count\": 1")
    );

    let roundtrip = schematic.to_schematic_sexpr();
    assert!(roundtrip.contains("(global_label \"NET_OK\" (shape input)"));
    assert!(roundtrip.contains("(fields_autoplaced yes)"));
    assert!(
        roundtrip.contains("(property \"Intersheet References\" \"${INTERSHEET_REFS}\" (id 0)")
    );
    assert!(roundtrip.contains("(justify left) hide"));
    assert!(roundtrip.contains("(hierarchical_label \"CHILD_IN\" (shape output)"));
    assert!(roundtrip.contains("(fields_autoplaced no)"));

    let reparsed = parse_schematic(&roundtrip, "label_metadata_roundtrip.nsp_sch").unwrap();
    assert_eq!(reparsed.labels[0].shape.as_deref(), Some("input"));
    assert_eq!(reparsed.labels[0].fields_autoplaced, Some(true));
    assert_eq!(reparsed.labels[0].properties[0].id, Some(0));
    assert_eq!(reparsed.labels[1].fields_autoplaced, Some(false));
}
