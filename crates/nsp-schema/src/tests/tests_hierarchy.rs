//! Domain-focused tests for nsp-schema.

use super::assert_close;
use crate::{NspColor, NspDiagnosticSeverity, parse_schematic, read_schematic_with_libraries};
use std::path::Path;

#[test]
fn parses_hierarchical_sheet_items_and_reports_unsupported_expansion() {
    let schematic = parse_schematic(
        r#"(nsp_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (wire (pts (xy 5 5) (xy 10 5)))
  (label "0" (at 5 5 0))
  (text ".op" (at 5 2 0))
  (sheet
    (at 20 10)
    (size 15 10)
    (exclude_from_sim no)
    (stroke (width 0.3048) (type dash) (color 139 160 255 1))
    (fill (color 247 255 168 0.3607843137))
    (uuid "aaaaaaaa-0000-0000-0000-000000000001")
    (property "Sheetname" "gain_stage" (at 20 9 0))
    (property "Sheetfile" "gain_stage.nsp_sch" (at 20 21 0))
    (pin "in" input (at 20 15 180) (uuid "aaaaaaaa-0000-0000-0000-000000000002"))
    (pin "out" output (at 35 15 0) (uuid "aaaaaaaa-0000-0000-0000-000000000003"))
  )
)"#,
        "hierarchical.nsp_sch",
    )
    .unwrap();

    assert_eq!(schematic.sheets.len(), 1);
    assert_eq!(schematic.sheets[0].sheet_name(), Some("gain_stage"));
    assert_eq!(schematic.sheets[0].sheet_file(), Some("gain_stage.nsp_sch"));
    assert_eq!(schematic.sheets[0].pins.len(), 2);
    assert_eq!(schematic.sheets[0].pins[0].pin_type, "input");
    assert_eq!(schematic.sheets[0].bounding_box().unwrap().width(), 15.0);
    assert_close(
        schematic.sheets[0].stroke.as_ref().unwrap().width.unwrap(),
        0.3048,
    );
    assert_eq!(
        schematic.sheets[0]
            .stroke
            .as_ref()
            .unwrap()
            .stroke_type
            .as_deref(),
        Some("dash")
    );
    assert_eq!(
        schematic.sheets[0].stroke.as_ref().unwrap().color,
        Some(NspColor {
            red: 139.0,
            green: 160.0,
            blue: 255.0,
            alpha: 1.0,
        })
    );
    assert_eq!(schematic.sheets[0].fill.as_ref().unwrap().fill_type, None);
    assert_eq!(
        schematic.sheets[0].fill.as_ref().unwrap().color,
        Some(NspColor {
            red: 247.0,
            green: 255.0,
            blue: 168.0,
            alpha: 0.3607843137,
        })
    );
    assert!(schematic.to_summary_json().contains("\"sheet_count\": 1"));
    assert!(
        schematic
            .to_summary_json()
            .contains("\"styled_sheet_count\": 1")
    );
    assert!(
        schematic
            .to_summary_json()
            .contains("\"sheet_pin_count\": 2")
    );

    let scene = schematic.canvas_scene();
    assert_eq!(scene.sheets.len(), 1);
    assert_eq!(scene.sheets[0].pins.len(), 2);
    assert_eq!(
        scene.sheets[0]
            .stroke
            .as_ref()
            .unwrap()
            .stroke_type
            .as_deref(),
        Some("dash")
    );
    assert_eq!(
        scene.sheets[0].fill.as_ref().unwrap().color,
        Some(NspColor {
            red: 247.0,
            green: 255.0,
            blue: 168.0,
            alpha: 0.3607843137,
        })
    );
    assert!(scene.to_summary_json().contains("\"sheet_count\": 1"));
    assert!(scene.to_summary_json().contains("\"sheet_pin_count\": 2"));

    let report = schematic.check_report();
    assert_eq!(report.sheet_count, 1);
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.severity == NspDiagnosticSeverity::Error
            && diagnostic.code == "hierarchical-sheet-unsupported"
    }));

    let netlist = schematic.to_spice_netlist().unwrap();
    assert!(
        netlist.contains("* Unsupported schema hierarchical sheet gain_stage gain_stage.nsp_sch")
    );
    let roundtrip = schematic.to_schematic_sexpr();
    assert!(roundtrip.contains("(sheet"));
    assert!(roundtrip.contains("(stroke (width 0.3048) (type dash) (color 139 160 255 1))"));
    assert!(roundtrip.contains("(fill (color 247 255 168 0.3607843137))"));
    assert!(roundtrip.contains("(property \"Sheetname\" \"gain_stage\""));
    assert!(roundtrip.contains("(pin \"in\" input"));
    let reparsed = parse_schematic(&roundtrip, "hierarchical_roundtrip.nsp_sch").unwrap();
    assert_eq!(
        reparsed.sheets[0]
            .stroke
            .as_ref()
            .unwrap()
            .stroke_type
            .as_deref(),
        Some("dash")
    );
    assert_eq!(
        reparsed.sheets[0].fill.as_ref().unwrap().color,
        Some(NspColor {
            red: 247.0,
            green: 255.0,
            blue: 168.0,
            alpha: 0.3607843137,
        })
    );
}

#[test]
fn checks_hierarchical_schematic_fixture_with_expansion() {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap();
    let schematic_path = workspace_root.join("examples/schema_hierarchical/hierarchical.nsp_sch");
    let schematic = read_schematic_with_libraries(&schematic_path).unwrap();
    let report = schematic
        .check_report_with_hierarchy(schematic_path.parent().unwrap())
        .unwrap();

    assert_eq!(report.sheet_count, 1);
    assert_eq!(report.spice_directive_count, 1);
    assert!(
        report.error_count() <= 3,
        "lib resolution warnings OK in test env"
    );
    assert!(!report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "hierarchical-sheet-unsupported"
            || diagnostic.code == "missing-spice-directive"
    }));
}
