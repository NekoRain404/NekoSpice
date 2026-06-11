// Verify command infrastructure.
// Covers: VerifyConfig, VerifyRun, VerifyTask, VerifyCheck,
// YAML parsing, validation, task execution, output generation.

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

fn finalize_run_output(output_dir: &Path, metadata: &mut RunMetadata) -> OslResult<()> {
    finalize_run_artifacts(output_dir, metadata)
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

pub(crate) fn trailing_positionals(args: &[String], skip: usize) -> Vec<&str> {
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

pub(crate) fn parse_positive_u32(value: &str, flag: &str) -> OslResult<u32> {
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

pub(crate) fn parse_optional_positive_u32(value: &str, flag: &str) -> OslResult<Option<u32>> {
    if value.eq_ignore_ascii_case("none") || value.eq_ignore_ascii_case("default") {
        Ok(None)
    } else {
        parse_positive_u32(value, flag).map(Some)
    }
}

pub(crate) fn parse_number(value: &str, context: &str) -> OslResult<f64> {
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

fn option_f64_text(value: Option<f64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "none".to_string())
}

fn window_text(from: Option<f64>, to: Option<f64>) -> String {
    match (from, to) {
        (Some(from), Some(to)) => format!("{}..{}", from, to),
        (Some(from), None) => format!("{}..", from),
        (None, Some(to)) => format!("..{}", to),
        (None, None) => "all".to_string(),
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

