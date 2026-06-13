use nsp_netlist::{ImportSeverity, NetlistFlavor, read_import_input};
use std::env;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy)]
struct DemoImportCase {
    name: &'static str,
    project: &'static str,
    schematic: &'static str,
    min_components: usize,
    min_symbols: usize,
    min_directives: usize,
    expected_directive: &'static str,
}

const DEMO_IMPORT_CASES: &[DemoImportCase] = &[
    DemoImportCase {
        name: "simulation_rectifier",
        project: "simulation/rectifier/rectifier.nsp_pro",
        schematic: "simulation/rectifier/rectifier.nsp_sch",
        min_components: 5,
        min_symbols: 5,
        min_directives: 3,
        expected_directive: ".tran",
    },
    DemoImportCase {
        name: "simulation_sallen_key",
        project: "simulation/sallen_key/sallen_key.nsp_pro",
        schematic: "simulation/sallen_key/sallen_key.nsp_sch",
        min_components: 7,
        min_symbols: 7,
        min_directives: 3,
        expected_directive: ".ac",
    },
    DemoImportCase {
        name: "simulation_subsheets",
        project: "simulation/subsheets/mainsheet.nsp_pro",
        schematic: "simulation/subsheets/mainsheet.nsp_sch",
        min_components: 13,
        min_symbols: 13,
        min_directives: 2,
        expected_directive: ".tran",
    },
    DemoImportCase {
        name: "simulation_v_i_sources",
        project: "simulation/v_i_sources/v_i_sources.nsp_pro",
        schematic: "simulation/v_i_sources/v_i_sources.nsp_sch",
        min_components: 30,
        min_symbols: 30,
        min_directives: 2,
        expected_directive: ".tran",
    },
];

#[test]
fn imports_representative_schema_simulation_demos_from_reference_source() {
    let Some(demo_root) = schema_demo_root() else {
        eprintln!(
            "skipping schema demo import smoke test: set NEKOSPICE_SCHEMA_DEMOS to a schema demos directory"
        );
        return;
    };

    for case in DEMO_IMPORT_CASES {
        smoke_demo_import_case(&demo_root, case);
    }
}

fn smoke_demo_import_case(demo_root: &Path, case: &DemoImportCase) {
    let project_path = demo_root.join(case.project);
    let schematic_path = demo_root.join(case.schematic);

    let input = read_import_input(&project_path)
        .unwrap_or_else(|error| panic!("{} import failed: {error}", case.name));
    let report = &input.report;
    assert_eq!(
        input.source_path, schematic_path,
        "{} project import should resolve to the root schematic",
        case.name
    );
    assert_eq!(
        report.flavor,
        NetlistFlavor::Schema,
        "{} should be classified as a schema import",
        case.name
    );
    assert!(
        report.component_count() >= case.min_components,
        "{} should import component instances: got {}",
        case.name,
        report.component_count()
    );
    assert!(
        report.symbol_count() >= case.min_symbols,
        "{} should import symbol-backed components: got {}",
        case.name,
        report.symbol_count()
    );
    assert!(
        report.directive_count() >= case.min_directives,
        "{} should preserve simulation directives: got {}",
        case.name,
        report.directive_count()
    );
    assert!(
        report
            .directives
            .iter()
            .any(|directive| directive.name == case.expected_directive),
        "{} should preserve {} analysis directive",
        case.name,
        case.expected_directive
    );
    assert!(
        input.source_netlist.contains(".end"),
        "{} should produce a runnable SPICE deck terminator",
        case.name
    );
    assert!(
        report.compatibility_score() > 0,
        "{} should keep a non-zero compatibility score",
        case.name
    );
    assert!(
        report.diagnostics.iter().any(|diagnostic| {
            diagnostic.severity == ImportSeverity::Error
                && diagnostic.code == "schema-missing-ground"
        }),
        "{} should surface schema missing-ground diagnostics explicitly",
        case.name
    );

    let project = report.normalized_project(&input.source_netlist);
    assert_normalized_project(
        case,
        &project.netlist,
        &project.validation_yaml,
        &project.manifest_json,
    );
}

fn assert_normalized_project(
    case: &DemoImportCase,
    netlist: &str,
    validation_yaml: &str,
    manifest_json: &str,
) {
    assert!(
        netlist.starts_with("* Imported from schema schematic:"),
        "{} normalized netlist should preserve the generated import header",
        case.name
    );
    assert!(
        netlist.contains(case.expected_directive),
        "{} normalized netlist should retain the analysis directive",
        case.name
    );
    assert!(
        validation_yaml.contains("runs:") && validation_yaml.contains("checks: []"),
        "{} normalized project should include a validation run stub",
        case.name
    );
    assert!(
        manifest_json.contains("\"flavor\": \"schema\""),
        "{} normalized manifest should record the schema flavor",
        case.name
    );
    assert!(
        manifest_json.contains("\"import_report\": \"../import.json\""),
        "{} normalized manifest should point back to the import report",
        case.name
    );
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
