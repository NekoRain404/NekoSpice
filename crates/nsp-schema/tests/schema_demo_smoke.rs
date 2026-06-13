use nsp_schema::{
    NspCanvasScene, NspDiagnosticSeverity, read_project, read_schematic_with_libraries,
    read_symbol_library_index,
};
use std::env;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy)]
struct DemoCase {
    name: &'static str,
    project: &'static str,
    schematic: &'static str,
    library_table: &'static str,
    min_symbols: usize,
    min_pins: usize,
    min_wires: usize,
    expect_spice_directive: bool,
    expect_sheet: bool,
    expect_bus: bool,
    expect_graphic: bool,
}

const DEMO_CASES: &[DemoCase] = &[
    DemoCase {
        name: "simulation_rectifier",
        project: "simulation/rectifier/rectifier.nsp_pro",
        schematic: "simulation/rectifier/rectifier.nsp_sch",
        library_table: "simulation/rectifier/sym-lib-table",
        min_symbols: 6,
        min_pins: 10,
        min_wires: 10,
        expect_spice_directive: true,
        expect_sheet: false,
        expect_bus: false,
        expect_graphic: false,
    },
    DemoCase {
        name: "complex_hierarchy",
        project: "complex_hierarchy/complex_hierarchy.nsp_pro",
        schematic: "complex_hierarchy/complex_hierarchy.nsp_sch",
        library_table: "complex_hierarchy/sym-lib-table",
        min_symbols: 20,
        min_pins: 40,
        min_wires: 30,
        expect_spice_directive: false,
        expect_sheet: true,
        expect_bus: false,
        expect_graphic: false,
    },
    DemoCase {
        name: "cm5_minima",
        project: "cm5_minima/CM5_MINIMA_3.nsp_pro",
        schematic: "cm5_minima/CM5_MINIMA_3.nsp_sch",
        library_table: "cm5_minima/sym-lib-table",
        min_symbols: 40,
        min_pins: 60,
        min_wires: 80,
        expect_spice_directive: false,
        expect_sheet: true,
        expect_bus: true,
        expect_graphic: true,
    },
];

#[test]
fn opens_representative_schema_demo_projects_from_reference_source() {
    let Some(demo_root) = schema_demo_root() else {
        eprintln!(
            "skipping schema demo smoke test: set NEKOSPICE_SCHEMA_DEMOS to a schema demos directory"
        );
        return;
    };

    for case in DEMO_CASES {
        smoke_demo_case(&demo_root, case);
    }
}

fn smoke_demo_case(demo_root: &Path, case: &DemoCase) {
    let project_path = demo_root.join(case.project);
    let schematic_path = demo_root.join(case.schematic);
    let library_table_path = demo_root.join(case.library_table);

    let project = read_project(&project_path)
        .unwrap_or_else(|error| panic!("{} project parse failed: {error}", case.name));
    assert!(
        !project.schematic_stem_candidates().is_empty(),
        "{} project should expose schematic stem candidates",
        case.name
    );

    let library_index = read_symbol_library_index(&library_table_path)
        .unwrap_or_else(|error| panic!("{} library index failed: {error}", case.name));
    assert!(
        !library_index.libraries.is_empty(),
        "{} should load at least one local symbol library",
        case.name
    );
    assert!(
        !library_index.symbols.is_empty(),
        "{} should index symbols from local sym-lib-table",
        case.name
    );
    assert!(
        library_index
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.severity != NspDiagnosticSeverity::Error),
        "{} symbol library index returned error diagnostics: {:?}",
        case.name,
        library_index.diagnostics
    );

    let schematic = read_schematic_with_libraries(&schematic_path)
        .unwrap_or_else(|error| panic!("{} schematic parse failed: {error}", case.name));
    let scene = schematic.canvas_scene();
    assert_scene_baseline(case, &scene);

    let report = schematic.check_report();
    assert_eq!(
        report.symbol_count,
        schematic.symbols.len(),
        "{} check report should stay aligned with parsed symbols",
        case.name
    );
    assert!(
        report.net_count > 0,
        "{} should build a non-empty connectivity graph",
        case.name
    );
}

fn assert_scene_baseline(case: &DemoCase, scene: &NspCanvasScene) {
    let pin_count = scene
        .symbols
        .iter()
        .map(|symbol| symbol.pins.len())
        .sum::<usize>();
    assert!(
        scene.symbols.len() >= case.min_symbols,
        "{} should expose placed symbols in the canvas scene: got {}",
        case.name,
        scene.symbols.len()
    );
    assert!(
        pin_count >= case.min_pins,
        "{} should expose transformed pins in the canvas scene: got {}",
        case.name,
        pin_count
    );
    assert!(
        scene.wires.len() >= case.min_wires,
        "{} should expose schematic wires in the canvas scene: got {}",
        case.name,
        scene.wires.len()
    );
    assert!(
        scene.bounds.is_some(),
        "{} should compute scene bounds for viewport fit and culling",
        case.name
    );

    if case.expect_spice_directive {
        assert!(
            scene.text_items.iter().any(|text| text.is_spice_directive),
            "{} should preserve SPICE directive text in the canvas scene",
            case.name
        );
    }
    if case.expect_sheet {
        assert!(
            !scene.sheets.is_empty(),
            "{} should expose hierarchical sheet boxes",
            case.name
        );
    }
    if case.expect_bus {
        assert!(
            !scene.buses.is_empty(),
            "{} should expose schema bus geometry",
            case.name
        );
    }
    if case.expect_graphic {
        assert!(
            !scene.graphics.is_empty(),
            "{} should expose schematic-level drawing graphics",
            case.name
        );
    }
}

fn schema_demo_root() -> Option<PathBuf> {
    if let Some(path) = env::var_os("NEKOSPICE_SCHEMA_DEMOS").map(PathBuf::from) {
        return valid_demo_root(path);
    }

    workspace_relative_demo_root()
        .into_iter()
        .chain(current_dir_demo_roots())
        .find_map(valid_demo_root)
}

fn workspace_relative_demo_root() -> Option<PathBuf> {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .find(|path| path.join("Cargo.toml").is_file() && path.join("crates").is_dir())
        .map(|workspace| workspace.join("schema-source-mirror/demos"))
}

fn current_dir_demo_roots() -> Vec<PathBuf> {
    env::current_dir()
        .ok()
        .into_iter()
        .flat_map(|cwd| {
            cwd.ancestors()
                .map(|path| path.join("schema-source-mirror/demos"))
                .collect::<Vec<_>>()
        })
        .collect()
}

fn valid_demo_root(path: PathBuf) -> Option<PathBuf> {
    path.join("simulation/rectifier/rectifier.nsp_sch")
        .is_file()
        .then_some(path)
}
