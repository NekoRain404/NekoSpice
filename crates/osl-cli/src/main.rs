//! NekoSpice CLI — command-line interface for simulation, verification, and analysis.

use osl_core::{
    OslError, OslResult, ParameterOverride, RunMetadata, RunStatus, make_run_id, read_text,
    write_text,
};
use osl_kicad::{
    KicadCanvasScene, KicadSymbolLibraryIndexQuery, read_kicad_project,
    read_kicad_schematic_with_libraries, read_kicad_symbol_library,
    read_kicad_symbol_library_index, read_kicad_symbol_library_table, write_kicad_schematic,
    write_kicad_symbol_library,
};
use osl_model::{ModelCheckOptions, ModelCheckReport};
use osl_netlist::{ImportReport, NormalizedDependency, read_import_input};
use osl_render::render_kicad_scene_svg;
use osl_report::{
    CheckResult, VerifyReport, VerifyRunResult, report_css, write_bench_report_bundle,
    write_json_html_report_bundle, write_report_directory_html, write_verify_report_bundle,
};
use osl_sim::{NgspiceCliBackend, SimulatorBackend, finalize_run_artifacts};
use osl_waveform::{
    MeasurementKind, WaveformSummary, WaveformViewportQuery, measure, read_ngspice_raw,
};
use serde::Deserialize;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::{Arc, Mutex};
use std::thread;

mod kicad_edit;

use kicad_edit::{parse_kicad_edit_ops, parse_kicad_point};

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    match run_cli() {
        Ok(exit_code) => process::exit(exit_code),
        Err(error) => {
            eprintln!("error: {error}");
            process::exit(1);
        }
    }
}

fn run_cli() -> OslResult<i32> {
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        print_help();
        return Ok(0);
    }

    let command = args.remove(0);
    match command.as_str() {
        "--help" | "-h" | "help" => {
            print_help();
            Ok(0)
        }
        "--version" | "-V" | "version" => {
            println!("osl {VERSION}");
            Ok(0)
        }
        "run" => run_command(&args),
        "verify" => verify_command(&args),
        "bench" => bench_command(&args),
        "model-check" => model_check_command(&args),
        "import" => import_command(&args),
        "kicad-inspect" => kicad_inspect_command(&args),
        "kicad-select" => kicad_select_command(&args),
        "kicad-check" => kicad_check_command(&args),
        "kicad-export" => kicad_export_command(&args),
        "kicad-edit" => kicad_edit_command(&args),
        "kicad-render" => kicad_render_command(&args),
        "waveform" => waveform_command(&args),
        "report" => report_command(&args),
        unknown => Err(OslError::InvalidInput(format!(
            "unknown command '{unknown}'. Run 'osl help'."
        ))),
    }
}

fn print_help() {
    println!(
        "\
osl {VERSION}

Usage:
  osl run <netlist.cir> [--output <dir>] [--ngspice <path>]
  osl verify <project.osl.yaml> [--output <dir>] [--ngspice <path>] [--jobs <n>]
  osl bench <directory> [--output <dir>] [--ngspice <path>]
  osl model-check <netlist-or-directory> [--output <dir>] [--symbol <ltspice.asy>]
  osl import <spice-netlist-or-ltspice.asc-or-kicad_sch-or-kicad-project> [--output <dir>]
  osl kicad-inspect <file.kicad_pro-or-file.kicad_sch-or-file.kicad_sym-or-sym-lib-table> [--canvas] [--index] [--output <file>]
  osl kicad-select <file.kicad_sch> <x,y> [--output <file>]
  osl kicad-check <file.kicad_sch> [--output <file>]
  osl kicad-export <file.kicad_sch-or-file.kicad_sym> --output <file>
  osl kicad-edit <file.kicad_sch> --output <file.kicad_sch> [--library <file.kicad_sym>] <ops...>
      delete-item:<uuid>
      move-item:<uuid>:<dx,dy>
      configure-symbol:<reference>[:unit=<n>][:body-style=<n|none>][:mirror=<x|y|xy|none>][:alt=<pin>=<alternate>[,<pin>=<alternate>...]]
      move-symbol:<reference>:<x,y>[:rotation]
      set-property:<reference>:<name>=<value>[:x,y[,rotation]]
      place-symbol:<lib_id>:<reference>:<value>:<x,y[,rotation]>[:unit=<n>][:body-style=<n>][:alt=<pin>=<alternate>[,<pin>=<alternate>...]]
      set-simulation-directive:<kind>:<body>[:x,y[,rotation]][:uuid=<uuid>]
  osl kicad-render <file.kicad_sch-or-file.kicad_sym> [--symbol <name>] [--unit <n>] [--body-style <n>] --output <file.svg>
  osl waveform <waveform.raw> --signal <name> [--from <time>] [--to <time>] [--points <n>] [--output <file>]
  osl report <run-or-verify-dir>
  osl --version

Three-day target:
  batch ngspice runs, reproducible run metadata, HTML/JSON/Markdown/JUnit reports, and CI-friendly pass/fail output.
"
    );
}

fn run_command(args: &[String]) -> OslResult<i32> {
    let input = positional(args, 0, "missing netlist path for 'osl run'")?;
    let ngspice = flag_value(args, "--ngspice").unwrap_or_else(|| "ngspice".to_string());
    let output_dir = flag_value(args, "--output")
        .map(PathBuf::from)
        .unwrap_or_else(|| default_run_dir(input));

    let backend = NgspiceCliBackend::new(ngspice);
    let mut metadata = backend.run(Path::new(input), &output_dir)?;
    let metadata_output_dir = PathBuf::from(&metadata.output_dir);
    finalize_run_output(&metadata_output_dir, &mut metadata)?;

    println!(
        "{} {} in {} ms -> {}",
        metadata.status.as_str().to_uppercase(),
        input,
        metadata.duration_ms,
        output_dir.display()
    );

    Ok(if metadata.status == RunStatus::Passed {
        0
    } else {
        2
    })
}

fn verify_command(args: &[String]) -> OslResult<i32> {
    let config_path = positional(args, 0, "missing config path for 'osl verify'")?;
    let ngspice = flag_value(args, "--ngspice").unwrap_or_else(|| "ngspice".to_string());
    let config = VerifyConfig::parse(Path::new(config_path))?;
    let jobs = flag_value(args, "--jobs")
        .map(|value| parse_jobs(&value))
        .transpose()?
        .unwrap_or(1);
    let output_dir = flag_value(args, "--output")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("reports").join(make_run_id(&config.project)));

    fs::create_dir_all(&output_dir)
        .map_err(|err| OslError::io(format!("create {}", output_dir.display()), err))?;

    let mut tasks = Vec::new();
    for item in &config.runs {
        for case in item.expand_cases()? {
            let run_dir = output_dir.join("runs").join(sanitize_name(&case.name));
            tasks.push(VerifyTask {
                index: tasks.len(),
                name: case.name,
                netlist: item.netlist.display().to_string(),
                netlist_path: item.netlist.clone(),
                run_dir: run_dir.display().to_string(),
                parameters: case.parameters,
                checks: item.checks.clone(),
            });
        }
    }
    let results = run_verify_tasks(tasks, &ngspice, jobs)?;

    let report = VerifyReport {
        project: config.project,
        results,
    };
    write_verify_report_bundle(&output_dir, &report)?;

    println!(
        "verify {}: {} passed, {} failed, jobs={} -> {}",
        report.project,
        report.passed_count(),
        report.failed_count(),
        jobs,
        output_dir.display()
    );

    Ok(if report.failed_count() == 0 { 0 } else { 2 })
}

fn bench_command(args: &[String]) -> OslResult<i32> {
    let root = positional(args, 0, "missing directory for 'osl bench'")?;
    let ngspice = flag_value(args, "--ngspice").unwrap_or_else(|| "ngspice".to_string());
    let output_dir = flag_value(args, "--output")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("bench-results").join(make_run_id("bench")));

    let circuits = find_circuits(Path::new(root))?;
    if circuits.is_empty() {
        return Err(OslError::InvalidInput(format!(
            "no .cir files found under {}",
            root
        )));
    }

    fs::create_dir_all(&output_dir)
        .map_err(|err| OslError::io(format!("create {}", output_dir.display()), err))?;
    let backend = NgspiceCliBackend::new(ngspice);
    let mut results = Vec::new();

    for (index, circuit) in circuits.into_iter().enumerate() {
        let name = circuit
            .file_stem()
            .and_then(|name| name.to_str())
            .map(sanitize_name)
            .unwrap_or_else(|| "circuit".to_string());
        let run_dir = output_dir.join(&name);
        let mut metadata = backend.run(&circuit, &run_dir)?;
        let metadata_output_dir = PathBuf::from(&metadata.output_dir);
        finalize_run_output(&metadata_output_dir, &mut metadata)?;
        results.push(VerifyRunResult {
            index,
            name,
            netlist: circuit.display().to_string(),
            run_dir: run_dir.display().to_string(),
            metadata,
            parameters: Vec::new(),
            checks: Vec::new(),
        });
    }

    let report = VerifyReport {
        project: "bench".to_string(),
        results,
    };
    write_bench_report_bundle(&output_dir, &report)?;

    println!(
        "bench: {} passed, {} failed -> {}",
        report.passed_count(),
        report.failed_count(),
        output_dir.display()
    );

    Ok(if report.failed_count() == 0 { 0 } else { 2 })
}

fn model_check_command(args: &[String]) -> OslResult<i32> {
    let input = positional(args, 0, "missing path for 'osl model-check'")?;
    let symbol_path = flag_value(args, "--symbol").map(PathBuf::from);
    let output_dir = flag_value(args, "--output")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("reports").join(make_run_id("model-check")));

    fs::create_dir_all(&output_dir)
        .map_err(|err| OslError::io(format!("create {}", output_dir.display()), err))?;

    let options = ModelCheckOptions { symbol_path };
    let report = ModelCheckReport::scan_with_options(Path::new(input), &options)?;
    write_json_html_report_bundle(
        &output_dir,
        "model-check.json",
        &report.to_json(),
        &report.to_html(report_css()),
    )?;

    println!(
        "model-check: {} files, {} subckts, {} models, {} diagnostics ({} errors, {} warnings) -> {}",
        report.files.len(),
        report.subckts.len(),
        report.models.len(),
        report.diagnostics.len(),
        report.error_count(),
        report.warning_count(),
        output_dir.display()
    );

    Ok(if report.error_count() == 0 { 0 } else { 2 })
}

fn import_command(args: &[String]) -> OslResult<i32> {
    let input = positional(args, 0, "missing netlist path for 'osl import'")?;
    let output_dir = flag_value(args, "--output")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("reports").join(make_run_id("import")));

    fs::create_dir_all(&output_dir)
        .map_err(|err| OslError::io(format!("create {}", output_dir.display()), err))?;

    let input_path = Path::new(input);
    let import = read_import_input(input_path)?;
    let source_netlist = import.source_netlist;
    let source_path = import.source_path;
    let report = import.report;
    write_json_html_report_bundle(
        &output_dir,
        "import.json",
        &report.to_json(),
        &report.to_html(report_css()),
    )?;
    let project_dir = output_dir.join("project");
    let dependencies = copy_import_dependencies(&report, &source_path, &project_dir)?;
    let project = report.normalized_project_with_dependencies(&source_netlist, &dependencies);
    write_text(&project_dir.join(&project.netlist_path), &project.netlist)?;
    write_text(
        &project_dir.join(&project.validation_path),
        &project.validation_yaml,
    )?;
    write_text(
        &project_dir.join(&project.manifest_path),
        &project.manifest_json,
    )?;

    println!(
        "import: flavor={}, components={}, symbols={}, directives={}, score={} -> {} (project: {})",
        report.flavor.as_str(),
        report.component_count(),
        report.symbol_count(),
        report.directive_count(),
        report.compatibility_score(),
        output_dir.display(),
        project_dir.join(&project.validation_path).display()
    );

    Ok(if report.error_count() == 0 { 0 } else { 2 })
}

fn waveform_command(args: &[String]) -> OslResult<i32> {
    let input = positional(args, 0, "missing raw path for 'osl waveform'")?;
    let signal = flag_value(args, "--signal").ok_or_else(|| {
        OslError::InvalidInput("missing --signal <name> for 'osl waveform'".to_string())
    })?;
    let max_points = flag_value(args, "--points")
        .map(|value| parse_positive_usize(&value, "--points"))
        .transpose()?
        .unwrap_or(500);
    let from = flag_value(args, "--from")
        .map(|value| parse_number(&value, "waveform --from"))
        .transpose()?;
    let to = flag_value(args, "--to")
        .map(|value| parse_number(&value, "waveform --to"))
        .transpose()?;
    let output = flag_value(args, "--output");

    let waveform = read_ngspice_raw(Path::new(input))?;
    let query = WaveformViewportQuery::new(signal, max_points).with_window(from, to);
    let envelope = waveform.viewport_envelope(&query)?;
    let json = envelope.to_json();

    if let Some(output) = output {
        write_text(Path::new(&output), &json)?;
        println!(
            "waveform: signal={}, buckets={}, source_points={} -> {}",
            envelope.signal,
            envelope.buckets.len(),
            envelope.source_points,
            output
        );
    } else {
        print!("{json}");
    }

    Ok(0)
}

include!("cli_kicad.rs");
include!("cli_verify.rs");

#[cfg(test)]
mod tests {
    use crate::kicad_edit::{parse_kicad_edit_ops, parse_kicad_point};

    use super::{
        SweepDimension, VerifyConfig, VerifyRun, flag_value, has_flag, parse_number,
        parse_positive_u32, positionals,
    };
    use osl_kicad::{
        KicadLabelKind, KicadSchematicEdit, KicadSimulationDirectiveKind,
        parse_kicad_symbol_library,
    };
    use std::path::PathBuf;

    #[test]
    fn expands_sweep_cartesian_product() {
        let run = VerifyRun {
            name: "case".to_string(),
            netlist: PathBuf::from("demo.cir"),
            sweep: vec![
                SweepDimension {
                    name: "vin".to_string(),
                    values: vec![9.0, 12.0],
                },
                SweepDimension {
                    name: "load".to_string(),
                    values: vec![1.0, 2.0],
                },
            ],
            checks: Vec::new(),
        };

        let cases = run.expand_cases().unwrap();

        assert_eq!(cases.len(), 4);
        assert_eq!(cases[0].name, "case__vin=9__load=1");
        assert_eq!(cases[3].parameters[0].name, "vin");
        assert_eq!(cases[3].parameters[0].value, 12.0);
        assert_eq!(cases[3].parameters[1].name, "load");
        assert_eq!(cases[3].parameters[1].value, 2.0);
    }

    #[test]
    fn parses_spice_numeric_suffixes() {
        assert_close(parse_number("3ms", "test").unwrap(), 0.003);
        assert_close(parse_number("50us", "test").unwrap(), 0.00005);
        assert_close(parse_number("1k", "test").unwrap(), 1000.0);
        assert_close(parse_number("2Meg", "test").unwrap(), 2_000_000.0);
    }

    #[test]
    fn parses_kicad_edit_positionals_after_output_flag() {
        let args = [
            "input.kicad_sch",
            "--output",
            "edited.kicad_sch",
            "move-symbol:R1:73.66,50.8",
            "move-item:22222222-2222-2222-2222-222222222222:2.54,-1.27",
            "set-property:R1:Value=2k",
            "add-bus:88.9,38.1;101.6,38.1",
            "add-bus-entry:101.6,38.1:2.54,-2.54",
            "add-junction:88.9,45.72",
            "add-no-connect:101.6,45.72",
            "add-global-label:sense:88.9,45.72",
            "set-simulation-directive:tran:2u 2m:30.48,20.32:uuid=aaaaaaaa-0000-4000-8000-000000000001",
            "delete-item:22222222-2222-2222-2222-222222222222",
        ]
        .iter()
        .map(|value| value.to_string())
        .collect::<Vec<_>>();

        assert_eq!(
            positionals(&args),
            vec![
                "input.kicad_sch",
                "move-symbol:R1:73.66,50.8",
                "move-item:22222222-2222-2222-2222-222222222222:2.54,-1.27",
                "set-property:R1:Value=2k",
                "add-bus:88.9,38.1;101.6,38.1",
                "add-bus-entry:101.6,38.1:2.54,-2.54",
                "add-junction:88.9,45.72",
                "add-no-connect:101.6,45.72",
                "add-global-label:sense:88.9,45.72",
                "set-simulation-directive:tran:2u 2m:30.48,20.32:uuid=aaaaaaaa-0000-4000-8000-000000000001",
                "delete-item:22222222-2222-2222-2222-222222222222",
            ]
        );

        let edits = parse_kicad_edit_ops(&args, &[]).unwrap();
        assert_eq!(edits.len(), 10);
        match &edits[0] {
            KicadSchematicEdit::MoveSymbol { reference, to, .. } => {
                assert_eq!(reference, "R1");
                assert_close(to.x, 73.66);
            }
            edit => panic!("expected move-symbol edit, got {edit:?}"),
        }
        match &edits[1] {
            KicadSchematicEdit::MoveItem { uuid, delta } => {
                assert_eq!(uuid, "22222222-2222-2222-2222-222222222222");
                assert_close(delta.x, 2.54);
                assert_close(delta.y, -1.27);
            }
            edit => panic!("expected move-item edit, got {edit:?}"),
        }
        match &edits[3] {
            KicadSchematicEdit::AddBus { points, .. } => {
                assert_eq!(points.len(), 2);
                assert_close(points[0].x, 88.9);
                assert_close(points[1].y, 38.1);
            }
            edit => panic!("expected add-bus edit, got {edit:?}"),
        }
        match &edits[4] {
            KicadSchematicEdit::AddBusEntry { at, size, .. } => {
                assert_close(at.x, 101.6);
                assert_close(at.y, 38.1);
                assert_close(size.width, 2.54);
                assert_close(size.height, -2.54);
            }
            edit => panic!("expected add-bus-entry edit, got {edit:?}"),
        }
        match &edits[5] {
            KicadSchematicEdit::AddJunction { at, .. } => {
                assert_close(at.x, 88.9);
                assert_close(at.y, 45.72);
            }
            edit => panic!("expected add-junction edit, got {edit:?}"),
        }
        match &edits[6] {
            KicadSchematicEdit::AddNoConnect { at, .. } => {
                assert_close(at.x, 101.6);
                assert_close(at.y, 45.72);
            }
            edit => panic!("expected add-no-connect edit, got {edit:?}"),
        }
        match &edits[7] {
            KicadSchematicEdit::AddLabel { text, kind, .. } => {
                assert_eq!(text, "sense");
                assert_eq!(*kind, KicadLabelKind::Global);
            }
            edit => panic!("expected add-label edit, got {edit:?}"),
        }
        match &edits[8] {
            KicadSchematicEdit::SetSimulationDirective {
                kind,
                body,
                at,
                uuid,
            } => {
                assert_eq!(*kind, KicadSimulationDirectiveKind::Tran);
                assert_eq!(body, "2u 2m");
                assert_close(at.unwrap().x, 30.48);
                assert_eq!(
                    uuid.as_deref(),
                    Some("aaaaaaaa-0000-4000-8000-000000000001")
                );
            }
            edit => panic!("expected set-simulation-directive edit, got {edit:?}"),
        }
        match &edits[9] {
            KicadSchematicEdit::DeleteItem { uuid } => {
                assert_eq!(uuid, "22222222-2222-2222-2222-222222222222");
            }
            edit => panic!("expected delete-item edit, got {edit:?}"),
        }
    }

    #[test]
    fn parses_kicad_inspect_index_flag_without_extra_positionals() {
        let args = [
            "sym-lib-table".to_string(),
            "--index".to_string(),
            "--query".to_string(),
            "opamp".to_string(),
            "--library".to_string(),
            "Device".to_string(),
            "--footprint".to_string(),
            "Package_SO:SOIC-8".to_string(),
            "--output".to_string(),
            "symbol_index.json".to_string(),
        ];

        assert!(has_flag(&args, "--index"));
        assert_eq!(positionals(&args), vec!["sym-lib-table"]);
        assert_eq!(flag_value(&args, "--query"), Some("opamp".to_string()));
        assert_eq!(flag_value(&args, "--library"), Some("Device".to_string()));
        assert_eq!(
            flag_value(&args, "--footprint"),
            Some("Package_SO:SOIC-8".to_string())
        );
        assert_eq!(
            flag_value(&args, "--output"),
            Some("symbol_index.json".to_string())
        );
    }

    #[test]
    fn parses_kicad_select_point_after_output_flag() {
        let args = [
            "input.kicad_sch".to_string(),
            "88.9,50.8".to_string(),
            "--output".to_string(),
            "hits.json".to_string(),
        ];

        assert_eq!(positionals(&args), vec!["input.kicad_sch", "88.9,50.8"]);
        assert_eq!(flag_value(&args, "--output"), Some("hits.json".to_string()));
        let point = parse_kicad_point(positionals(&args)[1], "select point").unwrap();
        assert_close(point.x, 88.9);
        assert_close(point.y, 50.8);
    }

    #[test]
    fn parses_kicad_render_symbol_preview_flags_without_extra_positionals() {
        let args = [
            "library.kicad_sym".to_string(),
            "--symbol".to_string(),
            "Device:R".to_string(),
            "--unit".to_string(),
            "2".to_string(),
            "--body-style".to_string(),
            "1".to_string(),
            "--output".to_string(),
            "preview.svg".to_string(),
        ];

        assert_eq!(positionals(&args), vec!["library.kicad_sym"]);
        assert_eq!(flag_value(&args, "--symbol"), Some("Device:R".to_string()));
        assert_eq!(
            flag_value(&args, "--unit")
                .map(|value| parse_positive_u32(&value, "--unit"))
                .transpose()
                .unwrap(),
            Some(2)
        );
        assert_eq!(
            flag_value(&args, "--body-style")
                .map(|value| parse_positive_u32(&value, "--body-style"))
                .transpose()
                .unwrap(),
            Some(1)
        );
        assert_eq!(
            flag_value(&args, "--output"),
            Some("preview.svg".to_string())
        );
    }

    #[test]
    fn parses_kicad_place_symbol_edit_with_library_definition() {
        let library = parse_kicad_symbol_library(
            r#"(kicad_symbol_lib
  (version 20230121)
  (symbol "NekoSpice:C"
    (property "Reference" "C" (at 0 0 0))
    (property "Value" "100n" (at 0 -2.54 0))
    (symbol "C_0_1"
      (pin passive line (at 0 -2.54 90) (length 2.54) (name "~") (number "1"))
      (pin passive line (at 0 2.54 270) (length 2.54) (name "~") (number "2"))
    )
  )
)"#,
            "inline.kicad_sym",
        )
        .unwrap();
        let args = [
            "input.kicad_sch",
            "--output",
            "placed.kicad_sch",
            "--library",
            "neko_spice.kicad_sym",
            "place-symbol:NekoSpice:C:C2:47n:101.6,53.34:unit=2:body-style=1:alt=6=SDA",
        ]
        .iter()
        .map(|value| value.to_string())
        .collect::<Vec<_>>();

        assert_eq!(
            positionals(&args),
            vec![
                "input.kicad_sch",
                "place-symbol:NekoSpice:C:C2:47n:101.6,53.34:unit=2:body-style=1:alt=6=SDA",
            ]
        );

        let edits = parse_kicad_edit_ops(&args, &library.symbols).unwrap();
        assert_eq!(edits.len(), 1);
        match &edits[0] {
            KicadSchematicEdit::PlaceSymbol {
                definition,
                library_symbols,
                reference,
                value,
                at,
                unit,
                body_style,
                pin_alternates,
                ..
            } => {
                assert_eq!(definition.name, "NekoSpice:C");
                assert_eq!(library_symbols.len(), library.symbols.len());
                assert_eq!(reference, "C2");
                assert_eq!(value, "47n");
                assert_close(at.x, 101.6);
                assert_close(at.y, 53.34);
                assert_eq!(*unit, Some(2));
                assert_eq!(*body_style, Some(1));
                assert_eq!(pin_alternates.get("6").map(String::as_str), Some("SDA"));
            }
            edit => panic!("expected place-symbol edit, got {edit:?}"),
        }
    }

    #[test]
    fn parses_kicad_configure_symbol_edit() {
        let args = [
            "input.kicad_sch",
            "--output",
            "configured.kicad_sch",
            "configure-symbol:U2:unit=2:body-style=1:mirror=xy:alt=4=ALT4",
            "configure-symbol:U2:body-style=none:mirror=none",
        ]
        .iter()
        .map(|value| value.to_string())
        .collect::<Vec<_>>();

        let edits = parse_kicad_edit_ops(&args, &[]).unwrap();
        assert_eq!(edits.len(), 2);
        match &edits[0] {
            KicadSchematicEdit::ConfigureSymbol {
                reference,
                unit,
                body_style,
                mirror,
                pin_alternates,
            } => {
                assert_eq!(reference, "U2");
                assert_eq!(*unit, Some(2));
                assert_eq!(*body_style, Some(Some(1)));
                assert_eq!(
                    mirror.as_ref().and_then(|mirror| mirror.as_deref()),
                    Some("x y")
                );
                assert_eq!(
                    pin_alternates
                        .as_ref()
                        .and_then(|alternates| alternates.get("4"))
                        .map(String::as_str),
                    Some("ALT4")
                );
            }
            edit => panic!("expected configure-symbol edit, got {edit:?}"),
        }
        match &edits[1] {
            KicadSchematicEdit::ConfigureSymbol {
                body_style, mirror, ..
            } => {
                assert_eq!(*body_style, Some(None));
                assert_eq!(*mirror, Some(None));
            }
            edit => panic!("expected configure-symbol edit, got {edit:?}"),
        }
    }

    #[test]
    fn parses_kicad_add_sheet_edit() {
        let args = [
            "input.kicad_sch",
            "--output",
            "hierarchical.kicad_sch",
            "add-sheet:gain_stage:gain_stage.kicad_sch:66.04,45.72:25.4,10.16:in@66.04,50.8,180,input;out@91.44,50.8,0,output:uuid=30000000-0000-0000-0000-000000000008",
        ]
        .iter()
        .map(|value| value.to_string())
        .collect::<Vec<_>>();

        let edits = parse_kicad_edit_ops(&args, &[]).unwrap();

        assert_eq!(edits.len(), 1);
        match &edits[0] {
            KicadSchematicEdit::AddSheet {
                name,
                file,
                at,
                size,
                pins,
                uuid,
            } => {
                assert_eq!(name, "gain_stage");
                assert_eq!(file, "gain_stage.kicad_sch");
                assert_close(at.x, 66.04);
                assert_close(at.y, 45.72);
                assert_close(size.width, 25.4);
                assert_close(size.height, 10.16);
                assert_eq!(pins.len(), 2);
                assert_eq!(pins[0].name, "in");
                assert_eq!(pins[0].pin_type, "input");
                assert_close(pins[0].at.unwrap().rotation, 180.0);
                assert_eq!(
                    uuid.as_deref(),
                    Some("30000000-0000-0000-0000-000000000008")
                );
            }
            edit => panic!("expected add-sheet edit, got {edit:?}"),
        }
    }

    #[test]
    fn parses_structured_yaml_with_units_and_flow_style() {
        let yaml = r#"
runs:
  - name: quoted_units
    netlist: "rc_filter/rc.cir"
    sweep: { rload: ["500", "1k", 2000] }
    checks:
      - { name: average, kind: avg, signal: "v(out)", from: "8us", to: "10us", min: 0.48, max: 0.52 }
      - name: default_kind
        signal: i(v1)
        min: -1m
        max: 1m
"#;

        let config = VerifyConfig::parse_from_str(
            yaml,
            std::path::Path::new("examples"),
            "default_project",
            "inline",
        )
        .unwrap();

        assert_eq!(config.project, "default_project");
        assert_eq!(config.runs.len(), 1);
        assert_eq!(
            config.runs[0].netlist,
            PathBuf::from("examples/rc_filter/rc.cir")
        );
        assert_eq!(config.runs[0].sweep[0].values, vec![500.0, 1000.0, 2000.0]);
        assert_close(config.runs[0].checks[0].from.unwrap(), 8e-6);
        assert_close(config.runs[0].checks[0].to.unwrap(), 10e-6);
        assert_eq!(config.runs[0].checks[1].kind, "final_value");
        assert_close(config.runs[0].checks[1].min.unwrap(), -1e-3);
    }

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() <= expected.abs().max(1.0) * 1e-12,
            "actual={actual} expected={expected}"
        );
    }
}
