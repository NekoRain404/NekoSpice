
fn suggested_signal_priority(signal: &str) -> (u8, String) {
    let lowered = signal.to_ascii_lowercase();
    let rank = if lowered == "v(out)" {
        0
    } else if lowered.contains("out") {
        1
    } else if lowered == "v(in)" {
        2
    } else if lowered.starts_with("v(") {
        3
    } else if lowered.starts_with("i(") {
        4
    } else {
        5
    };
    (rank, lowered)
}

fn suggested_check_kind(analysis: AnalysisKind, signal: &str) -> &'static str {
    let is_current = signal.to_ascii_lowercase().starts_with("i(");
    match (analysis, is_current) {
        (AnalysisKind::Transient, true) => "rms",
        (AnalysisKind::Transient, false) => "avg",
        (AnalysisKind::Ac, _) => "max",
        _ => "final_value",
    }
}

fn suggested_check_name(signal: &str, kind: &str) -> String {
    format!("{}_{}", signal_name_slug(signal), kind)
}

fn signal_name_slug(signal: &str) -> String {
    signal
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .split('_')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}

fn suggested_checks_yaml(checks: &[SuggestedCheck]) -> String {
    if checks.is_empty() {
        return concat!(
            "    # Suggested checks: no observable node or source-current signals were found.\n",
            "    # Add checks after confirming the imported netlist produces waveform data.\n"
        )
        .to_string();
    }

    let mut output = concat!(
        "    # Suggested checks to customize after the first run:\n",
        "    #   Keep checks empty for import smoke tests, then copy entries below\n",
        "    #   into checks with tuned min/max limits once waveform-summary.json is known.\n"
    )
    .to_string();
    for check in checks {
        output.push_str(&format!("    #   - name: {}\n", yaml_scalar(&check.name)));
        output.push_str(&format!("    #     kind: {}\n", yaml_scalar(&check.kind)));
        output.push_str(&format!(
            "    #     signal: {}\n",
            yaml_scalar(&check.signal)
        ));
        output.push_str("    #     min: TODO\n");
        output.push_str("    #     max: TODO\n");
    }
    output
}

fn spice_logical_lines(content: &str) -> Vec<(usize, String)> {
    let mut logical_lines = Vec::new();
    let mut current = None::<(usize, String)>;

    for (index, raw_line) in content.lines().enumerate() {
        let line_number = index + 1;
        let trimmed_start = raw_line.trim_start();
        if trimmed_start.starts_with('+') {
            let continuation = trimmed_start.trim_start_matches('+').trim();
            if let Some((_, line)) = current.as_mut() {
                line.push(' ');
                line.push_str(continuation);
            } else {
                current = Some((line_number, continuation.to_string()));
            }
            continue;
        }

        if let Some(line) = current.take() {
            logical_lines.push(line);
        }
        current = Some((line_number, raw_line.to_string()));
    }

    if let Some(line) = current {
        logical_lines.push(line);
    }

    logical_lines
}

fn normalized_spice_statement(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('*') || trimmed.starts_with(';') {
        return None;
    }
    let without_inline_comment = trimmed
        .split_once(';')
        .map(|(statement, _)| statement)
        .unwrap_or(trimmed)
        .trim();
    if without_inline_comment.is_empty() {
        None
    } else {
        Some(without_inline_comment.to_string())
    }
}

fn quoted_json_list(values: &[String]) -> String {
    values
        .iter()
        .map(|value| format!("\"{}\"", json_escape(value)))
        .collect::<Vec<_>>()
        .join(", ")
}

fn option_string_json(value: Option<&str>) -> String {
    value
        .map(|value| format!("\"{}\"", json_escape(value)))
        .unwrap_or_else(|| "null".to_string())
}

fn normalized_project_name(source: &str) -> String {
    Path::new(source)
        .file_stem()
        .and_then(|name| name.to_str())
        .map(sanitize_identifier)
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "imported_project".to_string())
}

fn sanitize_identifier(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    for character in input.chars() {
        if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
            output.push(character);
        } else {
            output.push('_');
        }
    }
    if output.is_empty() {
        "imported_project".to_string()
    } else {
        output
    }
}

fn normalize_imported_netlist(source: &str, dependencies: &[NormalizedDependency]) -> String {
    let mut output = source
        .lines()
        .map(|line| rewrite_dependency_line(line, dependencies))
        .collect::<Vec<_>>()
        .join("\n");
    output.push('\n');
    output
}

fn rewrite_dependency_line(line: &str, dependencies: &[NormalizedDependency]) -> String {
    let trimmed_start = line.trim_start();
    let indent_len = line.len() - trimmed_start.len();
    let indent = &line[..indent_len];
    let tokens = trimmed_start.split_whitespace().collect::<Vec<_>>();
    if tokens.len() < 2 {
        return line.to_string();
    }
    let directive = tokens[0].to_ascii_lowercase();
    if !matches!(directive.as_str(), ".include" | ".inc" | ".lib") {
        return line.to_string();
    }

    let raw_path = tokens[1];
    let path = raw_path.trim_matches('"').trim_matches('\'');
    let Some(dependency) = dependencies
        .iter()
        .find(|dependency| dependency.source == path)
    else {
        return line.to_string();
    };

    let suffix = tokens
        .iter()
        .skip(2)
        .map(|token| format!(" {}", token))
        .collect::<String>();
    format!(
        "{}{} \"{}\"{}",
        indent, tokens[0], dependency.project_path, suffix
    )
}

fn yaml_scalar(value: &str) -> String {
    if value.chars().all(|character| {
        character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.' | '/')
    }) && !value.is_empty()
    {
        value.to_string()
    } else {
        format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
    }
}

