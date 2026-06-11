// Netlist parsing and normalization (extracted from lib.rs).
// Covers: parse_netlist, normalize_project, dependency resolution, diagnostics.

pub fn parse_netlist(input: &str, source: &str) -> OslResult<ImportReport> {
    let mut report = ImportReport {
        source: source.to_string(),
        flavor: detect_flavor(input),
        line_count: input.lines().count(),
        components: Vec::new(),
        directives: Vec::new(),
        includes: Vec::new(),
        diagnostics: Vec::new(),
    };

    for (line_number, line) in spice_logical_lines(input) {
        let Some(statement) = normalized_spice_statement(&line) else {
            continue;
        };
        if statement.starts_with('.') {
            parse_directive(line_number, &statement, &mut report);
        } else {
            parse_component(line_number, &statement, &mut report);
        }
    }

    if report.components.is_empty() {
        push_diagnostic(
            &mut report,
            1,
            ImportSeverity::Warning,
            "no_components",
            "netlist contains no component instances",
            "Check whether the file is a model library instead of a runnable netlist.",
        );
    }
    if !report.directives.iter().any(|directive| {
        matches!(
            directive.name.as_str(),
            ".tran" | ".ac" | ".dc" | ".op" | ".control"
        )
    }) {
        push_diagnostic(
            &mut report,
            1,
            ImportSeverity::Warning,
            "missing_analysis",
            "netlist has no obvious analysis directive",
            "Add .tran, .ac, .dc, .op, or a .control block before running simulation.",
        );
    }

    Ok(report)
}

fn parse_directive(line: usize, statement: &str, report: &mut ImportReport) {
    let name = statement
        .split_whitespace()
        .next()
        .unwrap_or(statement)
        .to_ascii_lowercase();
    report.directives.push(DirectiveSummary {
        line,
        name: name.clone(),
        text: statement.to_string(),
    });

    match name.as_str() {
        ".include" | ".inc" | ".lib" => {
            if let Some(path) = statement.split_whitespace().nth(1) {
                report.includes.push(IncludeSummary {
                    line,
                    path: path.trim_matches('"').to_string(),
                });
            }
        }
        ".step" | ".protect" | ".unprotect" | ".alter" => push_diagnostic(
            report,
            line,
            ImportSeverity::Warning,
            "dialect_directive",
            &format!(
                "{} is dialect-specific and may not run as-is in ngspice",
                name
            ),
            "Normalize this directive during import or move it into a verification sweep.",
        ),
        ".end" | ".tran" | ".ac" | ".dc" | ".op" | ".control" | ".endc" | ".model" | ".subckt"
        | ".ends" | ".param" | ".options" | ".option" => {}
        _ => push_diagnostic(
            report,
            line,
            ImportSeverity::Info,
            "unknown_directive",
            &format!("{} is not classified by the importer yet", name),
            "Keep this directive in the import report for compatibility review.",
        ),
    }
}

fn parse_component(line: usize, statement: &str, report: &mut ImportReport) {
    let tokens = statement.split_whitespace().collect::<Vec<_>>();
    if tokens.is_empty() {
        return;
    }
    let reference = tokens[0].to_string();
    let kind = component_kind(&reference);
    let (nodes, value, model, min_pin_count) = match kind {
        ComponentKind::Subcircuit => {
            let instance_tokens = tokens.iter().skip(1).copied().collect::<Vec<_>>();
            let model = instance_tokens.last().map(|model| model.to_string());
            let nodes = instance_tokens
                .iter()
                .take(instance_tokens.len().saturating_sub(1))
                .map(|node| node.to_string())
                .collect::<Vec<_>>();
            (nodes, model.clone(), model, 1)
        }
        _ => {
            let pin_count = expected_pin_count(kind);
            let nodes = tokens
                .iter()
                .skip(1)
                .take(pin_count)
                .map(|token| token.to_string())
                .collect::<Vec<_>>();
            let value = component_value_tail(statement, pin_count);
            let model = match kind {
                ComponentKind::Diode
                | ComponentKind::Bjt
                | ComponentKind::Mosfet
                | ComponentKind::Jfet => value
                    .as_deref()
                    .and_then(|value| value.split_whitespace().next())
                    .map(str::to_string),
                _ => None,
            };
            (nodes, value, model, pin_count)
        }
    };

    if nodes.len() < min_pin_count {
        push_diagnostic(
            report,
            line,
            ImportSeverity::Error,
            "component_too_few_nodes",
            &format!(
                "{} expects at least {} nodes but only {} were found",
                reference,
                min_pin_count,
                nodes.len()
            ),
            "Check the exported netlist line and symbol pin mapping.",
        );
    }

    report.components.push(ComponentSummary {
        line,
        reference,
        kind,
        nodes,
        value,
        model,
    });
}

fn component_value_tail(statement: &str, pin_count: usize) -> Option<String> {
    let mut parts = statement.splitn(pin_count + 2, char::is_whitespace);
    for _ in 0..=pin_count {
        parts.next()?;
    }
    parts
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn push_diagnostic(
    report: &mut ImportReport,
    line: usize,
    severity: ImportSeverity,
    code: &str,
    message: &str,
    suggestion: &str,
) {
    report.diagnostics.push(ImportDiagnostic {
        line,
        severity,
        code: code.to_string(),
        message: message.to_string(),
        suggestion: suggestion.to_string(),
    });
}

fn detect_flavor(input: &str) -> NetlistFlavor {
    let lowered = input.to_ascii_lowercase();
    if lowered.contains("eeschema") || lowered.contains("kicad") {
        NetlistFlavor::KiCad
    } else if lowered.contains("ltspice") {
        NetlistFlavor::Ltspice
    } else {
        NetlistFlavor::GenericSpice
    }
}

fn component_kind(reference: &str) -> ComponentKind {
    match reference
        .chars()
        .next()
        .map(|character| character.to_ascii_uppercase())
    {
        Some('R') => ComponentKind::Resistor,
        Some('C') => ComponentKind::Capacitor,
        Some('L') => ComponentKind::Inductor,
        Some('V') => ComponentKind::VoltageSource,
        Some('I') => ComponentKind::CurrentSource,
        Some('D') => ComponentKind::Diode,
        Some('Q') => ComponentKind::Bjt,
        Some('M') => ComponentKind::Mosfet,
        Some('J') => ComponentKind::Jfet,
        Some('X') => ComponentKind::Subcircuit,
        Some('B') | Some('E') | Some('G') | Some('F') | Some('H') => ComponentKind::Behavioral,
        _ => ComponentKind::Other,
    }
}

fn expected_pin_count(kind: ComponentKind) -> usize {
    match kind {
        ComponentKind::Bjt | ComponentKind::Jfet => 3,
        ComponentKind::Mosfet => 4,
        ComponentKind::Subcircuit => 2,
        _ => 2,
    }
}

fn normalize_signal_node(node: &str) -> String {
    node.trim()
        .trim_matches('"')
        .trim_matches('\'')
        .to_ascii_lowercase()
}

fn is_ground_node(node: &str) -> bool {
    matches!(node, "0" | "gnd" | "agnd" | "dgnd")
}


// Suggested checks, signal normalization, and project rewrite.
include!("netlist_suggest_impl.rs");

#[cfg(test)]
mod tests {
    use crate::ltspice_import::import_ltspice_asc;

    use super::{
        ComponentKind, ImportSeverity, NetlistFlavor, NormalizedDependency, parse_netlist,
        read_import_input,
    };
    use std::fs;
    use std::path::Path;

    #[test]
    fn parses_kicad_style_netlist_summary() {
        let input = r#"
* KiCad Eeschema generated SPICE netlist
.include "models.lib"
V1 in 0 DC 5
R1 in out 1k
C1 out 0 10n
XU1 in out vcc vee GOODAMP
.tran 1u 1m
.end
"#;

        let report = parse_netlist(input, "demo.cir").unwrap();

        assert_eq!(report.flavor, NetlistFlavor::KiCad);
        assert_eq!(report.component_count(), 4);
        assert_eq!(report.directive_count(), 3);
        assert_eq!(report.includes[0].path, "models.lib");
        assert_eq!(report.components[3].kind, ComponentKind::Subcircuit);
        assert_eq!(report.components[3].nodes, ["in", "out", "vcc", "vee"]);
        assert_eq!(report.components[3].model.as_deref(), Some("GOODAMP"));
        assert_eq!(report.error_count(), 0);
    }

    #[test]
    fn reports_missing_analysis() {
        let report = parse_netlist("R1 in out 1k\n", "missing.cir").unwrap();

        assert_eq!(report.warning_count(), 1);
        assert_eq!(report.diagnostics[0].code, "missing_analysis");
    }

    #[test]
    fn preserves_source_value_expressions_in_component_summary() {
        let report = parse_netlist(
            "V1 in 0 PULSE(0 1 0 1u 1u 10u 20u)\n.tran 1u 1m\n.end\n",
            "pulse.cir",
        )
        .unwrap();

        assert_eq!(
            report.components[0].value.as_deref(),
            Some("PULSE(0 1 0 1u 1u 10u 20u)")
        );
    }

    #[test]
    fn builds_normalized_import_project() {
        let source = "* KiCad netlist\nV1 in 0 DC 5\nR1 in out 1k\n.tran 1u 1m\n.end\n";
        let report = parse_netlist(source, "examples/kicad_import/kicad_rc.cir").unwrap();

        let project = report.normalized_project(source);

        assert_eq!(project.project_name, "kicad_rc");
        assert_eq!(project.netlist_path, "input.cir");
        assert!(project.netlist.ends_with('\n'));
        assert!(project.validation_yaml.contains("project: kicad_rc"));
        assert!(project.validation_yaml.contains("netlist: input.cir"));
        assert!(project.validation_yaml.contains("checks: []"));
        assert!(
            project
                .validation_yaml
                .contains("Suggested checks to customize")
        );
        assert!(project.validation_yaml.contains("signal: \"v(out)\""));
        assert!(project.validation_yaml.contains("signal: \"v(in)\""));
        assert!(project.validation_yaml.contains("signal: \"i(v1)\""));
        assert!(project.manifest_json.contains("\"flavor\": \"kicad\""));
        assert!(project.manifest_json.contains("\"suggested_signals\""));
        assert!(project.manifest_json.contains("\"signal\": \"v(out)\""));
        assert!(project.manifest_json.contains("\"signal\": \"v(in)\""));
        assert!(project.manifest_json.contains("\"signal\": \"i(v1)\""));
        assert!(project.manifest_json.contains("\"suggested_checks\""));
        assert!(
            project
                .manifest_json
                .contains("\"validation\": \"project.osl.yaml\"")
        );
    }

    #[test]
    fn suggests_import_checks_without_activating_them() {
        let source =
            "* imported netlist\nV1 in 0 DC 5\nR1 in out 1k\nC1 out 0 10n\n.tran 1u 1m\n.end\n";
        let report = parse_netlist(source, "imported.cir").unwrap();

        let signals = report.suggested_signals();
        let checks = report.suggested_checks();
        let project = report.normalized_project(source);

        assert_eq!(
            signals
                .iter()
                .map(|signal| signal.signal.as_str())
                .collect::<Vec<_>>(),
            ["v(in)", "v(out)", "i(v1)"]
        );
        assert_eq!(checks[0].signal, "v(out)");
        assert_eq!(checks[0].kind, "avg");
        assert_eq!(checks[1].signal, "v(in)");
        assert_eq!(checks[1].kind, "avg");
        assert_eq!(checks[2].signal, "i(v1)");
        assert_eq!(checks[2].kind, "rms");
        assert!(project.validation_yaml.contains("    checks: []\n"));
        assert!(!project.validation_yaml.contains("    checks:\n"));
    }

    #[test]
    fn rewrites_normalized_include_dependencies() {
        let source = "* KiCad netlist\n.include \"models.lib\"\nV1 in 0 DC 5\n.tran 1u 1m\n.end\n";
        let report = parse_netlist(source, "examples/kicad_import/kicad_with_model.cir").unwrap();
        let dependencies = vec![NormalizedDependency {
            source: "models.lib".to_string(),
            project_path: "models/models.lib".to_string(),
        }];

        let project = report.normalized_project_with_dependencies(source, &dependencies);

        assert!(project.netlist.contains(".include \"models/models.lib\""));
        assert!(project.manifest_json.contains("\"source\": \"models.lib\""));
        assert!(
            project
                .manifest_json
                .contains("\"project_path\": \"models/models.lib\"")
        );
        assert_eq!(project.dependencies.len(), 1);
    }

    #[test]
    fn discovers_kicad_project_netlist_from_directory_or_project_file() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let project_dir = workspace_root.join("examples/kicad_project");

        let from_dir = read_import_input(&project_dir).unwrap();
        let from_project_file =
            read_import_input(&project_dir.join("kicad_project.kicad_pro")).unwrap();

        assert_eq!(from_dir.report.flavor, NetlistFlavor::KiCad);
        assert_eq!(from_project_file.report.flavor, NetlistFlavor::KiCad);
        assert_eq!(from_dir.source_path, project_dir.join("kicad_project.cir"));
        assert_eq!(
            from_project_file.source_path,
            project_dir.join("kicad_project.cir")
        );
        assert!(
            from_dir
                .source_netlist
                .contains(".include \"models/ideal_diode.lib\"")
        );
    }

    #[test]
    fn imports_kicad_project_schematic_with_external_symbol_library() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let project_dir = std::env::temp_dir().join(format!(
            "nekospice_kicad_project_schematic_import_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&project_dir);
        fs::create_dir_all(&project_dir).unwrap();
        fs::copy(
            workspace_root.join("examples/kicad_schematic/neko_spice.kicad_sym"),
            project_dir.join("neko_spice.kicad_sym"),
        )
        .unwrap();
        fs::write(
            project_dir.join("sym-lib-table"),
            r#"(sym_lib_table
  (version 7)
  (lib (name "NekoSpice")(type "KiCad")(uri "${KIPRJMOD}/neko_spice.kicad_sym")(options "")(descr ""))
)"#,
        )
        .unwrap();
        fs::write(
            project_dir.join("project.kicad_pro"),
            r#"{"meta":{"filename":"project.kicad_pro","version":1},"project":{"name":"project"}}"#,
        )
        .unwrap();
        fs::write(
            project_dir.join("project.kicad_sch"),
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols)
  (wire (pts (xy 50.8 50.8) (xy 67.31 50.8)))
  (wire (pts (xy 72.39 50.8) (xy 88.9 50.8)))
  (wire (pts (xy 88.9 55.88) (xy 50.8 55.88)))
  (label "in" (at 50.8 50.8 0))
  (label "out" (at 88.9 50.8 0))
  (label "0" (at 69.85 55.88 0))
  (text ".tran 1u 1m" (at 45.72 38.1 0))
  (symbol
    (lib_id "NekoSpice:V")
    (at 50.8 53.34 0)
    (property "Reference" "V1" (at 48.26 57.15 0))
    (property "Value" "PULSE(0 1 0 1u 1u 10u 20u)" (at 45.72 60.96 0))
  )
  (symbol
    (lib_id "NekoSpice:R")
    (at 69.85 50.8 0)
    (property "Reference" "R1" (at 69.85 48.26 0))
    (property "Value" "1k" (at 69.85 53.34 0))
  )
  (symbol
    (lib_id "NekoSpice:C")
    (at 88.9 53.34 0)
    (property "Reference" "C1" (at 91.44 55.88 0))
    (property "Value" "100n" (at 91.44 58.42 0))
  )
)"#,
        )
        .unwrap();

        let from_dir = read_import_input(&project_dir).unwrap();
        let from_project_file = read_import_input(&project_dir.join("project.kicad_pro")).unwrap();

        assert_eq!(from_dir.source_path, project_dir.join("project.kicad_sch"));
        assert_eq!(
            from_project_file.source_path,
            project_dir.join("project.kicad_sch")
        );
        assert_eq!(from_dir.report.flavor, NetlistFlavor::KiCad);
        assert_eq!(from_dir.report.error_count(), 0);
        assert!(from_dir.source_netlist.contains("V1 in 0 PULSE"));
        assert!(from_dir.source_netlist.contains("R1 in out 1k"));
        assert!(from_dir.source_netlist.contains("C1 out 0 100n"));
        assert!(from_dir.source_netlist.contains(".tran 1u 1m"));
        assert_eq!(from_project_file.source_netlist, from_dir.source_netlist);

        let _ = fs::remove_dir_all(project_dir);
    }

    #[test]
    fn imports_kicad_hierarchical_project_fixture() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let project_dir = workspace_root.join("examples/kicad_hierarchical");

        let from_dir = read_import_input(&project_dir).unwrap();
        let from_project_file =
            read_import_input(&project_dir.join("kicad_hierarchical.kicad_pro")).unwrap();

        assert_eq!(
            from_dir.source_path,
            project_dir.join("kicad_hierarchical.kicad_sch")
        );
        assert_eq!(from_dir.report.flavor, NetlistFlavor::KiCad);
        assert_eq!(from_dir.report.error_count(), 0);
        assert_eq!(from_dir.report.compatibility_score(), 100);
        assert!(from_dir.source_netlist.contains("V1 in 0 DC 5"));
        assert!(from_dir.source_netlist.contains("RLOAD out 0 1k"));
        assert!(from_dir.source_netlist.contains("Rgain_stage_1 in out 2k"));
        assert!(from_dir.source_netlist.contains(".op"));
        assert!(
            !from_dir
                .source_netlist
                .contains("Unsupported KiCad hierarchical sheet")
        );
        assert_eq!(
            from_project_file.source_path,
            project_dir.join("kicad_hierarchical.kicad_sch")
        );
        assert_eq!(from_project_file.source_netlist, from_dir.source_netlist);
    }

    #[test]
    fn prefers_kicad_project_named_schematic_over_other_sheets() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let project_dir = std::env::temp_dir().join(format!(
            "nekospice_kicad_project_source_select_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&project_dir);
        fs::create_dir_all(&project_dir).unwrap();
        fs::copy(
            workspace_root.join("examples/kicad_schematic/neko_spice.kicad_sym"),
            project_dir.join("neko_spice.kicad_sym"),
        )
        .unwrap();
        fs::write(
            project_dir.join("sym-lib-table"),
            r#"(sym_lib_table
  (version 7)
  (lib (name "NekoSpice")(type "KiCad")(uri "${KIPRJMOD}/neko_spice.kicad_sym")(options "")(descr ""))
)"#,
        )
        .unwrap();
        fs::write(
            project_dir.join("root_project.kicad_pro"),
            r#"{
  "meta": { "filename": "root_project.kicad_pro", "version": 1 },
  "project": { "name": "root_project" },
  "sheets": [
    [ "root-sheet-uuid", "Root" ],
    [ "child-sheet-uuid", "aaa_sheet" ]
  ],
  "text_variables": {}
}"#,
        )
        .unwrap();
        fs::write(
            project_dir.join("aaa_sheet.kicad_sch"),
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols)
  (wire (pts (xy 7 10) (xy 10 10)))
  (wire (pts (xy 15.08 10) (xy 18 10)))
  (label "child" (at 7 10 0))
  (label "0" (at 18 10 0))
  (text ".op" (at 5 5 0))
  (symbol
    (lib_id "NekoSpice:R")
    (at 12.54 10 0)
    (property "Reference" "Rchild" (at 12.54 8 0))
    (property "Value" "9k" (at 12.54 12 0))
  )
)"#,
        )
        .unwrap();
        fs::write(
            project_dir.join("root_project.kicad_sch"),
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols)
  (wire (pts (xy 7 10) (xy 10 10)))
  (wire (pts (xy 15.08 10) (xy 18 10)))
  (label "in" (at 7 10 0))
  (label "0" (at 18 10 0))
  (text ".op" (at 5 5 0))
  (symbol
    (lib_id "NekoSpice:R")
    (at 12.54 10 0)
    (property "Reference" "R1" (at 12.54 8 0))
    (property "Value" "2k" (at 12.54 12 0))
  )
)"#,
        )
        .unwrap();

        let from_dir = read_import_input(&project_dir).unwrap();
        let from_project_file =
            read_import_input(&project_dir.join("root_project.kicad_pro")).unwrap();

        assert_eq!(
            from_dir.source_path,
            project_dir.join("root_project.kicad_sch")
        );
        assert_eq!(
            from_project_file.source_path,
            project_dir.join("root_project.kicad_sch")
        );
        assert!(from_dir.source_netlist.contains("R1 in 0 2k"));
        assert!(!from_dir.source_netlist.contains("Rchild"));
        assert_eq!(from_project_file.source_netlist, from_dir.source_netlist);

        let _ = fs::remove_dir_all(project_dir);
    }

    #[test]
    fn imports_kicad_missing_child_sheet_with_diagnostic() {
        let project_dir = std::env::temp_dir().join(format!(
            "nekospice_kicad_hierarchical_sheet_import_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&project_dir);
        fs::create_dir_all(&project_dir).unwrap();
        let schematic_path = project_dir.join("hierarchical.kicad_sch");
        fs::write(
            &schematic_path,
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (wire (pts (xy 5 5) (xy 10 5)))
  (label "0" (at 5 5 0))
  (text ".op" (at 5 2 0))
  (sheet
    (at 20 10)
    (size 15 10)
    (property "Sheetname" "gain_stage" (at 20 9 0))
    (property "Sheetfile" "gain_stage.kicad_sch" (at 20 21 0))
    (pin "in" input (at 20 15 180))
    (pin "out" output (at 35 15 0))
  )
)"#,
        )
        .unwrap();

        let input = read_import_input(&schematic_path).unwrap();

        assert_eq!(input.report.flavor, NetlistFlavor::KiCad);
        assert!(input.report.error_count() >= 1);
        assert!(input.report.compatibility_score() < 100);
        assert!(input.report.diagnostics.iter().any(|diagnostic| {
            diagnostic.severity == ImportSeverity::Error
                && diagnostic.code == "kicad-missing-child-sheet"
        }));

        let _ = fs::remove_dir_all(project_dir);
    }

    #[test]
    fn imports_kicad_hierarchical_sheet_by_expanding_child_schematic() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let project_dir = std::env::temp_dir().join(format!(
            "nekospice_kicad_hierarchical_sheet_expand_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&project_dir);
        fs::create_dir_all(&project_dir).unwrap();
        fs::copy(
            workspace_root.join("examples/kicad_schematic/neko_spice.kicad_sym"),
            project_dir.join("neko_spice.kicad_sym"),
        )
        .unwrap();
        fs::write(
            project_dir.join("sym-lib-table"),
            r#"(sym_lib_table
  (version 7)
  (lib (name "NekoSpice")(type "KiCad")(uri "${KIPRJMOD}/neko_spice.kicad_sym")(options "")(descr ""))
)"#,
        )
        .unwrap();
        fs::write(
            project_dir.join("root.kicad_sch"),
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols)
  (wire (pts (xy 10 10) (xy 25 10)))
  (wire (pts (xy 10 15.08) (xy 60 15.08)))
  (wire (pts (xy 45 10) (xy 50 10)))
  (wire (pts (xy 55.08 10) (xy 60 10)))
  (label "in" (at 25 10 0))
  (label "out" (at 45 10 0))
  (label "0" (at 60 15.08 0))
  (label "0" (at 60 10 0))
  (symbol
    (lib_id "NekoSpice:V")
    (at 10 12.54 0)
    (property "Reference" "V1" (at 8 16 0))
    (property "Value" "DC 5" (at 8 18 0))
    (pin "1" (uuid "10000000-0000-0000-0000-000000000001"))
    (pin "2" (uuid "10000000-0000-0000-0000-000000000002"))
  )
  (sheet
    (at 25 5)
    (size 20 10)
    (property "Sheetname" "gain_stage" (at 25 4 0))
    (property "Sheetfile" "gain_stage.kicad_sch" (at 25 16 0))
    (pin "in" input (at 25 10 180))
    (pin "out" output (at 45 10 0))
  )
  (symbol
    (lib_id "NekoSpice:R")
    (at 52.54 10 0)
    (property "Reference" "RLOAD" (at 52.54 8 0))
    (property "Value" "1k" (at 52.54 12 0))
    (pin "1" (uuid "10000000-0000-0000-0000-000000000003"))
    (pin "2" (uuid "10000000-0000-0000-0000-000000000004"))
  )
)"#,
        )
        .unwrap();
        fs::write(
            project_dir.join("gain_stage.kicad_sch"),
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols)
  (wire (pts (xy 7 10) (xy 10 10)))
  (wire (pts (xy 15.08 10) (xy 18 10)))
  (hierarchical_label "in" (at 7 10 0))
  (hierarchical_label "out" (at 18 10 0))
  (text ".op" (at 5 3 0))
  (symbol
    (lib_id "NekoSpice:R")
    (at 12.54 10 0)
    (property "Reference" "R1" (at 12.54 8 0))
    (property "Value" "2k" (at 12.54 12 0))
    (pin "1" (uuid "20000000-0000-0000-0000-000000000001"))
    (pin "2" (uuid "20000000-0000-0000-0000-000000000002"))
  )
)"#,
        )
        .unwrap();

        let input = read_import_input(&project_dir.join("root.kicad_sch")).unwrap();

        assert_eq!(input.report.flavor, NetlistFlavor::KiCad);
        assert_eq!(input.report.error_count(), 0);
        assert_eq!(input.report.compatibility_score(), 100);
        assert!(input.source_netlist.contains("V1 in 0 DC 5"));
        assert!(input.source_netlist.contains("Rgain_stage_1 in out 2k"));
        assert!(input.source_netlist.contains("RLOAD out 0 1k"));
        assert!(input.source_netlist.contains(".op"));
        assert!(
            !input
                .source_netlist
                .contains("Unsupported KiCad hierarchical sheet")
        );
        assert!(
            !input
                .report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "kicad-hierarchical-sheet-unsupported")
        );

        let _ = fs::remove_dir_all(project_dir);
    }

    #[test]
    fn imports_kicad_schematic_to_runnable_netlist() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let input =
            read_import_input(&workspace_root.join("examples/kicad_schematic/rc.kicad_sch"))
                .unwrap();

        assert_eq!(input.report.flavor, NetlistFlavor::KiCad);
        assert_eq!(input.report.error_count(), 0);
        assert!(input.source_netlist.contains("V1 in 0 PULSE"));
        assert!(input.source_netlist.contains("R1 in out 1k"));
        assert!(input.source_netlist.contains("C1 out 0 100n"));
        assert!(input.source_netlist.contains(".tran 1u 1m"));

        let project = input.report.normalized_project(&input.source_netlist);
        assert!(project.validation_yaml.contains("signal: \"v(out)\""));
        assert!(project.manifest_json.contains("\"flavor\": \"kicad\""));
    }

    #[test]
    fn imports_ltspice_asc_to_runnable_netlist() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let input =
            read_import_input(&workspace_root.join("examples/ltspice_import/ltspice_rc.asc"))
                .unwrap();

        assert_eq!(input.report.flavor, NetlistFlavor::Ltspice);
        assert_eq!(input.report.error_count(), 0);
        assert!(input.source_netlist.contains("V1 n001 0 PULSE"));
        assert!(input.source_netlist.contains("R1 out n001 1k"));
        assert!(input.source_netlist.contains("C1 out 0 100n"));
        assert!(input.source_netlist.contains(".tran 1u 500u"));
        assert!(input.source_netlist.contains("out"));

        let project = input.report.normalized_project(&input.source_netlist);
        assert!(project.validation_yaml.contains("signal: \"v(out)\""));
        assert!(project.manifest_json.contains("\"flavor\": \"ltspice\""));
    }

    #[test]
    fn imports_ltspice_asc_with_local_asy_symbol() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let input =
            read_import_input(&workspace_root.join("examples/ltspice_import/ltspice_subckt.asc"))
                .unwrap();

        assert_eq!(input.report.flavor, NetlistFlavor::Ltspice);
        assert_eq!(input.report.error_count(), 0);
        assert!(input.source_netlist.contains("V1 n001 0 DC 1"));
        assert!(input.source_netlist.contains("XU1 n001 out 0 gain_block"));
        assert!(input.source_netlist.contains(".include \"gain_block.lib\""));

        let project = input.report.normalized_project(&input.source_netlist);
        assert!(project.validation_yaml.contains("signal: \"v(out)\""));
        assert!(project.validation_yaml.contains("signal: \"i(v1)\""));
    }

    #[test]
    fn imports_ltspice_asc_with_symbol_search_dir() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let input = read_import_input(
            &workspace_root.join("examples/ltspice_import/ltspice_sym_search.asc"),
        )
        .unwrap();

        assert_eq!(input.report.flavor, NetlistFlavor::Ltspice);
        assert_eq!(input.report.error_count(), 0);
        assert!(input.source_netlist.contains("XU1 n001 out 0 gain_block"));
        assert!(!input.report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "ltspice_unsupported_symbol"
                || diagnostic.code == "ltspice_symbol_no_pins"
        }));
    }

    #[test]
    fn imports_ltspice_bjt_builtin_symbol() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let input =
            read_import_input(&workspace_root.join("examples/ltspice_import/ltspice_bjt.asc"))
                .unwrap();

        assert_eq!(input.report.flavor, NetlistFlavor::Ltspice);
        assert_eq!(input.report.error_count(), 0);
        assert!(input.source_netlist.contains("Q1 vcc in 0 QTEST"));
        assert!(input.source_netlist.contains(".model QTEST NPN"));
        assert!(!input.report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "ltspice_unsupported_symbol"
                || diagnostic.code == "ltspice_unmapped_pin"
        }));
    }

    #[test]
    fn imports_ltspice_controlled_source_builtin_symbol() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let input =
            read_import_input(&workspace_root.join("examples/ltspice_import/ltspice_vcvs.asc"))
                .unwrap();

        assert_eq!(input.report.flavor, NetlistFlavor::Ltspice);
        assert_eq!(input.report.error_count(), 0);
        assert!(input.source_netlist.contains("E1 out 0 n001 0 2"));
        assert!(!input.report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "ltspice_unsupported_symbol"
                || diagnostic.code == "ltspice_unmapped_pin"
        }));
    }

    #[test]
    fn reports_unsupported_ltspice_asc_symbol() {
        let input = r#"
Version 4
SHEET 1 880 680
FLAG 0 0 0
SYMBOL opamp 0 0 R0
SYMATTR InstName U1
SYMATTR Value OPAMP
TEXT 0 96 Left 2 !.op
"#;

        let imported = import_ltspice_asc(input, "unsupported.asc", Path::new("."));

        assert!(imported.netlist.contains(".op"));
        assert!(
            imported
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "ltspice_unsupported_symbol"
                    && diagnostic.severity == ImportSeverity::Error)
        );
    }
}
