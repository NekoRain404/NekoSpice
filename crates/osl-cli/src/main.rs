use osl_core::{
    OslError, OslResult, ParameterOverride, RunMetadata, RunStatus, html_escape, json_escape,
    make_run_id, parameters_json, read_text, write_text,
};
use osl_sim::{NgspiceCliBackend, SimulatorBackend};
use osl_waveform::{MeasurementKind, measure, read_ngspice_ascii_raw};
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
    let metadata = backend.run(Path::new(input), &output_dir)?;
    write_single_run_report(&output_dir, &metadata)?;

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
        let metadata = backend.run(&circuit, &run_dir)?;
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

fn report_command(args: &[String]) -> OslResult<i32> {
    let dir = PathBuf::from(positional(args, 0, "missing directory for 'osl report'")?);
    let run_json = dir.join("run.json");
    let verify_json = dir.join("verify.json");
    let bench_json = dir.join("bench.json");

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

    if verify_json.is_file() || bench_json.is_file() {
        let source = if verify_json.is_file() {
            verify_json
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
        "{} does not contain run.json, verify.json, or bench.json",
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
    min: Option<f64>,
    max: Option<f64>,
}

impl VerifyConfig {
    fn parse(path: &Path) -> OslResult<Self> {
        let content = read_text(path)?;
        let base_dir = path.parent().unwrap_or_else(|| Path::new("."));
        let mut project = path
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or("verification")
            .to_string();
        let mut runs = Vec::new();
        let mut current: Option<VerifyRun> = None;
        let mut current_check: Option<VerifyCheck> = None;
        let mut in_runs = false;
        let mut in_sweep = false;
        let mut in_checks = false;

        for raw_line in content.lines() {
            let indent = raw_line
                .chars()
                .take_while(|character| *character == ' ')
                .count();
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some(value) = strip_key(line, "project:") {
                project = unquote(value);
                continue;
            }

            if line == "runs:" {
                in_runs = true;
                continue;
            }

            if !in_runs {
                continue;
            }

            if indent <= 2
                && let Some(value) = strip_key(line, "- name:")
            {
                if let Some(check) = current_check.take() {
                    push_check(&mut current, check)?;
                }
                if let Some(run) = current.take() {
                    runs.push(validate_run(run)?);
                }
                current = Some(VerifyRun {
                    name: unquote(value),
                    netlist: PathBuf::new(),
                    sweep: Vec::new(),
                    checks: Vec::new(),
                });
                in_sweep = false;
                in_checks = false;
                continue;
            }

            if let Some(value) = strip_key(line, "netlist:") {
                let Some(run) = current.as_mut() else {
                    return Err(OslError::InvalidInput(format!(
                        "{} has netlist before run name",
                        path.display()
                    )));
                };
                let netlist = PathBuf::from(unquote(value));
                run.netlist = if netlist.is_absolute() {
                    netlist
                } else {
                    base_dir.join(netlist)
                };
                continue;
            }

            if line == "sweep:" {
                if current.is_none() {
                    return Err(OslError::InvalidInput(format!(
                        "{} has sweep before run name",
                        path.display()
                    )));
                }
                in_sweep = true;
                in_checks = false;
                continue;
            }

            if line == "checks:" {
                if current.is_none() {
                    return Err(OslError::InvalidInput(format!(
                        "{} has checks before run name",
                        path.display()
                    )));
                }
                in_sweep = false;
                in_checks = true;
                continue;
            }

            if in_sweep && let Some((name, values)) = parse_sweep_line(line, path)? {
                let Some(run) = current.as_mut() else {
                    return Err(OslError::InvalidInput(
                        "sweep entry appears before a verify run".to_string(),
                    ));
                };
                run.sweep.push(SweepDimension { name, values });
                continue;
            }

            if in_checks
                && indent >= 6
                && let Some(value) = strip_key(line, "- name:")
            {
                if let Some(check) = current_check.take() {
                    push_check(&mut current, check)?;
                }
                current_check = Some(VerifyCheck {
                    name: unquote(value),
                    kind: "final_value".to_string(),
                    signal: String::new(),
                    min: None,
                    max: None,
                });
                continue;
            }

            if in_checks && let Some(check) = current_check.as_mut() {
                if let Some(value) = strip_key(line, "kind:") {
                    check.kind = unquote(value);
                    continue;
                }
                if let Some(value) = strip_key(line, "signal:") {
                    check.signal = unquote(value);
                    continue;
                }
                if let Some(value) = strip_key(line, "min:") {
                    check.min = Some(parse_f64(value, "check min")?);
                    continue;
                }
                if let Some(value) = strip_key(line, "max:") {
                    check.max = Some(parse_f64(value, "check max")?);
                    continue;
                }
            }
        }

        if let Some(check) = current_check.take() {
            push_check(&mut current, check)?;
        }
        if let Some(run) = current.take() {
            runs.push(validate_run(run)?);
        }

        if runs.is_empty() {
            return Err(OslError::InvalidInput(format!(
                "{} must contain runs with name and netlist",
                path.display()
            )));
        }

        Ok(Self { project, runs })
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

fn parse_sweep_line(line: &str, path: &Path) -> OslResult<Option<(String, Vec<f64>)>> {
    let Some((name, raw_values)) = line.split_once(':') else {
        return Ok(None);
    };
    let name = name.trim();
    if name.is_empty() {
        return Err(OslError::InvalidInput(format!(
            "{} has sweep entry with empty parameter name",
            path.display()
        )));
    }
    Ok(Some((
        name.to_string(),
        parse_f64_list(raw_values, "sweep values")?,
    )))
}

fn parse_f64_list(value: &str, context: &str) -> OslResult<Vec<f64>> {
    let value = value.trim();
    if !value.starts_with('[') || !value.ends_with(']') {
        return Err(OslError::InvalidInput(format!(
            "{context} must use [a, b, c] syntax"
        )));
    }
    let inner = &value[1..value.len() - 1];
    if inner.trim().is_empty() {
        return Ok(Vec::new());
    }

    inner
        .split(',')
        .map(|part| parse_f64(part.trim(), context))
        .collect()
}

fn push_check(current: &mut Option<VerifyRun>, check: VerifyCheck) -> OslResult<()> {
    let Some(run) = current.as_mut() else {
        return Err(OslError::InvalidInput(
            "check appears before a verify run".to_string(),
        ));
    };
    run.checks.push(validate_check(check)?);
    Ok(())
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
    let metadata = backend.run_with_parameters(&task.netlist_path, &run_dir, &task.parameters)?;
    let checks = evaluate_checks(&run_dir, &task.checks);
    write_single_run_report(&run_dir, &metadata)?;

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
    let waveform = match read_ngspice_ascii_raw(&waveform_path) {
        Ok(waveform) => waveform,
        Err(error) => {
            return CheckResult {
                name: check.name.clone(),
                kind: check.kind.clone(),
                signal: check.signal.clone(),
                value: None,
                min: check.min,
                max: check.max,
                passed: false,
                message: error.to_string(),
            };
        }
    };
    let values = match waveform.signal_values(&check.signal) {
        Ok(values) => values,
        Err(error) => {
            return CheckResult {
                name: check.name.clone(),
                kind: check.kind.clone(),
                signal: check.signal.clone(),
                value: None,
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
                value: None,
                min: check.min,
                max: check.max,
                passed: false,
                message: error.to_string(),
            };
        }
    };
    let value = match measure(kind, values) {
        Ok(value) => value,
        Err(error) => {
            return CheckResult {
                name: check.name.clone(),
                kind: check.kind.clone(),
                signal: check.signal.clone(),
                value: None,
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
        "value={} min={} max={}",
        value,
        option_f64_text(check.min),
        option_f64_text(check.max)
    );

    CheckResult {
        name: check.name.clone(),
        kind: check.kind.clone(),
        signal: check.signal.clone(),
        value: Some(value),
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
}

#[derive(Debug)]
struct CheckResult {
    name: String,
    kind: String,
    signal: String,
    value: Option<f64>,
    min: Option<f64>,
    max: Option<f64>,
    passed: bool,
    message: String,
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

    fn to_json(&self) -> String {
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
                                "\"value\": {}, \"min\": {}, \"max\": {}, \"passed\": {}, \"message\": \"{}\" }}"
                            ),
                            json_escape(&check.name),
                            json_escape(&check.kind),
                            json_escape(&check.signal),
                            option_f64_json(check.value),
                            option_f64_json(check.min),
                            option_f64_json(check.max),
                            check.passed,
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
                "  \"runs\": [\n",
                "{}\n",
                "  ]\n",
                "}}\n"
            ),
            json_escape(&self.project),
            self.passed_count(),
            self.failed_count(),
            runs
        )
    }

    fn to_html(&self) -> String {
        let rows = self
            .results
            .iter()
            .map(|result| {
                let status = result.status().as_str();
                let parameters = if result.parameters.is_empty() {
                    "none".to_string()
                } else {
                    result
                        .parameters
                        .iter()
                        .map(|parameter| format!("{}={}", parameter.name, parameter.value))
                        .collect::<Vec<_>>()
                        .join(", ")
                };
                let checks = if result.checks.is_empty() {
                    "no checks".to_string()
                } else {
                    result
                        .checks
                        .iter()
                        .map(|check| {
                            format!(
                                "{}: {} ({})",
                                check.name,
                                if check.passed { "pass" } else { "fail" },
                                check.message
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("; ")
                };
                format!(
                    concat!(
                        "<tr class=\"{}\"><td>{}</td><td>{}</td><td>{}</td>",
                        "<td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>"
                    ),
                    status,
                    html_escape(status),
                    html_escape(&result.name),
                    html_escape(&result.netlist),
                    html_escape(&parameters),
                    result.metadata.duration_ms,
                    html_escape(&result.run_dir),
                    html_escape(&checks)
                )
            })
            .collect::<String>();

        format!(
            concat!(
                "<!doctype html><html><head><meta charset=\"utf-8\">",
                "<title>NekoSpice Verification Report</title>{}</head><body>",
                "<main><h1>{}</h1>",
                "<section class=\"summary\"><strong>{}</strong> passed, <strong>{}</strong> failed</section>",
                "<table><thead><tr><th>Status</th><th>Run</th><th>Netlist</th><th>Parameters</th><th>ms</th><th>Output</th><th>Checks</th></tr></thead>",
                "<tbody>{}</tbody></table>",
                "</main></body></html>\n"
            ),
            report_css(),
            html_escape(&self.project),
            self.passed_count(),
            self.failed_count(),
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
                "<li><code>{}</code> <span>{}</span></li>",
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
    let jobs = value.parse::<usize>().map_err(|_| {
        OslError::InvalidInput(format!(
            "--jobs expects a positive integer, got '{}'",
            value
        ))
    })?;
    if jobs == 0 {
        return Err(OslError::InvalidInput(
            "--jobs expects a positive integer, got 0".to_string(),
        ));
    }
    Ok(jobs)
}

fn strip_key<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    line.strip_prefix(key).map(str::trim)
}

fn parse_f64(value: &str, context: &str) -> OslResult<f64> {
    let value = unquote(value);
    value.parse::<f64>().map_err(|_| {
        OslError::InvalidInput(format!(
            "invalid {context}: '{}' is not a floating point number",
            value
        ))
    })
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
    use super::{SweepDimension, VerifyRun};
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
}
