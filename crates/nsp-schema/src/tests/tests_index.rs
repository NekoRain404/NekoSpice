//! Domain-focused tests for nsp-schema.

use crate::{
    NspCanvasScene, NspIndexedSymbolBodyStyle, NspIndexedSymbolUnit, NspSymbolLibraryIndexQuery,
    parse_project, parse_schematic, parse_symbol_library, parse_symbol_library_table, read_project,
    read_symbol_library, read_symbol_library_index,
};
use std::fs;
use std::path::Path;

#[test]
fn resolves_missing_symbols_from_project_library_table() {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap();
    let project_dir = std::env::temp_dir().join(format!(
        "nekospice_schema_library_resolution_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&project_dir);
    fs::create_dir_all(&project_dir).unwrap();
    fs::copy(
        workspace_root.join("examples/schema_schematic/neko_spice.nsp_sym"),
        project_dir.join("neko_spice.nsp_sym"),
    )
    .unwrap();
    fs::write(
            project_dir.join("sym-lib-table"),
            r#"(sym_lib_table
  (version 7)
  (lib (name "NekoSpice")(type "NekoSpice")(uri "${KIPRJMOD}/neko_spice.nsp_sym")(options "")(descr ""))
)"#,
        )
        .unwrap();
    let mut schematic = parse_schematic(
        r#"(nsp_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols)
  (wire (pts (xy 10 10) (xy 7 10)))
  (wire (pts (xy 15.08 10) (xy 18 10)))
  (label "in" (at 7 10 0))
  (label "0" (at 18 10 0))
  (text ".op" (at 5 5 0))
  (symbol
    (lib_id "NekoSpice:R")
    (at 12.54 10 0)
    (property "Reference" "R1" (at 12.54 8 0))
    (property "Value" "1k" (at 12.54 12 0))
  )
)"#,
        "library_resolution.nsp_sch",
    )
    .unwrap();

    assert!(
        schematic
            .check_report()
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "missing-symbol-definition")
    );
    let _diagnostics = schematic
        .resolve_project_symbol_libraries(&project_dir)
        .unwrap();
    let netlist = schematic.to_spice_netlist().unwrap();

    // lib resolution warnings are environmental
    assert_eq!(schematic.library_symbols.len(), 1);
    assert!(netlist.contains("R1 in 0 1k"));
    assert!(
        !schematic
            .check_report()
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "missing-symbol-definition")
    );

    let _ = fs::remove_dir_all(project_dir);
}

#[test]
fn resolves_external_derived_symbol_parent_from_library_table() {
    let project_dir = std::env::temp_dir().join(format!(
        "nekospice_schema_derived_library_resolution_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&project_dir);
    fs::create_dir_all(&project_dir).unwrap();
    fs::write(
        project_dir.join("derived.nsp_sym"),
        r#"(nsp_symbol_lib
  (version 20230121)
  (generator "NekoSpice")
  (symbol "BaseR"
    (property "Reference" "R" (at 0 0 0))
    (property "Value" "1k" (at 0 -2.54 0))
    (property "Sim.Device" "R" (at 0 0 0))
    (symbol "BaseR_0_1"
      (pin passive line (at -2.54 0 0) (length 2.54) (name "~") (number "1"))
      (pin passive line (at 2.54 0 180) (length 2.54) (name "~") (number "2"))
    )
  )
  (symbol "DerivedR"
    (extends "BaseR")
    (property "Reference" "R" (at 0 0 0))
    (property "Value" "10k" (at 0 -2.54 0))
  )
)"#,
    )
    .unwrap();
    fs::write(
        project_dir.join("sym-lib-table"),
        r#"(sym_lib_table
  (version 7)
  (lib (name "Demo")(type "NekoSpice")(uri "${KIPRJMOD}/derived.nsp_sym")(options "")(descr ""))
)"#,
    )
    .unwrap();
    let mut schematic = parse_schematic(
        r#"(nsp_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols)
  (wire (pts (xy 17.46 10) (xy 10 10)))
  (wire (pts (xy 22.54 10) (xy 30 10)))
  (label "in" (at 10 10 0))
  (label "0" (at 30 10 0))
  (text ".op" (at 5 5 0))
  (symbol
    (lib_id "Demo:DerivedR")
    (at 20 10 0)
    (property "Reference" "R1" (at 20 8 0))
    (property "Value" "4.7k" (at 20 12 0))
  )
)"#,
        "derived_library_resolution.nsp_sch",
    )
    .unwrap();

    let _diagnostics = schematic
        .resolve_project_symbol_libraries(&project_dir)
        .unwrap();
    let scene = schematic.canvas_scene();
    let netlist = schematic.to_spice_netlist().unwrap();

    // lib resolution warnings are environmental
    assert!(schematic.symbol_definition("Demo:DerivedR").is_some());
    assert!(schematic.symbol_definition("Demo:BaseR").is_some());
    assert_eq!(scene.symbols[0].pins.len(), 2);
    assert!(netlist.contains("R1 in 0 4.7k"));

    let _ = fs::remove_dir_all(project_dir);
}

#[test]
fn parses_schema_project_fixture_and_sheet_summary() {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap();
    let project = read_project(
        &workspace_root.join("examples/schema_project_schematic/project_schematic.nsp_pro"),
    )
    .unwrap();

    assert_eq!(
        project.meta_filename.as_deref(),
        Some("project_schematic.nsp_pro")
    );
    assert_eq!(project.meta_version, Some(1));
    assert_eq!(project.project_name.as_deref(), Some("project_schematic"));
    assert!(
        project
            .schematic_stem_candidates()
            .contains(&"project_schematic".to_string())
    );
    assert!(project.to_summary_json().contains("\"project_name\""));

    let project = parse_project(
        r#"{
  "meta": { "filename": "root_project.nsp_pro", "version": 2 },
  "schematic": { "page_layout_descr_file": "layout.schema_wks" },
  "sheets": [
    [ "root-sheet", "Root" ],
    [ "child-sheet", "child" ]
  ],
  "text_variables": { "REV": "A" }
}"#,
        "root_project.nsp_pro",
    )
    .unwrap();

    assert_eq!(project.meta_version, Some(2));
    assert_eq!(
        project.schematic_page_layout_descr_file.as_deref(),
        Some("layout.schema_wks")
    );
    assert_eq!(project.sheets.len(), 2);
    assert_eq!(project.sheets[0].name, "Root");
    assert_eq!(project.sheets[1].uuid, "child-sheet");
    assert_eq!(project.text_variable_count, 1);
    assert_eq!(
        project.schematic_stem_candidates(),
        vec!["root_project".to_string()]
    );
}

#[test]
fn builds_schema_symbol_library_index_fixture() {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap();
    let index =
        read_symbol_library_index(&workspace_root.join("examples/schema_schematic/sym-lib-table"))
            .unwrap();

    assert_eq!(index.libraries.len(), 1);
    assert_eq!(index.symbols.len(), 3);
    assert_eq!(index.diagnostics.len(), 0);
    let resistor = index.symbol("NekoSpice:R").unwrap();
    assert_eq!(resistor.library, "NekoSpice");
    assert_eq!(resistor.name, "R");
    assert_eq!(resistor.pin_count, 2);
    assert_eq!(resistor.graphic_count, 1);
    assert!(resistor.bounding_box.is_some());
    assert!(index.to_summary_json().contains("\"symbol_count\": 3"));
}

#[test]
fn indexes_schema_symbol_library_browser_metadata() {
    let project_dir = std::env::temp_dir().join(format!(
        "nekospice_schema_symbol_index_metadata_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&project_dir);
    fs::create_dir_all(&project_dir).unwrap();
    fs::write(
        project_dir.join("browser.nsp_sym"),
        r##"(nsp_symbol_lib
  (version 20230121)
  (generator "NekoSpice")
  (symbol "Parent"
    (property "Reference" "U" (at 0 0 0))
    (property "Value" "Parent" (at 0 -2.54 0))
    (property "Description" "Parent analog switch" (at 0 -5.08 0))
    (property "ki_keywords" "switch analog mux" (at 0 -7.62 0) (hide yes))
    (property "ki_fp_filters" "Package_SO:SOIC-* Connector{space}Foo:*" (at 0 -10.16 0) (hide yes))
    (symbol "Parent_0_1"
      (rectangle (start -1 -1) (end 1 1) (stroke (width 0.127) (type default)) (fill (type none)))
    )
  )
  (symbol "Derived"
    (extends "Parent")
    (body_styles "normal" "alternate-body" "unused-body")
    (property "Reference" "U" (at 0 0 0))
    (property "Value" "Derived" (at 0 -2.54 0))
    (property "ki_keywords" "" (at 0 -5.08 0) (hide yes))
    (symbol "Derived_1_1"
      (unit_name "Logic")
      (pin passive line
        (at -2.54 0 0)
        (length 2.54)
        (name "A")
        (number "1")
        (alternate "A_ALT" bidirectional line)
      )
    )
    (symbol "Derived_2_1"
      (unit_name "Power")
      (pin power_in line (at 2.54 0 180) (length 2.54) (name "VCC") (number "2"))
    )
    (symbol "Derived_1_2"
      (pin passive inverted (at -2.54 2.54 0) (length 2.54) (name "A2") (number "3"))
    )
  )
  (symbol "PWR" (power global)
    (property "Reference" "#PWR" (at 0 0 0))
    (property "Value" "PWR" (at 0 -2.54 0))
  )
)"##,
    )
    .unwrap();
    fs::write(
        project_dir.join("sym-lib-table"),
        r#"(sym_lib_table
  (version 7)
  (lib (name "Browser")(type "NekoSpice")(uri "${KIPRJMOD}/browser.nsp_sym")(options "")(descr ""))
)"#,
    )
    .unwrap();

    let index = read_symbol_library_index(&project_dir.join("sym-lib-table")).unwrap();
    let derived = index.symbol("Browser:Derived").unwrap();
    let power = index.symbol("Browser:PWR").unwrap();

    assert_eq!(derived.description.as_deref(), Some("Parent analog switch"));
    assert_eq!(derived.keywords.as_deref(), Some("switch analog mux"));
    assert_eq!(
        derived.footprint_filters,
        vec![
            "Package_SO:SOIC-*".to_string(),
            "Connector Foo:*".to_string()
        ]
    );
    assert_eq!(derived.pin_count, 3);
    assert_eq!(derived.graphic_count, 1);
    assert_eq!(derived.unit_count, 2);
    assert_eq!(
        derived.units,
        vec![
            NspIndexedSymbolUnit {
                unit: 1,
                name: Some("Logic".to_string())
            },
            NspIndexedSymbolUnit {
                unit: 2,
                name: Some("Power".to_string())
            }
        ]
    );
    assert_eq!(
        derived.body_styles,
        vec![
            NspIndexedSymbolBodyStyle {
                body_style: 1,
                name: Some("normal".to_string())
            },
            NspIndexedSymbolBodyStyle {
                body_style: 2,
                name: Some("alternate-body".to_string())
            },
            NspIndexedSymbolBodyStyle {
                body_style: 3,
                name: Some("unused-body".to_string())
            }
        ]
    );
    assert_eq!(derived.pins.len(), 3);
    assert_eq!(derived.pins[0].number, "1");
    assert_eq!(derived.pins[0].alternates[0].name, "A_ALT");
    assert_eq!(derived.pins[2].body_style, 2);
    assert_eq!(derived.extends.as_deref(), Some("Parent"));
    assert_eq!(power.power.as_deref(), Some("global"));
    assert!(index.to_summary_json().contains("\"unit_count\": 4"));
    assert!(
        index
            .to_summary_json()
            .contains("\"described_symbol_count\": 2")
    );
    assert!(
        index
            .to_summary_json()
            .contains("\"keyword_symbol_count\": 2")
    );
    assert!(
        index
            .to_summary_json()
            .contains("\"footprint_filter_count\": 4")
    );
    assert!(
        index
            .to_summary_json()
            .contains("\"extended_symbol_count\": 1")
    );
    assert!(
        index
            .to_summary_json()
            .contains("\"power_symbol_count\": 1")
    );
    let index_json: serde_json::Value = serde_json::from_str(&index.to_json()).unwrap();
    assert_eq!(index_json["library_count"], 1);
    assert_eq!(index_json["symbol_count"], 3);
    assert_eq!(index_json["libraries"][0]["name"], "Browser");
    assert_eq!(index_json["symbols"][1]["id"], "Browser:Derived");
    assert_eq!(
        index_json["symbols"][1]["description"],
        "Parent analog switch"
    );
    assert_eq!(
        index_json["symbols"][1]["footprint_filters"][1],
        "Connector Foo:*"
    );
    assert_eq!(index_json["symbols"][1]["units"][0]["name"], "Logic");
    assert_eq!(
        index_json["symbols"][1]["body_styles"][1]["name"],
        "alternate-body"
    );
    assert_eq!(
        index_json["symbols"][1]["body_styles"][2]["name"],
        "unused-body"
    );
    assert_eq!(
        index_json["symbols"][1]["pins"][0]["alternates"][0]["name"],
        "A_ALT"
    );
    assert_eq!(index_json["symbols"][1]["bounding_box"]["min"]["x"], -2.54);
    assert_eq!(index_json["diagnostic_count"], 0);
    assert!(index_json["diagnostics"].as_array().unwrap().is_empty());

    let by_text = index.query(&NspSymbolLibraryIndexQuery {
        text: Some("analog".to_string()),
        ..Default::default()
    });
    assert_eq!(
        by_text
            .symbols
            .iter()
            .map(|symbol| symbol.id.as_str())
            .collect::<Vec<_>>(),
        vec!["Browser:Parent", "Browser:Derived"]
    );
    let by_footprint = index.query(&NspSymbolLibraryIndexQuery {
        footprint: Some("Connector Foo:Bar".to_string()),
        ..Default::default()
    });
    assert_eq!(by_footprint.symbols.len(), 2);
    assert_eq!(by_footprint.libraries[0].symbol_count, 2);
    let by_library = index.query(&NspSymbolLibraryIndexQuery {
        library: Some("missing".to_string()),
        ..Default::default()
    });
    assert!(by_library.symbols.is_empty());
    assert!(by_library.libraries.is_empty());

    let library = read_symbol_library(&project_dir.join("browser.nsp_sym")).unwrap();
    let parent = library.symbol("Parent").unwrap();
    assert_eq!(parent.description(), Some("Parent analog switch"));
    assert_eq!(parent.keywords(), Some("switch analog mux"));
    assert_eq!(
        parent.footprint_filters(),
        vec![
            "Package_SO:SOIC-*".to_string(),
            "Connector Foo:*".to_string()
        ]
    );
    let exported = library.to_symbol_library_sexpr();
    assert!(exported.contains("(property \"Description\" \"Parent analog switch\""));
    assert!(exported.contains("(property \"ki_keywords\" \"switch analog mux\""));
    assert!(
        exported
            .contains("(property \"ki_fp_filters\" \"Package_SO:SOIC-* Connector{space}Foo:*\"")
    );
    let reparsed = parse_symbol_library(&exported, "browser_roundtrip.nsp_sym").unwrap();
    assert_eq!(
        reparsed.symbol("Parent").unwrap().footprint_filters(),
        vec![
            "Package_SO:SOIC-*".to_string(),
            "Connector Foo:*".to_string()
        ]
    );

    let _ = fs::remove_dir_all(project_dir);
}

#[test]
fn builds_symbol_library_preview_canvas_scene() {
    let library = parse_symbol_library(
        r#"(nsp_symbol_lib
  (version 20230121)
  (generator "NekoSpice")
  (symbol "Parent"
    (property "Reference" "U" (at 0 0 0))
    (property "Value" "Parent" (at 0 -2.54 0))
    (symbol "Parent_0_1"
      (rectangle (start -1 -1) (end 1 1) (stroke (width 0.127) (type default)) (fill (type none)))
    )
  )
  (symbol "Derived"
    (extends "Parent")
    (property "Reference" "U" (at 0 0 0))
    (property "Value" "Derived" (at 0 -2.54 0))
    (symbol "Derived_1_1"
      (unit_name "Logic")
      (pin passive line (at -2.54 0 0) (length 2.54) (name "A") (number "1"))
    )
  )
)"#,
        "preview.nsp_sym",
    )
    .unwrap();
    let symbol = library.symbol_by_name_or_local_name("Derived").unwrap();
    let scene = NspCanvasScene::from_symbol_definition(
        "preview.nsp_sym:Derived",
        symbol,
        &library.symbols,
        Some(1),
        None,
    );

    assert_eq!(scene.source, "preview.nsp_sym:Derived");
    assert_eq!(scene.symbols.len(), 1);
    assert_eq!(scene.symbols[0].lib_id, "Derived");
    assert_eq!(scene.symbols[0].value, "Derived");
    assert_eq!(scene.symbols[0].unit_name.as_deref(), Some("Logic"));
    assert_eq!(scene.symbols[0].graphics.len(), 1);
    assert_eq!(scene.symbols[0].pins.len(), 1);
    assert!(scene.bounds.is_some());
    assert!(scene.to_summary_json().contains("\"symbol_count\": 1"));
    let json: serde_json::Value = serde_json::from_str(&scene.to_json()).unwrap();
    assert_eq!(json["symbol_count"], 1);
    assert_eq!(json["symbols"][0]["lib_id"], "Derived");
    assert_eq!(json["symbols"][0]["unit_name"], "Logic");
    assert_eq!(json["symbols"][0]["pins"][0]["number"], "1");
    assert_eq!(json["symbols"][0]["graphics"][0]["kind"], "rectangle");
}

#[test]
fn indexes_schema_library_table_diagnostics() {
    let table = parse_symbol_library_table(
        r#"(sym_lib_table
  (version 7)
  (lib (name "Disabled")(type "NekoSpice")(uri "disabled.nsp_sym")(options "")(descr "")(disabled))
  (lib (name "Future")(type "FutureCAD")(uri "future.nsp_sym")(options "")(descr ""))
)"#,
        "inline",
    )
    .unwrap();

    let index = crate::NspSymbolLibraryIndex::from_table(table, Path::new("."));
    assert_eq!(index.libraries.len(), 0);
    assert_eq!(index.symbols.len(), 0);
    assert_eq!(index.diagnostics.len(), 2);
    assert!(
        index
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message == "library row is disabled")
    );
    assert!(index.diagnostics.iter().any(|diagnostic| {
        diagnostic
            .message
            .contains("unsupported symbol library type")
    }));
}
