//! Domain-focused tests for nsp-schema.

use crate::{parse_schematic, read_schematic};
use std::path::Path;

#[test]
fn builds_connectivity_and_exports_spice() {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap();
    let schematic =
        read_schematic(&workspace_root.join("examples/schema_schematic/rc.nsp_sch")).unwrap();

    let graph = schematic.connectivity_graph();
    assert_eq!(
        graph
            .nets
            .iter()
            .map(|net| net.name.as_str())
            .collect::<Vec<_>>(),
        ["0", "in", "out"]
    );

    let netlist = schematic.to_spice_netlist().unwrap();
    assert!(netlist.contains("V1 in 0 PULSE(0 1 0 1u 1u 10u 20u)"));
    assert!(netlist.contains("R1 in out 1k"));
    assert!(netlist.contains("C1 out 0 100n"));
    assert!(netlist.contains(".tran 1u 1m"));
    assert!(netlist.ends_with(".end\n"));
}

#[test]
fn exports_schema_sim_fields_to_spice_netlist() {
    let schematic = parse_schematic(
        r#"(nsp_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols
    (symbol "NekoSpice:Dual"
      (property "Reference" "U" (at 0 0 0))
      (property "Value" "unused" (at 0 -2.54 0))
      (property "Sim.Device" "SUBCKT" (at 0 0 0))
      (property "Sim.Library" "models/opamp.lib" (at 0 0 0))
      (symbol "Dual_0_1"
        (pin passive line (at -2.54 0 0) (length 2.54) (name "IN") (number "1"))
        (pin passive line (at 0 -2.54 90) (length 2.54) (name "OUT") (number "2"))
        (pin passive line (at 2.54 0 180) (length 2.54) (name "VCC") (number "3"))
      )
    )
    (symbol "NekoSpice:R"
      (property "Reference" "R" (at 0 0 0))
      (property "Value" "1k" (at 0 -2.54 0))
      (symbol "R_0_1"
        (pin passive line (at -2.54 0 0) (length 2.54) (name "~") (number "1"))
        (pin passive line (at 2.54 0 180) (length 2.54) (name "~") (number "2"))
      )
    )
  )
  (wire (pts (xy 10 10) (xy 17.46 10)))
  (wire (pts (xy 20 0) (xy 20 7.46)))
  (wire (pts (xy 22.54 10) (xy 30 10)))
  (label "in" (at 10 10 0))
  (label "out" (at 20 0 0))
  (label "vcc" (at 30 10 0))
  (text ".op" (at 5 5 0))
  (symbol
    (lib_id "NekoSpice:Dual")
    (at 20 10 0)
    (property "Reference" "U1" (at 20 8 0))
    (property "Value" "opamp_model" (at 20 12 0))
    (property "Sim.Pins" "2=OUT 1=IN 3=VCC" (at 20 10 0))
    (property "Sim.Params" "model=\"opamp_model\" gain=100k" (at 20 10 0))
  )
  (symbol
    (lib_id "NekoSpice:R")
    (at 50 50 0)
    (exclude_from_sim yes)
    (property "Reference" "Rskip" (at 50 48 0))
    (property "Value" "1k" (at 50 52 0))
  )
)"#,
        "sim_fields.nsp_sch",
    )
    .unwrap();

    let netlist = schematic.to_spice_netlist().unwrap();

    assert!(netlist.contains(".include \"models/opamp.lib\""));
    assert!(netlist.contains("XU1 out in vcc opamp_model gain=100k"));
    assert!(!netlist.contains("Rskip"));
    assert!(netlist.contains(".op"));
    let reparsed = parse_schematic(
        &schematic.to_schematic_sexpr(),
        "sim_fields_roundtrip.nsp_sch",
    )
    .unwrap();
    assert_eq!(
        reparsed
            .symbols
            .iter()
            .find(|symbol| symbol.reference() == Some("Rskip"))
            .unwrap()
            .exclude_from_sim,
        Some(true)
    );
}

#[test]
fn exports_legacy_schema_spice_fields_to_spice_netlist() {
    let schematic = parse_schematic(
        r#"(nsp_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols
    (symbol "NekoSpice:LegacyD"
      (property "Reference" "D" (at 0 0 0))
      (property "Value" "unused" (at 0 -2.54 0))
      (property "Spice_Primitive" "D" (at 0 0 0))
      (property "Spice_Model" "Dfast" (at 0 0 0))
      (symbol "LegacyD_0_1"
        (pin passive line (at 0 -2.54 90) (length 2.54) (name "A") (number "1"))
        (pin passive line (at 0 2.54 270) (length 2.54) (name "K") (number "2"))
      )
    )
  )
  (wire (pts (xy 40 37.46) (xy 35 37.46)))
  (wire (pts (xy 40 42.54) (xy 45 42.54)))
  (label "anode" (at 35 37.46 0))
  (label "0" (at 45 42.54 0))
  (text ".op" (at 5 5 0))
  (symbol
    (lib_id "NekoSpice:LegacyD")
    (at 40 40 0)
    (property "Reference" "XD1" (at 40 38 0))
    (property "Value" "ignored" (at 40 42 0))
    (property "Spice_Node_Sequence" "2 1" (at 40 40 0))
  )
)"#,
        "legacy_spice_fields.nsp_sch",
    )
    .unwrap();

    let netlist = schematic.to_spice_netlist().unwrap();

    assert!(netlist.contains("DXD1 0 anode Dfast"));
}

#[test]
fn reports_invalid_sim_pin_mapping() {
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
  (wire (pts (xy 10 10) (xy 20 10)))
  (label "0" (at 10 10 0))
  (text ".op" (at 5 5 0))
  (symbol
    (lib_id "NekoSpice:R")
    (at 12.54 10 0)
    (property "Reference" "R1" (at 12.54 8 0))
    (property "Value" "1k" (at 12.54 12 0))
    (property "Sim.Pins" "1 99" (at 12.54 10 0))
  )
)"#,
        "bad_sim_pins.nsp_sch",
    )
    .unwrap();

    let report = schematic.check_report();

    assert!(report.error_count() >= 1);
    assert!(
        report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "invalid-sim-pin")
    );
}
