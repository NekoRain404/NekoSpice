use osl_core::{
    Artifact, OslError, OslResult, ParameterOverride, RunMetadata, RunStatus, html_escape,
    json_escape, make_run_id, parameters_json, read_text, write_text,
};
use osl_kicad::{
    KicadAt, KicadCanvasScene, KicadLabelKind, KicadPoint, KicadSchematicEdit, KicadSheetPin,
    KicadSize, KicadSymbolDef, KicadSymbolLibraryIndexQuery, normalize_symbol_mirror,
    read_kicad_project, read_kicad_schematic_with_libraries, read_kicad_symbol_library,
    read_kicad_symbol_library_index, read_kicad_symbol_library_table, write_kicad_schematic,
    write_kicad_symbol_library,
};
use osl_model::{ModelCheckOptions, ModelCheckReport};
use osl_netlist::{ImportReport, NormalizedDependency, read_import_input};
use osl_render::render_kicad_scene_svg;
use osl_sim::{NgspiceCliBackend, SimulatorBackend};
use osl_waveform::{
    MeasurementKind, WaveformSummary, WaveformViewportQuery, measure, read_ngspice_raw,
};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::{Arc, Mutex};
use std::thread;

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
  osl kicad-check <file.kicad_sch> [--output <file>]
  osl kicad-export <file.kicad_sch-or-file.kicad_sym> --output <file>
  osl kicad-edit <file.kicad_sch> --output <file.kicad_sch> [--library <file.kicad_sym>] <ops...>
      delete-item:<uuid>
      move-item:<uuid>:<dx,dy>
      configure-symbol:<reference>[:unit=<n>][:body-style=<n|none>][:mirror=<x|y|xy|none>][:alt=<pin>=<alternate>[,<pin>=<alternate>...]]
      move-symbol:<reference>:<x,y>[:rotation]
      set-property:<reference>:<name>=<value>[:x,y[,rotation]]
      place-symbol:<lib_id>:<reference>:<value>:<x,y[,rotation]>[:unit=<n>][:body-style=<n>][:alt=<pin>=<alternate>[,<pin>=<alternate>...]]
  osl kicad-render <file.kicad_sch-or-file.kicad_sym> [--symbol <name>] [--unit <n>] [--body-style <n>] --output <file.svg>
  osl waveform <waveform.raw> --signal <name> [--from <time>] [--to <time>] [--points <n>] [--output <file>]
  osl report <run-or-verify-dir>
  osl --version

Three-day target:
  batch ngspice runs, reproducible run metadata, HTML/JSON reports, and CI-friendly pass/fail output.
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
    write_text(&output_dir.join("verify.json"), &report.to_json())?;
    write_text(&output_dir.join("report.html"), &report.to_html())?;

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
    write_text(&output_dir.join("bench.json"), &report.to_json())?;
    write_text(&output_dir.join("report.html"), &report.to_html())?;

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
    write_text(&output_dir.join("model-check.json"), &report.to_json())?;
    write_text(
        &output_dir.join("report.html"),
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
    write_text(&output_dir.join("import.json"), &report.to_json())?;
    write_text(
        &output_dir.join("report.html"),
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

fn kicad_inspect_command(args: &[String]) -> OslResult<i32> {
    let input = positional(args, 0, "missing KiCad path for 'osl kicad-inspect'")?;
    let output = flag_value(args, "--output");
    let path = Path::new(input);
    let should_emit_canvas = has_flag(args, "--canvas");
    let should_index = has_flag(args, "--index");
    let index_query = KicadSymbolLibraryIndexQuery {
        text: flag_value(args, "--query"),
        library: flag_value(args, "--library"),
        footprint: flag_value(args, "--footprint"),
    };
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();

    let json = match (
        extension.as_str(),
        path.file_name().and_then(|name| name.to_str()),
    ) {
        ("kicad_sch", _) if should_emit_canvas => read_kicad_schematic_with_libraries(path)?
            .canvas_scene()
            .to_json(),
        ("kicad_sch", _) => read_kicad_schematic_with_libraries(path)?.to_summary_json(),
        ("kicad_pro", _) => read_kicad_project(path)?.to_summary_json(),
        ("kicad_sym", _) => read_kicad_symbol_library(path)?.to_summary_json(),
        (_, Some("sym-lib-table")) if should_index => {
            let index = read_kicad_symbol_library_index(path)?;
            if index_query.is_empty() {
                index.to_json()
            } else {
                index.query(&index_query).to_json()
            }
        }
        (_, Some("sym-lib-table")) => read_kicad_symbol_library_table(path)?.to_summary_json(),
        _ => {
            return Err(OslError::InvalidInput(format!(
                "{} is not a supported KiCad project/schematic/library file (.kicad_pro, .kicad_sch, .kicad_sym, sym-lib-table)",
                path.display()
            )));
        }
    };

    if let Some(output) = output {
        write_text(Path::new(&output), &json)?;
        println!("kicad-inspect -> {output}");
    } else {
        print!("{json}");
    }
    Ok(0)
}

fn kicad_check_command(args: &[String]) -> OslResult<i32> {
    let input = positional(args, 0, "missing KiCad path for 'osl kicad-check'")?;
    let output = flag_value(args, "--output");
    let input_path = Path::new(input);
    let extension = input_path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    if extension != "kicad_sch" {
        return Err(OslError::InvalidInput(format!(
            "{} is not a supported KiCad check input (.kicad_sch)",
            input_path.display()
        )));
    }

    let schematic = read_kicad_schematic_with_libraries(input_path)?;
    let report = schematic
        .check_report_with_hierarchy(input_path.parent().unwrap_or_else(|| Path::new(".")))?;
    let json = report.to_json();
    if let Some(output) = output {
        write_text(Path::new(&output), &json)?;
        println!(
            "kicad-check: {} diagnostics ({} errors, {} warnings) -> {}",
            report.diagnostics.len(),
            report.error_count(),
            report.warning_count(),
            output
        );
    } else {
        print!("{json}");
    }

    Ok(if report.error_count() == 0 { 0 } else { 2 })
}

fn kicad_export_command(args: &[String]) -> OslResult<i32> {
    let input = positional(args, 0, "missing KiCad path for 'osl kicad-export'")?;
    let output = flag_value(args, "--output").ok_or_else(|| {
        OslError::InvalidInput("missing --output <file> for 'osl kicad-export'".to_string())
    })?;
    let input_path = Path::new(input);
    let output_path = Path::new(&output);
    let extension = input_path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();

    match extension.as_str() {
        "kicad_sch" => {
            let schematic = read_kicad_schematic_with_libraries(input_path)?;
            write_kicad_schematic(output_path, &schematic)?;
        }
        "kicad_sym" => {
            let library = read_kicad_symbol_library(input_path)?;
            write_kicad_symbol_library(output_path, &library)?;
        }
        _ => {
            return Err(OslError::InvalidInput(format!(
                "{} is not a supported KiCad export input (.kicad_sch, .kicad_sym)",
                input_path.display()
            )));
        }
    }

    println!("kicad-export -> {output}");
    Ok(0)
}

fn kicad_edit_command(args: &[String]) -> OslResult<i32> {
    let input = positional(args, 0, "missing KiCad path for 'osl kicad-edit'")?;
    let output = flag_value(args, "--output").ok_or_else(|| {
        OslError::InvalidInput("missing --output <file.kicad_sch> for 'osl kicad-edit'".to_string())
    })?;
    let input_path = Path::new(input);
    let extension = input_path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    if extension != "kicad_sch" {
        return Err(OslError::InvalidInput(format!(
            "{} is not a supported KiCad edit input (.kicad_sch)",
            input_path.display()
        )));
    }

    let mut schematic = read_kicad_schematic_with_libraries(input_path)?;
    let mut symbol_definitions = schematic.library_symbols.clone();
    for library_path in flag_values(args, "--library") {
        let library = read_kicad_symbol_library(Path::new(&library_path))?;
        symbol_definitions.extend(library.symbols);
    }

    let edits = parse_kicad_edit_ops(args, &symbol_definitions)?;
    if edits.is_empty() {
        return Err(OslError::InvalidInput(
            "kicad-edit requires at least one edit op".to_string(),
        ));
    }

    let mut summaries = Vec::new();
    for edit in edits {
        summaries.push(schematic.apply_edit(edit)?);
    }
    write_kicad_schematic(Path::new(&output), &schematic)?;

    println!("kicad-edit -> {output} ({} edits)", summaries.len());
    for summary in summaries {
        println!("  {} {}", summary.operation, summary.target);
    }
    Ok(0)
}

fn kicad_render_command(args: &[String]) -> OslResult<i32> {
    let input = positional(args, 0, "missing KiCad path for 'osl kicad-render'")?;
    let output = flag_value(args, "--output").ok_or_else(|| {
        OslError::InvalidInput("missing --output <file.svg> for 'osl kicad-render'".to_string())
    })?;
    let input_path = Path::new(input);
    let extension = input_path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    let scene = match extension.as_str() {
        "kicad_sch" => read_kicad_schematic_with_libraries(input_path)?.canvas_scene(),
        "kicad_sym" => {
            let symbol_name = flag_value(args, "--symbol").ok_or_else(|| {
                OslError::InvalidInput(
                    "missing --symbol <name> for rendering a KiCad symbol library".to_string(),
                )
            })?;
            let unit = flag_value(args, "--unit")
                .map(|value| parse_positive_u32(&value, "--unit"))
                .transpose()?;
            let body_style = flag_value(args, "--body-style")
                .map(|value| parse_positive_u32(&value, "--body-style"))
                .transpose()?;
            let library = read_kicad_symbol_library(input_path)?;
            let symbol = library
                .symbol_by_name_or_local_name(&symbol_name)
                .ok_or_else(|| {
                    OslError::InvalidInput(format!(
                        "symbol '{}' was not found in {}",
                        symbol_name,
                        input_path.display()
                    ))
                })?;
            KicadCanvasScene::from_symbol_definition(
                format!("{}:{}", input_path.display(), symbol.local_name()),
                symbol,
                &library.symbols,
                unit,
                body_style,
            )
        }
        _ => {
            return Err(OslError::InvalidInput(format!(
                "{} is not a supported KiCad render input (.kicad_sch, .kicad_sym)",
                input_path.display()
            )));
        }
    };
    let svg = render_kicad_scene_svg(&scene);
    write_text(Path::new(&output), &svg)?;
    println!("kicad-render -> {output}");
    Ok(0)
}

fn copy_import_dependencies(
    report: &ImportReport,
    input_path: &Path,
    project_dir: &Path,
) -> OslResult<Vec<NormalizedDependency>> {
    let base_dir = input_path.parent().unwrap_or_else(|| Path::new("."));
    let mut dependencies = Vec::new();

    for include in &report.includes {
        let include_path = Path::new(&include.path);
        if include_path.is_absolute() {
            continue;
        }
        let Some(file_name) = include_path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let source_path = base_dir.join(include_path);
        if !source_path.is_file() {
            continue;
        }
        let project_path = Path::new("models").join(sanitize_dependency_file_name(file_name));
        let content = read_text(&source_path)?;
        write_text(&project_dir.join(&project_path), &content)?;
        dependencies.push(NormalizedDependency {
            source: include.path.clone(),
            project_path: project_path.display().to_string(),
        });
    }

    Ok(dependencies)
}

fn sanitize_dependency_file_name(file_name: &str) -> String {
    let mut output = String::with_capacity(file_name.len());
    for character in file_name.chars() {
        if character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_') {
            output.push(character);
        } else {
            output.push('_');
        }
    }
    if output.is_empty() {
        "dependency.lib".to_string()
    } else {
        output
    }
}

fn report_command(args: &[String]) -> OslResult<i32> {
    let dir = PathBuf::from(positional(args, 0, "missing directory for 'osl report'")?);
    let run_json = dir.join("run.json");
    let verify_json = dir.join("verify.json");
    let bench_json = dir.join("bench.json");
    let model_check_json = dir.join("model-check.json");
    let import_json = dir.join("import.json");

    if run_json.is_file() {
        let content = read_text(&run_json)?;
        let html = format!(
            concat!(
                "<!doctype html><html><head><meta charset=\"utf-8\">",
                "<title>NekoSpice Run Report</title>{}</head><body>",
                "<main><h1>NekoSpice Run Report</h1><pre>{}</pre></main></body></html>\n"
            ),
            report_css(),
            html_escape(&content)
        );
        write_text(&dir.join("report.html"), &html)?;
        println!("report -> {}", dir.join("report.html").display());
        return Ok(0);
    }

    if verify_json.is_file()
        || bench_json.is_file()
        || model_check_json.is_file()
        || import_json.is_file()
    {
        let source = if verify_json.is_file() {
            verify_json
        } else if model_check_json.is_file() {
            model_check_json
        } else if import_json.is_file() {
            import_json
        } else {
            bench_json
        };
        let content = read_text(&source)?;
        let html = format!(
            concat!(
                "<!doctype html><html><head><meta charset=\"utf-8\">",
                "<title>NekoSpice Batch Report</title>{}</head><body>",
                "<main><h1>NekoSpice Batch Report</h1><pre>{}</pre></main></body></html>\n"
            ),
            report_css(),
            html_escape(&content)
        );
        write_text(&dir.join("report.html"), &html)?;
        println!("report -> {}", dir.join("report.html").display());
        return Ok(0);
    }

    Err(OslError::InvalidInput(format!(
        "{} does not contain run.json, verify.json, bench.json, model-check.json, or import.json",
        dir.display()
    )))
}

#[derive(Debug)]
struct VerifyConfig {
    project: String,
    runs: Vec<VerifyRun>,
}

#[derive(Debug)]
struct VerifyRun {
    name: String,
    netlist: PathBuf,
    sweep: Vec<SweepDimension>,
    checks: Vec<VerifyCheck>,
}

impl VerifyRun {
    fn expand_cases(&self) -> OslResult<Vec<RunCase>> {
        if self.sweep.is_empty() {
            return Ok(vec![RunCase {
                name: self.name.clone(),
                parameters: Vec::new(),
            }]);
        }

        let mut cases = vec![RunCase {
            name: self.name.clone(),
            parameters: Vec::new(),
        }];

        for dimension in &self.sweep {
            if dimension.values.is_empty() {
                return Err(OslError::InvalidInput(format!(
                    "sweep '{}' in run '{}' has no values",
                    dimension.name, self.name
                )));
            }

            let mut expanded = Vec::new();
            for case in &cases {
                for value in &dimension.values {
                    let mut parameters = case.parameters.clone();
                    parameters.push(ParameterOverride::new(&dimension.name, *value));
                    expanded.push(RunCase {
                        name: format!(
                            "{}__{}={}",
                            case.name,
                            dimension.name,
                            format_parameter_value(*value)
                        ),
                        parameters,
                    });
                }
            }
            cases = expanded;
        }

        Ok(cases)
    }
}

#[derive(Debug, Clone)]
struct SweepDimension {
    name: String,
    values: Vec<f64>,
}

#[derive(Debug, Clone)]
struct RunCase {
    name: String,
    parameters: Vec<ParameterOverride>,
}

#[derive(Debug, Clone)]
struct VerifyTask {
    index: usize,
    name: String,
    netlist: String,
    netlist_path: PathBuf,
    run_dir: String,
    parameters: Vec<ParameterOverride>,
    checks: Vec<VerifyCheck>,
}

#[derive(Debug, Clone)]
struct VerifyCheck {
    name: String,
    kind: String,
    signal: String,
    from: Option<f64>,
    to: Option<f64>,
    min: Option<f64>,
    max: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct VerifyConfigYaml {
    project: Option<String>,
    #[serde(default)]
    runs: Vec<VerifyRunYaml>,
}

#[derive(Debug, Deserialize)]
struct VerifyRunYaml {
    name: String,
    netlist: PathBuf,
    #[serde(default)]
    sweep: std::collections::BTreeMap<String, Vec<YamlNumber>>,
    #[serde(default)]
    checks: Vec<VerifyCheckYaml>,
}

#[derive(Debug, Deserialize)]
struct VerifyCheckYaml {
    name: String,
    #[serde(default = "default_check_kind")]
    kind: String,
    signal: String,
    from: Option<YamlNumber>,
    to: Option<YamlNumber>,
    min: Option<YamlNumber>,
    max: Option<YamlNumber>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum YamlNumber {
    Number(f64),
    Text(String),
}

impl YamlNumber {
    fn parse(&self, context: &str) -> OslResult<f64> {
        match self {
            Self::Number(value) => Ok(*value),
            Self::Text(value) => parse_number(value, context),
        }
    }
}

fn default_check_kind() -> String {
    "final_value".to_string()
}

impl VerifyConfig {
    fn parse(path: &Path) -> OslResult<Self> {
        let content = read_text(path)?;
        Self::parse_from_str(
            &content,
            path.parent().unwrap_or_else(|| Path::new(".")),
            &default_project_name(path),
            &path.display().to_string(),
        )
    }

    fn parse_from_str(
        content: &str,
        base_dir: &Path,
        default_project: &str,
        source_name: &str,
    ) -> OslResult<Self> {
        let parsed = serde_yaml::from_str::<VerifyConfigYaml>(content).map_err(|err| {
            OslError::InvalidInput(format!("{source_name} has invalid YAML: {err}"))
        })?;
        let project = parsed
            .project
            .unwrap_or_else(|| default_project.to_string());
        let runs = parsed
            .runs
            .into_iter()
            .map(|run| run.into_verify_run(base_dir))
            .collect::<OslResult<Vec<_>>>()?;

        if runs.is_empty() {
            return Err(OslError::InvalidInput(format!(
                "{source_name} must contain runs with name and netlist"
            )));
        }

        Ok(Self { project, runs })
    }
}

fn default_project_name(path: &Path) -> String {
    path.file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("verification")
        .to_string()
}

impl VerifyRunYaml {
    fn into_verify_run(self, base_dir: &Path) -> OslResult<VerifyRun> {
        let netlist = if self.netlist.is_absolute() {
            self.netlist
        } else {
            base_dir.join(self.netlist)
        };
        let sweep = self
            .sweep
            .into_iter()
            .map(|(name, values)| {
                if name.trim().is_empty() {
                    return Err(OslError::InvalidInput(
                        "verify sweep has an empty parameter name".to_string(),
                    ));
                }
                let values = values
                    .iter()
                    .map(|value| value.parse(&format!("sweep '{}'", name)))
                    .collect::<OslResult<Vec<_>>>()?;
                Ok(SweepDimension { name, values })
            })
            .collect::<OslResult<Vec<_>>>()?;
        let checks = self
            .checks
            .into_iter()
            .map(VerifyCheckYaml::into_verify_check)
            .collect::<OslResult<Vec<_>>>()?;

        validate_run(VerifyRun {
            name: self.name,
            netlist,
            sweep,
            checks,
        })
    }
}

impl VerifyCheckYaml {
    fn into_verify_check(self) -> OslResult<VerifyCheck> {
        validate_check(VerifyCheck {
            name: self.name,
            kind: self.kind,
            signal: self.signal,
            from: self
                .from
                .as_ref()
                .map(|value| value.parse("check from"))
                .transpose()?,
            to: self
                .to
                .as_ref()
                .map(|value| value.parse("check to"))
                .transpose()?,
            min: self
                .min
                .as_ref()
                .map(|value| value.parse("check min"))
                .transpose()?,
            max: self
                .max
                .as_ref()
                .map(|value| value.parse("check max"))
                .transpose()?,
        })
    }
}

fn validate_run(run: VerifyRun) -> OslResult<VerifyRun> {
    if run.name.trim().is_empty() {
        return Err(OslError::InvalidInput(
            "verify run has an empty name".to_string(),
        ));
    }
    if run.netlist.as_os_str().is_empty() {
        return Err(OslError::InvalidInput(format!(
            "verify run '{}' is missing netlist",
            run.name
        )));
    }
    Ok(run)
}

fn validate_check(check: VerifyCheck) -> OslResult<VerifyCheck> {
    if check.name.trim().is_empty() {
        return Err(OslError::InvalidInput(
            "verify check has an empty name".to_string(),
        ));
    }
    if check.signal.trim().is_empty() {
        return Err(OslError::InvalidInput(format!(
            "verify check '{}' is missing signal",
            check.name
        )));
    }
    if check.min.is_none() && check.max.is_none() {
        return Err(OslError::InvalidInput(format!(
            "verify check '{}' must define min or max",
            check.name
        )));
    }
    if let (Some(from), Some(to)) = (check.from, check.to)
        && from > to
    {
        return Err(OslError::InvalidInput(format!(
            "verify check '{}' has from > to",
            check.name
        )));
    }
    Ok(check)
}

fn run_verify_tasks(
    tasks: Vec<VerifyTask>,
    ngspice: &str,
    jobs: usize,
) -> OslResult<Vec<VerifyRunResult>> {
    if tasks.is_empty() {
        return Ok(Vec::new());
    }

    if jobs == 1 || tasks.len() == 1 {
        return tasks
            .into_iter()
            .map(|task| run_verify_task(task, ngspice))
            .collect();
    }

    let worker_count = jobs.min(tasks.len());
    let queue = Arc::new(Mutex::new(tasks.into_iter().rev().collect::<Vec<_>>()));
    let results = Arc::new(Mutex::new(Vec::<VerifyRunResult>::new()));
    let errors = Arc::new(Mutex::new(Vec::<String>::new()));
    let mut handles = Vec::new();

    for _ in 0..worker_count {
        let queue = Arc::clone(&queue);
        let results = Arc::clone(&results);
        let errors = Arc::clone(&errors);
        let ngspice = ngspice.to_string();
        handles.push(thread::spawn(move || {
            loop {
                let task = {
                    let mut queue = queue.lock().expect("verify task queue lock poisoned");
                    queue.pop()
                };
                let Some(task) = task else {
                    break;
                };

                match run_verify_task(task, &ngspice) {
                    Ok(result) => {
                        let mut results = results.lock().expect("verify result lock poisoned");
                        results.push(result);
                    }
                    Err(error) => {
                        let mut errors = errors.lock().expect("verify error lock poisoned");
                        errors.push(error.to_string());
                    }
                }
            }
        }));
    }

    for handle in handles {
        handle
            .join()
            .map_err(|_| OslError::Process("verify worker thread panicked".to_string()))?;
    }

    let errors = Arc::try_unwrap(errors)
        .map_err(|_| OslError::Process("verify errors still shared".to_string()))?
        .into_inner()
        .map_err(|_| OslError::Process("verify error lock poisoned".to_string()))?;
    if !errors.is_empty() {
        return Err(OslError::Process(errors.join("; ")));
    }

    let mut results = Arc::try_unwrap(results)
        .map_err(|_| OslError::Process("verify results still shared".to_string()))?
        .into_inner()
        .map_err(|_| OslError::Process("verify result lock poisoned".to_string()))?;
    results.sort_by_key(|result| result.index);
    Ok(results)
}

fn run_verify_task(task: VerifyTask, ngspice: &str) -> OslResult<VerifyRunResult> {
    let backend = NgspiceCliBackend::new(ngspice);
    let run_dir = PathBuf::from(&task.run_dir);
    let mut metadata =
        backend.run_with_parameters(&task.netlist_path, &run_dir, &task.parameters)?;
    let metadata_output_dir = PathBuf::from(&metadata.output_dir);
    finalize_run_output(&metadata_output_dir, &mut metadata)?;
    let checks = evaluate_checks(&run_dir, &task.checks);

    Ok(VerifyRunResult {
        index: task.index,
        name: task.name,
        netlist: task.netlist,
        run_dir: task.run_dir,
        metadata,
        parameters: task.parameters,
        checks,
    })
}

fn evaluate_checks(run_dir: &Path, checks: &[VerifyCheck]) -> Vec<CheckResult> {
    checks
        .iter()
        .map(|check| evaluate_check(run_dir, check))
        .collect()
}

fn evaluate_check(run_dir: &Path, check: &VerifyCheck) -> CheckResult {
    let waveform_path = run_dir.join("waveform.raw");
    let waveform = match read_ngspice_raw(&waveform_path) {
        Ok(waveform) => waveform,
        Err(error) => {
            return CheckResult {
                name: check.name.clone(),
                kind: check.kind.clone(),
                signal: check.signal.clone(),
                from: check.from,
                to: check.to,
                value: None,
                summary: None,
                min: check.min,
                max: check.max,
                passed: false,
                message: error.to_string(),
            };
        }
    };
    let values = match waveform.signal_values_in_window(&check.signal, check.from, check.to) {
        Ok(values) => values,
        Err(error) => {
            return CheckResult {
                name: check.name.clone(),
                kind: check.kind.clone(),
                signal: check.signal.clone(),
                from: check.from,
                to: check.to,
                value: None,
                summary: None,
                min: check.min,
                max: check.max,
                passed: false,
                message: error.to_string(),
            };
        }
    };
    let kind = match MeasurementKind::parse(&check.kind) {
        Ok(kind) => kind,
        Err(error) => {
            return CheckResult {
                name: check.name.clone(),
                kind: check.kind.clone(),
                signal: check.signal.clone(),
                from: check.from,
                to: check.to,
                value: None,
                summary: None,
                min: check.min,
                max: check.max,
                passed: false,
                message: error.to_string(),
            };
        }
    };
    let value = match measure(kind, &values) {
        Ok(value) => value,
        Err(error) => {
            return CheckResult {
                name: check.name.clone(),
                kind: check.kind.clone(),
                signal: check.signal.clone(),
                from: check.from,
                to: check.to,
                value: None,
                summary: None,
                min: check.min,
                max: check.max,
                passed: false,
                message: error.to_string(),
            };
        }
    };
    let summary = match WaveformSummary::summarize(&values) {
        Ok(summary) => summary,
        Err(error) => {
            return CheckResult {
                name: check.name.clone(),
                kind: check.kind.clone(),
                signal: check.signal.clone(),
                from: check.from,
                to: check.to,
                value: None,
                summary: None,
                min: check.min,
                max: check.max,
                passed: false,
                message: error.to_string(),
            };
        }
    };

    let above_min = check.min.is_none_or(|min| value >= min);
    let below_max = check.max.is_none_or(|max| value <= max);
    let passed = above_min && below_max;
    let message = format!(
        "value={} min={} max={} window={}",
        value,
        option_f64_text(check.min),
        option_f64_text(check.max),
        window_text(check.from, check.to)
    );

    CheckResult {
        name: check.name.clone(),
        kind: check.kind.clone(),
        signal: check.signal.clone(),
        from: check.from,
        to: check.to,
        value: Some(value),
        summary: Some(summary),
        min: check.min,
        max: check.max,
        passed,
        message,
    }
}

#[derive(Debug)]
struct VerifyRunResult {
    index: usize,
    name: String,
    netlist: String,
    run_dir: String,
    metadata: RunMetadata,
    parameters: Vec<ParameterOverride>,
    checks: Vec<CheckResult>,
}

impl VerifyRunResult {
    fn status(&self) -> RunStatus {
        if self.metadata.status == RunStatus::Passed && self.checks.iter().all(|check| check.passed)
        {
            RunStatus::Passed
        } else {
            RunStatus::Failed
        }
    }

    fn run_dir_name(&self) -> String {
        Path::new(&self.run_dir)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(&self.name)
            .to_string()
    }

    fn artifact_href(&self, artifact: &str) -> String {
        format!("runs/{}/{}", self.run_dir_name(), artifact)
    }

    fn failed_checks(&self) -> impl Iterator<Item = &CheckResult> {
        self.checks.iter().filter(|check| !check.passed)
    }
}

#[derive(Debug)]
struct CheckResult {
    name: String,
    kind: String,
    signal: String,
    from: Option<f64>,
    to: Option<f64>,
    value: Option<f64>,
    summary: Option<WaveformSummary>,
    min: Option<f64>,
    max: Option<f64>,
    passed: bool,
    message: String,
}

impl CheckResult {
    fn status_text(&self) -> &'static str {
        if self.passed { "pass" } else { "fail" }
    }
}

#[derive(Debug)]
struct VerifyReport {
    project: String,
    results: Vec<VerifyRunResult>,
}

impl VerifyReport {
    fn passed_count(&self) -> usize {
        self.results
            .iter()
            .filter(|result| result.status() == RunStatus::Passed)
            .count()
    }

    fn failed_count(&self) -> usize {
        self.results.len() - self.passed_count()
    }

    fn failure_count(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.failed_checks().count())
            .sum()
    }

    fn to_json(&self) -> String {
        let failures = self
            .results
            .iter()
            .flat_map(|result| {
                result.failed_checks().map(|check| {
                    format!(
                        concat!(
                            "    {{ \"run\": \"{}\", \"netlist\": \"{}\", \"run_dir\": \"{}\", ",
                            "\"check\": \"{}\", \"signal\": \"{}\", \"value\": {}, ",
                            "\"min\": {}, \"max\": {}, \"summary\": {}, \"message\": \"{}\" }}"
                        ),
                        json_escape(&result.name),
                        json_escape(&result.netlist),
                        json_escape(&result.run_dir),
                        json_escape(&check.name),
                        json_escape(&check.signal),
                        option_f64_json(check.value),
                        option_f64_json(check.min),
                        option_f64_json(check.max),
                        summary_json(check.summary),
                        json_escape(&check.message)
                    )
                })
            })
            .collect::<Vec<_>>()
            .join(",\n");
        let runs = self
            .results
            .iter()
            .map(|result| {
                let parameters = parameters_json(&result.parameters, 8);
                let checks = result
                    .checks
                    .iter()
                    .map(|check| {
                        format!(
                            concat!(
                                "        {{ \"name\": \"{}\", \"kind\": \"{}\", \"signal\": \"{}\", ",
                                "\"from\": {}, \"to\": {}, \"value\": {}, \"min\": {}, \"max\": {}, \"passed\": {}, \"summary\": {}, \"message\": \"{}\" }}"
                            ),
                            json_escape(&check.name),
                            json_escape(&check.kind),
                            json_escape(&check.signal),
                            option_f64_json(check.from),
                            option_f64_json(check.to),
                            option_f64_json(check.value),
                            option_f64_json(check.min),
                            option_f64_json(check.max),
                            check.passed,
                            summary_json(check.summary),
                            json_escape(&check.message)
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(",\n");
                format!(
                    concat!(
                        "    {{\n",
                        "      \"name\": \"{}\",\n",
                        "      \"netlist\": \"{}\",\n",
                        "      \"run_dir\": \"{}\",\n",
                        "      \"status\": \"{}\",\n",
                        "      \"simulation_status\": \"{}\",\n",
                        "      \"exit_code\": {},\n",
                        "      \"duration_ms\": {},\n",
                        "      \"parameters\": [\n",
                        "{}\n",
                        "      ],\n",
                        "      \"checks\": [\n",
                        "{}\n",
                        "      ]\n",
                        "    }}"
                    ),
                    json_escape(&result.name),
                    json_escape(&result.netlist),
                    json_escape(&result.run_dir),
                    result.status().as_str(),
                    result.metadata.status.as_str(),
                    result
                        .metadata
                        .exit_code
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "null".to_string()),
                    result.metadata.duration_ms,
                    parameters,
                    checks
                )
            })
            .collect::<Vec<_>>()
            .join(",\n");

        format!(
            concat!(
                "{{\n",
                "  \"schema_version\": 1,\n",
                "  \"project\": \"{}\",\n",
                "  \"passed\": {},\n",
                "  \"failed\": {},\n",
                "  \"failure_count\": {},\n",
                "  \"failures\": [\n",
                "{}\n",
                "  ],\n",
                "  \"runs\": [\n",
                "{}\n",
                "  ]\n",
                "}}\n"
            ),
            json_escape(&self.project),
            self.passed_count(),
            self.failed_count(),
            self.failure_count(),
            failures,
            runs
        )
    }

    fn to_html(&self) -> String {
        let failure_rows = self
            .results
            .iter()
            .flat_map(|result| {
                result.failed_checks().map(|check| {
                    format!(
                        concat!(
                            "<tr class=\"failed\"><td>{}</td><td>{}</td><td>{}</td>",
                            "<td>{}</td><td>{}</td><td><a href=\"{}\">run</a> <a href=\"{}\">raw</a> <a href=\"{}\">csv</a> <a href=\"{}\">summary</a> <a href=\"{}\">log</a></td></tr>"
                        ),
                        html_escape(&result.name),
                        html_escape(&parameters_text(&result.parameters)),
                        html_escape(&check.name),
                        html_escape(&check.message),
                        html_escape(&summary_text(check.summary)),
                        html_escape(&result.artifact_href("report.html")),
                        html_escape(&result.artifact_href("waveform.raw")),
                        html_escape(&result.artifact_href("waveform.csv")),
                        html_escape(&result.artifact_href("waveform-summary.json")),
                        html_escape(&result.artifact_href("ngspice.log"))
                    )
                })
            })
            .collect::<String>();
        let failure_section = if self.failure_count() == 0 {
            "<section class=\"summary\"><strong>No failed checks.</strong></section>".to_string()
        } else {
            format!(
                concat!(
                    "<section><h2>Failures</h2>",
                    "<table><thead><tr><th>Run</th><th>Parameters</th><th>Check</th><th>Message</th><th>Summary</th><th>Artifacts</th></tr></thead>",
                    "<tbody>{}</tbody></table></section>"
                ),
                failure_rows
            )
        };

        let rows = self
            .results
            .iter()
            .map(|result| {
                let status = result.status().as_str();
                let parameters = parameters_text(&result.parameters);
                let checks = if result.checks.is_empty() {
                    "no checks".to_string()
                } else {
                    result
                        .checks
                        .iter()
                        .map(|check| {
                            format!(
                                "{}: {} ({}; {})",
                                check.name,
                                check.status_text(),
                                check.message,
                                summary_text(check.summary)
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("; ")
                };
                format!(
                    concat!(
                        "<tr class=\"{}\"><td>{}</td><td>{}</td><td>{}</td>",
                        "<td>{}</td><td>{}</td><td><a href=\"{}\">report</a> <a href=\"{}\">run.json</a> <a href=\"{}\">raw</a> <a href=\"{}\">csv</a> <a href=\"{}\">summary</a> <a href=\"{}\">ngspice.log</a> <a href=\"{}\">input.cir</a></td><td>{}</td></tr>"
                    ),
                    status,
                    html_escape(status),
                    html_escape(&result.name),
                    html_escape(&result.netlist),
                    html_escape(&parameters),
                    result.metadata.duration_ms,
                    html_escape(&result.artifact_href("report.html")),
                    html_escape(&result.artifact_href("run.json")),
                    html_escape(&result.artifact_href("waveform.raw")),
                    html_escape(&result.artifact_href("waveform.csv")),
                    html_escape(&result.artifact_href("waveform-summary.json")),
                    html_escape(&result.artifact_href("ngspice.log")),
                    html_escape(&result.artifact_href("input.cir")),
                    html_escape(&checks)
                )
            })
            .collect::<String>();

        format!(
            concat!(
                "<!doctype html><html><head><meta charset=\"utf-8\">",
                "<title>NekoSpice Verification Report</title>{}</head><body>",
                "<main><h1>{}</h1>",
                "<section class=\"summary\"><strong>{}</strong> passed, <strong>{}</strong> failed, <strong>{}</strong> failed checks</section>",
                "{}",
                "<h2>Runs</h2>",
                "<table><thead><tr><th>Status</th><th>Run</th><th>Netlist</th><th>Parameters</th><th>ms</th><th>Artifacts</th><th>Checks</th></tr></thead>",
                "<tbody>{}</tbody></table>",
                "</main></body></html>\n"
            ),
            report_css(),
            html_escape(&self.project),
            self.passed_count(),
            self.failed_count(),
            self.failure_count(),
            failure_section,
            rows
        )
    }
}

fn write_single_run_report(output_dir: &Path, metadata: &RunMetadata) -> OslResult<()> {
    let artifact_items = metadata
        .artifacts
        .iter()
        .map(|artifact| {
            format!(
                "<li><a href=\"{}\"><code>{}</code></a> <span>{}</span></li>",
                html_escape(&artifact.path),
                html_escape(&artifact.path),
                html_escape(&artifact.kind)
            )
        })
        .collect::<String>();
    let html = format!(
        concat!(
            "<!doctype html><html><head><meta charset=\"utf-8\">",
            "<title>NekoSpice Run Report</title>{}</head><body>",
            "<main><h1>{}</h1>",
            "<section class=\"summary\"><strong>Status:</strong> {} <strong>Duration:</strong> {} ms</section>",
            "<dl><dt>Backend</dt><dd>{}</dd><dt>Netlist</dt><dd>{}</dd><dt>Output</dt><dd>{}</dd></dl>",
            "<h2>Artifacts</h2><ul>{}</ul>",
            "</main></body></html>\n"
        ),
        report_css(),
        html_escape(&metadata.run_id),
        html_escape(metadata.status.as_str()),
        metadata.duration_ms,
        html_escape(&metadata.backend),
        html_escape(&metadata.source_netlist),
        html_escape(&metadata.output_dir),
        artifact_items
    );
    write_text(&output_dir.join("report.html"), &html)
}

fn finalize_run_output(output_dir: &Path, metadata: &mut RunMetadata) -> OslResult<()> {
    if metadata.status == RunStatus::Passed {
        export_waveform_artifacts(output_dir)?;
    }
    refresh_artifacts(output_dir, metadata)?;
    write_single_run_report(output_dir, metadata)?;
    refresh_artifacts(output_dir, metadata)?;
    write_text(&output_dir.join("run.json"), &metadata.to_json())
}

fn export_waveform_artifacts(output_dir: &Path) -> OslResult<()> {
    let raw_path = output_dir.join("waveform.raw");
    if !raw_path.is_file() {
        return Ok(());
    }

    let waveform = read_ngspice_raw(&raw_path)?;
    write_text(&output_dir.join("waveform.csv"), &waveform.to_csv()?)?;
    write_text(
        &output_dir.join("waveform-summary.json"),
        &waveform.to_summary_json()?,
    )
}

fn refresh_artifacts(output_dir: &Path, metadata: &mut RunMetadata) -> OslResult<()> {
    metadata.artifacts = collect_run_artifacts(output_dir)?;
    metadata
        .artifacts
        .sort_by(|left, right| left.path.cmp(&right.path));
    Ok(())
}

fn collect_run_artifacts(output_dir: &Path) -> OslResult<Vec<Artifact>> {
    let mut artifacts = Vec::new();
    for entry in fs::read_dir(output_dir)
        .map_err(|err| OslError::io(format!("read {}", output_dir.display()), err))?
    {
        let entry = entry.map_err(|err| OslError::io("read output directory entry", err))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if file_name == "run.json" {
            continue;
        }

        artifacts.push(Artifact {
            path: file_name.to_string(),
            kind: run_artifact_kind(file_name).to_string(),
        });
    }
    Ok(artifacts)
}

fn run_artifact_kind(file_name: &str) -> &'static str {
    if file_name == "waveform-summary.json"
        || file_name.ends_with(".raw")
        || file_name.ends_with(".csv")
    {
        "waveform"
    } else if file_name.ends_with(".log") || file_name.ends_with(".txt") {
        "log"
    } else if file_name.ends_with(".cir") || file_name.ends_with(".net") {
        "netlist"
    } else if file_name.ends_with(".html") {
        "report"
    } else {
        "file"
    }
}

fn report_css() -> &'static str {
    concat!(
        "<style>",
        "body{margin:0;background:#f7f7f4;color:#1f2933;font:14px/1.5 system-ui,-apple-system,BlinkMacSystemFont,sans-serif}",
        "main{max-width:1100px;margin:0 auto;padding:32px}",
        "h1{font-size:28px;margin:0 0 20px}",
        "h2{font-size:18px;margin-top:24px}",
        ".summary{background:#fff;border:1px solid #d9ded7;border-radius:6px;padding:12px 14px;margin-bottom:16px}",
        "table{width:100%;border-collapse:collapse;background:#fff;border:1px solid #d9ded7}",
        "th,td{padding:10px 12px;border-bottom:1px solid #e5e7eb;text-align:left;vertical-align:top}",
        "th{background:#ecefeb;font-weight:700}",
        "tr.passed td:first-child{color:#0f766e;font-weight:700}",
        "tr.warning td:first-child{color:#b45309;font-weight:700}",
        "tr.failed td:first-child{color:#b91c1c;font-weight:700}",
        "code,pre{font-family:ui-monospace,SFMono-Regular,Consolas,monospace}",
        "pre{white-space:pre-wrap;background:#fff;border:1px solid #d9ded7;border-radius:6px;padding:16px;overflow:auto}",
        "dl{display:grid;grid-template-columns:120px 1fr;gap:8px 12px;background:#fff;border:1px solid #d9ded7;border-radius:6px;padding:14px}",
        "dt{font-weight:700}",
        "ul{background:#fff;border:1px solid #d9ded7;border-radius:6px;padding:14px 14px 14px 30px}",
        "</style>"
    )
}

fn positional<'a>(args: &'a [String], index: usize, missing: &str) -> OslResult<&'a str> {
    positionals(args)
        .into_iter()
        .nth(index)
        .ok_or_else(|| OslError::InvalidInput(missing.to_string()))
}

fn flag_value(args: &[String], flag: &str) -> Option<String> {
    let mut index = 0;
    while index < args.len() {
        if args[index] == flag {
            return args.get(index + 1).cloned();
        }
        index += 1;
    }
    None
}

fn flag_values(args: &[String], flag: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut index = 0;
    while index < args.len() {
        if args[index] == flag
            && let Some(value) = args.get(index + 1)
        {
            values.push(value.clone());
            index += 2;
        } else {
            index += 1;
        }
    }
    values
}

fn has_flag(args: &[String], flag: &str) -> bool {
    args.iter().any(|arg| arg == flag)
}

fn positionals(args: &[String]) -> Vec<&str> {
    let mut values = Vec::new();
    let mut index = 0;
    while index < args.len() {
        if args[index].starts_with("--") {
            index += if flag_takes_value(&args[index]) { 2 } else { 1 };
        } else {
            values.push(args[index].as_str());
            index += 1;
        }
    }
    values
}

fn flag_takes_value(flag: &str) -> bool {
    matches!(
        flag,
        "--body-style"
            | "--from"
            | "--jobs"
            | "--ngspice"
            | "--output"
            | "--points"
            | "--footprint"
            | "--library"
            | "--query"
            | "--signal"
            | "--symbol"
            | "--to"
            | "--unit"
    )
}

fn trailing_positionals(args: &[String], skip: usize) -> Vec<&str> {
    positionals(args).into_iter().skip(skip).collect()
}

fn parse_jobs(value: &str) -> OslResult<usize> {
    parse_positive_usize(value, "--jobs")
}

fn parse_positive_usize(value: &str, flag: &str) -> OslResult<usize> {
    let jobs = value.parse::<usize>().map_err(|_| {
        OslError::InvalidInput(format!(
            "{flag} expects a positive integer, got '{}'",
            value
        ))
    })?;
    if jobs == 0 {
        return Err(OslError::InvalidInput(format!(
            "{flag} expects a positive integer, got 0"
        )));
    }
    Ok(jobs)
}

fn parse_positive_u32(value: &str, flag: &str) -> OslResult<u32> {
    let parsed = value.parse::<u32>().map_err(|_| {
        OslError::InvalidInput(format!(
            "{flag} expects a positive integer, got '{}'",
            value
        ))
    })?;
    if parsed == 0 {
        return Err(OslError::InvalidInput(format!(
            "{flag} expects a positive integer, got 0"
        )));
    }
    Ok(parsed)
}

fn parse_optional_positive_u32(value: &str, flag: &str) -> OslResult<Option<u32>> {
    if value.eq_ignore_ascii_case("none") || value.eq_ignore_ascii_case("default") {
        Ok(None)
    } else {
        parse_positive_u32(value, flag).map(Some)
    }
}

fn parse_kicad_edit_ops(
    args: &[String],
    symbol_definitions: &[KicadSymbolDef],
) -> OslResult<Vec<KicadSchematicEdit>> {
    trailing_positionals(args, 1)
        .into_iter()
        .map(|op| parse_kicad_edit_op(op, symbol_definitions))
        .collect()
}

fn parse_kicad_edit_op(
    op: &str,
    symbol_definitions: &[KicadSymbolDef],
) -> OslResult<KicadSchematicEdit> {
    let (name, payload) = op.split_once(':').ok_or_else(|| {
        OslError::InvalidInput(format!(
            "invalid kicad-edit op '{op}', expected <op>:<payload>"
        ))
    })?;
    match name {
        "move-symbol" => parse_kicad_move_symbol_edit(payload),
        "move-item" => parse_kicad_move_item_edit(payload),
        "delete-item" => parse_kicad_delete_item_edit(payload),
        "configure-symbol" => parse_kicad_configure_symbol_edit(payload),
        "set-property" => parse_kicad_set_property_edit(payload),
        "place-symbol" => parse_kicad_place_symbol_edit(payload, symbol_definitions),
        "add-wire" => parse_kicad_add_wire_edit(payload),
        "add-bus" => parse_kicad_add_bus_edit(payload),
        "add-bus-entry" => parse_kicad_add_bus_entry_edit(payload),
        "add-junction" => parse_kicad_add_junction_edit(payload),
        "add-no-connect" => parse_kicad_add_no_connect_edit(payload),
        "add-label" => parse_kicad_add_label_edit(payload),
        "add-global-label" => parse_kicad_add_label_edit_with_kind(payload, KicadLabelKind::Global),
        "add-hierarchical-label" => {
            parse_kicad_add_label_edit_with_kind(payload, KicadLabelKind::Hierarchical)
        }
        "add-sheet" => parse_kicad_add_sheet_edit(payload),
        "add-text" => parse_kicad_add_text_edit(payload),
        _ => Err(OslError::InvalidInput(format!(
            "unsupported kicad-edit op '{name}'"
        ))),
    }
}

fn parse_kicad_delete_item_edit(payload: &str) -> OslResult<KicadSchematicEdit> {
    let uuid = payload.trim();
    if uuid.is_empty() {
        return Err(OslError::InvalidInput(
            "delete-item expects delete-item:<uuid>".to_string(),
        ));
    }

    Ok(KicadSchematicEdit::DeleteItem {
        uuid: uuid.to_string(),
    })
}

fn parse_kicad_move_item_edit(payload: &str) -> OslResult<KicadSchematicEdit> {
    let (uuid, delta) = payload.rsplit_once(':').ok_or_else(|| {
        OslError::InvalidInput("move-item expects move-item:<uuid>:<dx,dy>".to_string())
    })?;
    let uuid = uuid.trim();
    if uuid.is_empty() {
        return Err(OslError::InvalidInput(
            "move-item expects move-item:<uuid>:<dx,dy>".to_string(),
        ));
    }

    Ok(KicadSchematicEdit::MoveItem {
        uuid: uuid.to_string(),
        delta: parse_kicad_point(delta, "item move delta")?,
    })
}

fn parse_kicad_configure_symbol_edit(payload: &str) -> OslResult<KicadSchematicEdit> {
    let options = split_kicad_configure_symbol_options(payload)?;
    let reference = options.payload.trim();
    if reference.is_empty() {
        return Err(OslError::InvalidInput(
            "configure-symbol expects configure-symbol:<reference>[:unit=<n>][:body-style=<n|none>][:mirror=<x|y|xy|none>][:alt=<pin>=<alternate>[,<pin>=<alternate>...]]"
                .to_string(),
        ));
    }
    if options.unit.is_none()
        && options.body_style.is_none()
        && options.mirror.is_none()
        && options.pin_alternates.is_none()
    {
        return Err(OslError::InvalidInput(
            "configure-symbol requires at least one unit, body-style, mirror, or alt option"
                .to_string(),
        ));
    }

    Ok(KicadSchematicEdit::ConfigureSymbol {
        reference: reference.to_string(),
        unit: options.unit,
        body_style: options.body_style,
        mirror: options.mirror,
        pin_alternates: options.pin_alternates,
    })
}

fn parse_kicad_move_symbol_edit(payload: &str) -> OslResult<KicadSchematicEdit> {
    let parts = payload.split(':').collect::<Vec<_>>();
    if parts.len() < 2 || parts.len() > 3 {
        return Err(OslError::InvalidInput(
            "move-symbol expects move-symbol:<reference>:<x,y>[:rotation]".to_string(),
        ));
    }
    let reference = parts[0].to_string();
    let to = parse_kicad_point(parts[1], "move-symbol target")?;
    let rotation = parts
        .get(2)
        .map(|value| parse_number(value, "move-symbol rotation"))
        .transpose()?;

    Ok(KicadSchematicEdit::MoveSymbol {
        reference,
        to,
        rotation,
    })
}

fn parse_kicad_set_property_edit(payload: &str) -> OslResult<KicadSchematicEdit> {
    let (reference, rest) = payload.split_once(':').ok_or_else(|| {
        OslError::InvalidInput(
            "set-property expects set-property:<reference>:<name>=<value>[:x,y[,rotation]]"
                .to_string(),
        )
    })?;
    let (assignment, at) = match rest.split_once(':') {
        Some((assignment, at)) => (assignment, Some(parse_kicad_at(at, "property position")?)),
        None => (rest, None),
    };
    let (name, value) = assignment.split_once('=').ok_or_else(|| {
        OslError::InvalidInput(
            "set-property expects set-property:<reference>:<name>=<value>[:x,y[,rotation]]"
                .to_string(),
        )
    })?;

    Ok(KicadSchematicEdit::SetSymbolProperty {
        reference: reference.to_string(),
        name: name.to_string(),
        value: value.to_string(),
        at,
    })
}

fn parse_kicad_place_symbol_edit(
    payload: &str,
    symbol_definitions: &[KicadSymbolDef],
) -> OslResult<KicadSchematicEdit> {
    let (payload, uuid) = split_payload_uuid(payload);
    let options = split_kicad_place_symbol_options(payload)?;
    let (rest, at) = options.payload.rsplit_once(':').ok_or_else(|| {
        OslError::InvalidInput(
            "place-symbol expects place-symbol:<lib_id>:<reference>:<value>:<x,y[,rotation]>[:unit=<n>][:body-style=<n>][:alt=<pin>=<alternate>[,<pin>=<alternate>...]]"
                .to_string(),
        )
    })?;
    let (rest, value) = rest.rsplit_once(':').ok_or_else(|| {
        OslError::InvalidInput(
            "place-symbol expects place-symbol:<lib_id>:<reference>:<value>:<x,y[,rotation]>[:unit=<n>][:body-style=<n>][:alt=<pin>=<alternate>[,<pin>=<alternate>...]]"
                .to_string(),
        )
    })?;
    let (lib_id, reference) = rest.rsplit_once(':').ok_or_else(|| {
        OslError::InvalidInput(
            "place-symbol expects place-symbol:<lib_id>:<reference>:<value>:<x,y[,rotation]>[:unit=<n>][:body-style=<n>][:alt=<pin>=<alternate>[,<pin>=<alternate>...]]"
                .to_string(),
        )
    })?;
    let definition = symbol_definitions
        .iter()
        .find(|definition| definition.name == lib_id || definition.local_name() == lib_id)
        .cloned()
        .ok_or_else(|| {
            OslError::InvalidInput(format!(
                "KiCad symbol definition '{lib_id}' was not found; pass --library <file.kicad_sym>"
            ))
        })?;

    Ok(KicadSchematicEdit::PlaceSymbol {
        definition: Box::new(definition),
        reference: reference.to_string(),
        value: value.to_string(),
        at: parse_kicad_at(at, "symbol placement")?,
        unit: Some(options.unit.unwrap_or(1)),
        body_style: options.body_style,
        pin_alternates: options.pin_alternates,
        uuid,
    })
}

fn parse_kicad_add_wire_edit(payload: &str) -> OslResult<KicadSchematicEdit> {
    let (points, uuid) = split_payload_uuid(payload);
    let points = points
        .split(';')
        .map(|point| parse_kicad_point(point, "wire point"))
        .collect::<OslResult<Vec<_>>>()?;
    Ok(KicadSchematicEdit::AddWire { points, uuid })
}

fn parse_kicad_add_bus_edit(payload: &str) -> OslResult<KicadSchematicEdit> {
    let (points, uuid) = split_payload_uuid(payload);
    let points = points
        .split(';')
        .map(|point| parse_kicad_point(point, "bus point"))
        .collect::<OslResult<Vec<_>>>()?;
    Ok(KicadSchematicEdit::AddBus { points, uuid })
}

fn parse_kicad_add_bus_entry_edit(payload: &str) -> OslResult<KicadSchematicEdit> {
    let (payload, uuid) = split_payload_uuid(payload);
    let (at, size) = payload.split_once(':').ok_or_else(|| {
        OslError::InvalidInput("add-bus-entry expects add-bus-entry:<x,y>:<dx,dy>".to_string())
    })?;
    Ok(KicadSchematicEdit::AddBusEntry {
        at: parse_kicad_point(at, "bus entry position")?,
        size: parse_kicad_size(size, "bus entry size")?,
        uuid,
    })
}

fn parse_kicad_add_junction_edit(payload: &str) -> OslResult<KicadSchematicEdit> {
    let (payload, uuid) = split_payload_uuid(payload);
    Ok(KicadSchematicEdit::AddJunction {
        at: parse_kicad_point(payload, "junction position")?,
        uuid,
    })
}

fn parse_kicad_add_no_connect_edit(payload: &str) -> OslResult<KicadSchematicEdit> {
    let (payload, uuid) = split_payload_uuid(payload);
    Ok(KicadSchematicEdit::AddNoConnect {
        at: parse_kicad_point(payload, "no-connect position")?,
        uuid,
    })
}

fn parse_kicad_add_label_edit(payload: &str) -> OslResult<KicadSchematicEdit> {
    parse_kicad_add_label_edit_with_kind(payload, KicadLabelKind::Local)
}

fn parse_kicad_add_label_edit_with_kind(
    payload: &str,
    default_kind: KicadLabelKind,
) -> OslResult<KicadSchematicEdit> {
    let (payload, uuid) = split_payload_uuid(payload);
    let parts = payload.split(':').collect::<Vec<_>>();
    if parts.len() < 2 || parts.len() > 3 {
        return Err(OslError::InvalidInput(
            "add-label expects add-label:<text>:<x,y[,rotation]>[:local|global|hierarchical]"
                .to_string(),
        ));
    }
    let kind = parts
        .get(2)
        .map(|kind| parse_kicad_label_kind(kind))
        .transpose()?
        .unwrap_or(default_kind);
    Ok(KicadSchematicEdit::AddLabel {
        text: parts[0].to_string(),
        kind,
        at: parse_kicad_at(parts[1], "label position")?,
        uuid,
    })
}

fn parse_kicad_add_text_edit(payload: &str) -> OslResult<KicadSchematicEdit> {
    let (payload, uuid) = split_payload_uuid(payload);
    let (text, at) = payload.split_once(':').ok_or_else(|| {
        OslError::InvalidInput("add-text expects add-text:<text>:<x,y[,rotation]>".to_string())
    })?;
    Ok(KicadSchematicEdit::AddText {
        text: text.to_string(),
        at: parse_kicad_at(at, "text position")?,
        uuid,
    })
}

fn parse_kicad_add_sheet_edit(payload: &str) -> OslResult<KicadSchematicEdit> {
    let (payload, uuid) = split_payload_uuid(payload);
    let parts = payload.split(':').collect::<Vec<_>>();
    if parts.len() < 4 || parts.len() > 5 {
        return Err(OslError::InvalidInput(
            "add-sheet expects add-sheet:<name>:<file>:<x,y>:<w,h>[:<pin@x,y[,rotation],type;...>]"
                .to_string(),
        ));
    }
    let pins = parts
        .get(4)
        .filter(|pins| !pins.trim().is_empty())
        .map(|pins| {
            pins.split(';')
                .map(parse_kicad_sheet_pin)
                .collect::<OslResult<Vec<_>>>()
        })
        .transpose()?
        .unwrap_or_default();
    Ok(KicadSchematicEdit::AddSheet {
        name: parts[0].to_string(),
        file: parts[1].to_string(),
        at: sheet_at_from_point(parse_kicad_point(parts[2], "sheet position")?),
        size: parse_kicad_size(parts[3], "sheet size")?,
        pins,
        uuid,
    })
}

fn parse_kicad_sheet_pin(value: &str) -> OslResult<KicadSheetPin> {
    let (name, rest) = value.split_once('@').ok_or_else(|| {
        OslError::InvalidInput("sheet pin expects <name>@<x,y[,rotation],type>".to_string())
    })?;
    let parts = rest.split(',').collect::<Vec<_>>();
    if parts.len() < 3 || parts.len() > 4 {
        return Err(OslError::InvalidInput(
            "sheet pin expects <name>@<x,y[,rotation],type>".to_string(),
        ));
    }
    let pin_type = parts.last().copied().unwrap_or_default().to_string();
    let at = if parts.len() == 3 {
        KicadAt {
            x: parse_number(parts[0], "sheet pin position")?,
            y: parse_number(parts[1], "sheet pin position")?,
            rotation: 0.0,
        }
    } else {
        KicadAt {
            x: parse_number(parts[0], "sheet pin position")?,
            y: parse_number(parts[1], "sheet pin position")?,
            rotation: parse_number(parts[2], "sheet pin rotation")?,
        }
    };
    Ok(KicadSheetPin {
        name: name.to_string(),
        pin_type,
        at: Some(at),
        uuid: None,
        effects: None,
    })
}

fn split_payload_uuid(payload: &str) -> (&str, Option<String>) {
    match payload.rsplit_once(":uuid=") {
        Some((payload, uuid)) => (payload, Some(uuid.to_string())),
        None => (payload, None),
    }
}

struct KicadPlaceSymbolOptions<'a> {
    payload: &'a str,
    unit: Option<u32>,
    body_style: Option<u32>,
    pin_alternates: BTreeMap<String, String>,
}

struct KicadConfigureSymbolOptions<'a> {
    payload: &'a str,
    unit: Option<u32>,
    body_style: Option<Option<u32>>,
    mirror: Option<Option<String>>,
    pin_alternates: Option<BTreeMap<String, String>>,
}

fn split_kicad_place_symbol_options(mut payload: &str) -> OslResult<KicadPlaceSymbolOptions<'_>> {
    let mut unit = None;
    let mut body_style = None;
    let mut pin_alternates = BTreeMap::new();

    while let Some((rest, suffix)) = payload.rsplit_once(':') {
        if let Some(value) = suffix.strip_prefix("unit=") {
            if unit.is_some() {
                return Err(OslError::InvalidInput(
                    "place-symbol unit option was provided more than once".to_string(),
                ));
            }
            unit = Some(parse_positive_u32(value, "symbol unit")?);
            payload = rest;
        } else if let Some(value) = suffix.strip_prefix("body-style=") {
            if body_style.is_some() {
                return Err(OslError::InvalidInput(
                    "place-symbol body-style option was provided more than once".to_string(),
                ));
            }
            body_style = Some(parse_positive_u32(value, "symbol body style")?);
            payload = rest;
        } else if let Some(value) = suffix.strip_prefix("alt=") {
            if !pin_alternates.is_empty() {
                return Err(OslError::InvalidInput(
                    "place-symbol alt option was provided more than once".to_string(),
                ));
            }
            pin_alternates = parse_kicad_pin_alternates(value)?;
            payload = rest;
        } else {
            break;
        }
    }

    Ok(KicadPlaceSymbolOptions {
        payload,
        unit,
        body_style,
        pin_alternates,
    })
}

fn split_kicad_configure_symbol_options(
    mut payload: &str,
) -> OslResult<KicadConfigureSymbolOptions<'_>> {
    let mut unit = None;
    let mut body_style = None;
    let mut mirror = None;
    let mut pin_alternates = None;

    while let Some((rest, suffix)) = payload.rsplit_once(':') {
        if let Some(value) = suffix.strip_prefix("unit=") {
            if unit.is_some() {
                return Err(OslError::InvalidInput(
                    "configure-symbol unit option was provided more than once".to_string(),
                ));
            }
            unit = Some(parse_positive_u32(value, "symbol unit")?);
            payload = rest;
        } else if let Some(value) = suffix.strip_prefix("body-style=") {
            if body_style.is_some() {
                return Err(OslError::InvalidInput(
                    "configure-symbol body-style option was provided more than once".to_string(),
                ));
            }
            body_style = Some(parse_optional_positive_u32(value, "symbol body style")?);
            payload = rest;
        } else if let Some(value) = suffix.strip_prefix("mirror=") {
            if mirror.is_some() {
                return Err(OslError::InvalidInput(
                    "configure-symbol mirror option was provided more than once".to_string(),
                ));
            }
            mirror = Some(normalize_symbol_mirror(value)?);
            payload = rest;
        } else if let Some(value) = suffix.strip_prefix("alt=") {
            if pin_alternates.is_some() {
                return Err(OslError::InvalidInput(
                    "configure-symbol alt option was provided more than once".to_string(),
                ));
            }
            pin_alternates = Some(parse_kicad_pin_alternates(value)?);
            payload = rest;
        } else {
            break;
        }
    }

    Ok(KicadConfigureSymbolOptions {
        payload,
        unit,
        body_style,
        mirror,
        pin_alternates,
    })
}

fn parse_kicad_pin_alternates(value: &str) -> OslResult<BTreeMap<String, String>> {
    if value.trim().is_empty() {
        return Err(OslError::InvalidInput(
            "place-symbol alt expects <pin>=<alternate>[,<pin>=<alternate>...]".to_string(),
        ));
    }

    let mut alternates = BTreeMap::new();
    for entry in value.split(',') {
        let (pin, alternate) = entry.split_once('=').ok_or_else(|| {
            OslError::InvalidInput(
                "place-symbol alt expects <pin>=<alternate>[,<pin>=<alternate>...]".to_string(),
            )
        })?;
        if pin.trim().is_empty() || alternate.trim().is_empty() {
            return Err(OslError::InvalidInput(
                "place-symbol alt pin and alternate names must not be empty".to_string(),
            ));
        }
        if alternates
            .insert(pin.to_string(), alternate.to_string())
            .is_some()
        {
            return Err(OslError::InvalidInput(format!(
                "place-symbol alt pin '{pin}' was provided more than once"
            )));
        }
    }

    Ok(alternates)
}

fn parse_kicad_point(value: &str, context: &str) -> OslResult<KicadPoint> {
    let parts = value.split(',').collect::<Vec<_>>();
    if parts.len() != 2 {
        return Err(OslError::InvalidInput(format!(
            "{context} expects x,y coordinates"
        )));
    }
    Ok(KicadPoint {
        x: parse_number(parts[0], context)?,
        y: parse_number(parts[1], context)?,
    })
}

fn sheet_at_from_point(point: KicadPoint) -> KicadAt {
    KicadAt {
        x: point.x,
        y: point.y,
        rotation: 0.0,
    }
}

fn parse_kicad_at(value: &str, context: &str) -> OslResult<KicadAt> {
    let parts = value.split(',').collect::<Vec<_>>();
    if !(2..=3).contains(&parts.len()) {
        return Err(OslError::InvalidInput(format!(
            "{context} expects x,y or x,y,rotation"
        )));
    }
    Ok(KicadAt {
        x: parse_number(parts[0], context)?,
        y: parse_number(parts[1], context)?,
        rotation: parts
            .get(2)
            .map(|value| parse_number(value, context))
            .transpose()?
            .unwrap_or(0.0),
    })
}

fn parse_kicad_size(value: &str, context: &str) -> OslResult<KicadSize> {
    let parts = value.split(',').collect::<Vec<_>>();
    if parts.len() != 2 {
        return Err(OslError::InvalidInput(format!(
            "{context} expects width,height"
        )));
    }
    Ok(KicadSize {
        width: parse_number(parts[0], context)?,
        height: parse_number(parts[1], context)?,
    })
}

fn parse_kicad_label_kind(value: &str) -> OslResult<KicadLabelKind> {
    match value {
        "local" => Ok(KicadLabelKind::Local),
        "global" => Ok(KicadLabelKind::Global),
        "hierarchical" => Ok(KicadLabelKind::Hierarchical),
        _ => Err(OslError::InvalidInput(format!(
            "unsupported KiCad label kind '{value}'"
        ))),
    }
}

fn parse_number(value: &str, context: &str) -> OslResult<f64> {
    let value = unquote(value);
    let value = value.trim();
    if value.is_empty() {
        return Err(OslError::InvalidInput(format!("{context} is empty")));
    }

    if let Ok(value) = value.parse::<f64>() {
        return Ok(value);
    }

    let split_at = value
        .char_indices()
        .find(|(_, character)| {
            !character.is_ascii_digit()
                && *character != '.'
                && *character != '-'
                && *character != '+'
                && *character != 'e'
                && *character != 'E'
        })
        .map(|(index, _)| index)
        .unwrap_or(value.len());

    let (number, suffix) = value.split_at(split_at);
    if number.is_empty() || suffix.is_empty() {
        return Err(OslError::InvalidInput(format!(
            "invalid {context}: '{}' is not a number",
            value
        )));
    }

    let number = number.parse::<f64>().map_err(|_| {
        OslError::InvalidInput(format!(
            "invalid {context}: '{}' is not a floating point number",
            value
        ))
    })?;
    let multiplier = spice_suffix_multiplier(suffix).ok_or_else(|| {
        OslError::InvalidInput(format!(
            "invalid {context}: unknown numeric suffix '{}'",
            suffix
        ))
    })?;
    Ok(number * multiplier)
}

fn spice_suffix_multiplier(suffix: &str) -> Option<f64> {
    match suffix.to_ascii_lowercase().as_str() {
        "t" => Some(1e12),
        "g" => Some(1e9),
        "meg" => Some(1e6),
        "k" => Some(1e3),
        "m" | "ms" => Some(1e-3),
        "u" | "us" => Some(1e-6),
        "n" | "ns" => Some(1e-9),
        "p" | "ps" => Some(1e-12),
        "f" | "fs" => Some(1e-15),
        "s" => Some(1.0),
        _ => None,
    }
}

fn option_f64_json(value: Option<f64>) -> String {
    value
        .map(|value| {
            if value.is_finite() {
                value.to_string()
            } else {
                "null".to_string()
            }
        })
        .unwrap_or_else(|| "null".to_string())
}

fn option_f64_text(value: Option<f64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "none".to_string())
}

fn summary_json(summary: Option<WaveformSummary>) -> String {
    match summary {
        Some(summary) => format!(
            concat!(
                "{{ \"samples\": {}, \"first\": {}, \"last\": {}, \"min\": {}, ",
                "\"max\": {}, \"avg\": {}, \"pp\": {}, \"rms\": {} }}"
            ),
            summary.samples,
            summary.first,
            summary.last,
            summary.min,
            summary.max,
            summary.avg,
            summary.peak_to_peak,
            summary.rms
        ),
        None => "null".to_string(),
    }
}

fn summary_text(summary: Option<WaveformSummary>) -> String {
    match summary {
        Some(summary) => format!(
            "samples={} first={} last={} min={} max={} avg={} pp={} rms={}",
            summary.samples,
            summary.first,
            summary.last,
            summary.min,
            summary.max,
            summary.avg,
            summary.peak_to_peak,
            summary.rms
        ),
        None => "summary unavailable".to_string(),
    }
}

fn window_text(from: Option<f64>, to: Option<f64>) -> String {
    match (from, to) {
        (Some(from), Some(to)) => format!("{}..{}", from, to),
        (Some(from), None) => format!("{}..", from),
        (None, Some(to)) => format!("..{}", to),
        (None, None) => "all".to_string(),
    }
}

fn parameters_text(parameters: &[ParameterOverride]) -> String {
    if parameters.is_empty() {
        "none".to_string()
    } else {
        parameters
            .iter()
            .map(|parameter| format!("{}={}", parameter.name, parameter.value))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn format_parameter_value(value: f64) -> String {
    let mut text = value.to_string();
    text = text.replace('-', "m");
    text = text.replace('.', "p");
    text = text.replace('+', "");
    text
}

fn unquote(value: &str) -> String {
    let value = value.trim();
    if value.len() >= 2 {
        let bytes = value.as_bytes();
        if (bytes[0] == b'"' && bytes[value.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[value.len() - 1] == b'\'')
        {
            return value[1..value.len() - 1].to_string();
        }
    }
    value.to_string()
}

fn sanitize_name(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    for character in input.chars() {
        if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
            output.push(character);
        } else {
            output.push('_');
        }
    }
    if output.is_empty() {
        "run".to_string()
    } else {
        output
    }
}

fn default_run_dir(input: &str) -> PathBuf {
    let stem = Path::new(input)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(sanitize_name)
        .unwrap_or_else(|| "run".to_string());
    PathBuf::from("runs").join(make_run_id(&stem))
}

fn find_circuits(root: &Path) -> OslResult<Vec<PathBuf>> {
    let mut circuits = Vec::new();
    find_circuits_inner(root, &mut circuits)?;
    circuits.sort();
    Ok(circuits)
}

fn find_circuits_inner(path: &Path, circuits: &mut Vec<PathBuf>) -> OslResult<()> {
    if path.is_file() {
        if path.extension().and_then(|ext| ext.to_str()) == Some("cir") {
            circuits.push(path.to_path_buf());
        }
        return Ok(());
    }

    for entry in
        fs::read_dir(path).map_err(|err| OslError::io(format!("read {}", path.display()), err))?
    {
        let entry = entry.map_err(|err| OslError::io("read directory entry", err))?;
        let entry_path = entry.path();
        if entry_path.is_dir() {
            find_circuits_inner(&entry_path, circuits)?;
        } else if entry_path.extension().and_then(|ext| ext.to_str()) == Some("cir") {
            circuits.push(entry_path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        SweepDimension, VerifyConfig, VerifyRun, flag_value, has_flag, parse_kicad_edit_ops,
        parse_number, parse_positive_u32, positionals,
    };
    use osl_kicad::{KicadLabelKind, KicadSchematicEdit, parse_kicad_symbol_library};
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
                "delete-item:22222222-2222-2222-2222-222222222222",
            ]
        );

        let edits = parse_kicad_edit_ops(&args, &[]).unwrap();
        assert_eq!(edits.len(), 9);
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
                reference,
                value,
                at,
                unit,
                body_style,
                pin_alternates,
                ..
            } => {
                assert_eq!(definition.name, "NekoSpice:C");
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
