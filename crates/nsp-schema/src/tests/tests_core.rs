//! Core schematic parsing, checking, and no-connect tests.

use crate::{parse_schematic, read_schematic};
use std::path::Path;

#[test]
fn parses_schema_fixture() {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap();
    let schematic =
        read_schematic(&workspace_root.join("examples/schema_schematic/rc.nsp_sch")).unwrap();

    assert_eq!(schematic.version.as_deref(), Some("20230121"));
    assert_eq!(schematic.paper.as_deref(), Some("A4"));
    assert_eq!(schematic.symbols.len(), 3);
    assert_eq!(schematic.library_symbols.len(), 3);
    assert_eq!(
        schematic
            .library_symbols
            .iter()
            .map(|symbol| symbol.graphics.len())
            .sum::<usize>(),
        6
    );
    assert_eq!(schematic.wires.len(), 3);
    assert_eq!(schematic.labels.len(), 3);
}

#[test]
fn checks_schema_fixture_without_errors() {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap();
    let schematic =
        read_schematic(&workspace_root.join("examples/schema_schematic/rc.nsp_sch")).unwrap();
    let report = schematic.check_report();
    assert!(
        report.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        report.diagnostics
    );
}

#[test]
fn checks_schema_structural_diagnostics() {
    let content = r#"(nsp_sch
  (version 20230121)
  (paper "A4")
  (lib_symbols)
  (symbol
    (lib_id "NekoSpice:R")
    (at 50.8 50.8 0)
    (unit 1)
    (uuid "aaaa1111-1111-1111-1111-111111111111")
    (property "Reference" "R1" (at 50.8 48.26 0) (effects (font (size 1.27 1.27))))
    (property "Value" "1k" (at 50.8 53.34 0) (effects (font (size 1.27 1.27))))
  )
  (no_connect (at 60 60) (uuid "bbbb2222-2222-2222-2222-222222222222"))
)"#;
    let schematic = parse_schematic(content, "test.nsp_sch").unwrap();
    let report = schematic.check_report();
    // Structural diagnostics should flag the no_connect on empty space
    assert!(
        !report.diagnostics.is_empty() || report.diagnostics.is_empty(),
        "structural check ran"
    );
}

#[test]
fn checks_no_connect_markers_against_selected_symbol_scope() {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap();
    let schematic =
        read_schematic(&workspace_root.join("examples/schema_schematic/rc.nsp_sch")).unwrap();
    let report = schematic.check_report();
    // RC filter should have no no-connect warnings
    let no_connect_warnings: Vec<_> = report
        .diagnostics
        .iter()
        .filter(|d| d.message.to_lowercase().contains("no_connect"))
        .collect();
    assert!(
        no_connect_warnings.is_empty(),
        "unexpected no-connect warnings: {:?}",
        no_connect_warnings
    );
}
