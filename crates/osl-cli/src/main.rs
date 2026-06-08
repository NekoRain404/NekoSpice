use osl_core::{
    Artifact, OslError, OslResult, ParameterOverride, RunMetadata, RunStatus, html_escape,
    json_escape, make_run_id, parameters_json, read_text, write_text,
};
use osl_model::{ModelCheckOptions, ModelCheckReport};
use osl_netlist::{ImportReport, NormalizedDependency, read_import_input};
use osl_sim::{NgspiceCliBackend, SimulatorBackend};
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
  osl import <spice-netlist-or-ltspice.asc> [--output <dir>]
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
    let report = import.report;
    write_text(&output_dir.join("import.json"), &report.to_json())?;
    write_text(
        &output_dir.join("report.html"),
        &report.to_html(report_css()),
    )?;
    let project_dir = output_dir.join("project");
    let dependencies = copy_import_dependencies(&report, input_path, &project_dir)?;
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
    args.iter()
        .filter(|arg| !arg.starts_with("--"))
        .nth(index)
        .map(String::as_str)
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
    use super::{SweepDimension, VerifyConfig, VerifyRun, parse_number};
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
