
#[derive(Debug)]
struct RawHeader {
    title: String,
    plot_name: String,
    flags: Vec<String>,
    expected_point_count: Option<usize>,
    variables: Vec<WaveformVariable>,
}

pub fn read_ngspice_raw(path: &Path) -> OslResult<Waveform> {
    let content =
        fs::read(path).map_err(|err| OslError::io(format!("read {}", path.display()), err))?;
    parse_ngspice_raw(&content, &path.display().to_string())
}

pub fn parse_ngspice_raw(input: &[u8], source_name: &str) -> OslResult<Waveform> {
    if find_section_payload_offset(input, b"Binary:").is_some() {
        return parse_ngspice_binary_raw(input, source_name);
    }

    let content = str::from_utf8(input).map_err(|err| {
        OslError::InvalidInput(format!(
            "{} is not valid UTF-8 ASCII raw data: {}",
            source_name, err
        ))
    })?;
    parse_ngspice_ascii_raw(content, source_name)
}

pub fn read_ngspice_ascii_raw(path: &Path) -> OslResult<Waveform> {
    let content = read_text(path)?;
    parse_ngspice_ascii_raw(&content, &path.display().to_string())
}

pub fn parse_ngspice_ascii_raw(input: &str, source_name: &str) -> OslResult<Waveform> {
    let mut lines = input.lines().enumerate().peekable();
    let header = parse_header(&mut lines, source_name)?;

    let mut found_values = false;
    while let Some((_, raw_line)) = lines.peek() {
        if raw_line.trim() == "Values:" {
            lines.next();
            found_values = true;
            break;
        }
        lines.next();
    }

    if !found_values {
        return Err(OslError::InvalidInput(format!(
            "{} does not contain ngspice Values section",
            source_name
        )));
    }

    let columns = parse_values(
        &mut lines,
        source_name,
        header.variables.len(),
        header.expected_point_count,
    )?;

    Ok(Waveform {
        title: header.title,
        plot_name: header.plot_name,
        variables: header.variables,
        columns,
    })
}

pub fn parse_ngspice_binary_raw(input: &[u8], source_name: &str) -> OslResult<Waveform> {
    let payload_offset = find_section_payload_offset(input, b"Binary:").ok_or_else(|| {
        OslError::InvalidInput(format!(
            "{} does not contain ngspice Binary section",
            source_name
        ))
    })?;
    let header_text = str::from_utf8(&input[..payload_offset]).map_err(|err| {
        OslError::InvalidInput(format!(
            "{} header is not valid UTF-8 before binary payload: {}",
            source_name, err
        ))
    })?;
    let mut lines = header_text.lines().enumerate().peekable();
    let header = parse_header(&mut lines, source_name)?;

    if header.flags.iter().any(|flag| flag == "complex") {
        return Err(OslError::InvalidInput(format!(
            "{} contains complex binary raw data, which is not supported yet",
            source_name
        )));
    }

    let variable_count = header.variables.len();
    if variable_count == 0 {
        return Err(OslError::InvalidInput(format!(
            "{} does not declare any binary raw variables",
            source_name
        )));
    }

    let payload = &input[payload_offset..];
    if !payload.len().is_multiple_of(8) {
        return Err(OslError::InvalidInput(format!(
            "{} binary payload length {} at byte offset {} is not aligned to f64 values",
            source_name,
            payload.len(),
            payload_offset
        )));
    }

    let value_count = payload.len() / 8;
    if !value_count.is_multiple_of(variable_count) {
        return Err(OslError::InvalidInput(format!(
            "{} binary payload contains {} f64 values, which is not divisible by {} variables",
            source_name, value_count, variable_count
        )));
    }

    let point_count = value_count / variable_count;
    if let Some(expected) = header.expected_point_count
        && expected != point_count
    {
        return Err(OslError::InvalidInput(format!(
            "{} declares {} points but binary payload contains {}",
            source_name, expected, point_count
        )));
    }

    let mut columns = vec![Vec::<f64>::with_capacity(point_count); variable_count];
    for point_index in 0..point_count {
        for (variable_index, column) in columns.iter_mut().enumerate() {
            let value_index = point_index * variable_count + variable_index;
            let byte_offset = payload_offset + value_index * 8;
            let bytes = input[byte_offset..byte_offset + 8]
                .try_into()
                .expect("binary payload was checked for f64 alignment");
            column.push(f64::from_le_bytes(bytes));
        }
    }

    Ok(Waveform {
        title: header.title,
        plot_name: header.plot_name,
        variables: header.variables,
        columns,
    })
}

fn parse_header<'a, I>(
    lines: &mut std::iter::Peekable<I>,
    source_name: &str,
) -> OslResult<RawHeader>
where
    I: Iterator<Item = (usize, &'a str)>,
{
    let mut title = String::new();
    let mut plot_name = String::new();
    let mut flags = Vec::new();
    let mut expected_variable_count = None;
    let mut expected_point_count = None;
    let mut variables = Vec::new();

    while let Some((line_index, raw_line)) = lines.next() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }

        if let Some(value) = strip_header(line, "Title:") {
            title = value.to_string();
            continue;
        }
        if let Some(value) = strip_header(line, "Plotname:") {
            plot_name = value.to_string();
            continue;
        }
        if let Some(value) = strip_header(line, "Flags:") {
            flags = value
                .split_whitespace()
                .map(|flag| flag.to_ascii_lowercase())
                .collect();
            continue;
        }
        if let Some(value) = strip_header(line, "No. Variables:") {
            expected_variable_count = Some(parse_usize(value, source_name, line_index + 1)?);
            continue;
        }
        if let Some(value) = strip_header(line, "No. Points:") {
            expected_point_count = Some(parse_usize(value, source_name, line_index + 1)?);
            continue;
        }
        if line == "Variables:" {
            parse_variables(lines, source_name, expected_variable_count, &mut variables)?;
            break;
        }
    }

    if variables.is_empty() {
        return Err(OslError::InvalidInput(format!(
            "{} does not contain ngspice Variables section",
            source_name
        )));
    }

    Ok(RawHeader {
        title,
        plot_name,
        flags,
        expected_point_count,
        variables,
    })
}

fn parse_variables<'a, I>(
    lines: &mut std::iter::Peekable<I>,
    source_name: &str,
    expected_variable_count: Option<usize>,
    variables: &mut Vec<WaveformVariable>,
) -> OslResult<()>
where
    I: Iterator<Item = (usize, &'a str)>,
{
    while let Some((line_index, raw_line)) = lines.peek().copied() {
        let line = raw_line.trim();
        if line.is_empty() {
            lines.next();
            continue;
        }
        if line == "Values:" || line == "Binary:" {
            break;
        }

        let parts = line.split_whitespace().collect::<Vec<_>>();
        if parts.len() < 3 {
            return Err(OslError::InvalidInput(format!(
                "{} line {} is not a valid variable row: {}",
                source_name,
                line_index + 1,
                line
            )));
        }

        let index = parse_usize(parts[0], source_name, line_index + 1)?;
        variables.push(WaveformVariable {
            index,
            name: parts[1].to_string(),
            unit: parts[2].to_string(),
        });
        lines.next();

        if let Some(expected) = expected_variable_count
            && variables.len() == expected
        {
            break;
        }
    }

    if let Some(expected) = expected_variable_count
        && variables.len() != expected
    {
        return Err(OslError::InvalidInput(format!(
            "{} declares {} variables but contains {} variable rows",
            source_name,
            expected,
            variables.len()
        )));
    }

    Ok(())
}

fn parse_values<'a, I>(
    lines: &mut std::iter::Peekable<I>,
    source_name: &str,
    variable_count: usize,
    expected_point_count: Option<usize>,
) -> OslResult<Vec<Vec<f64>>>
where
    I: Iterator<Item = (usize, &'a str)>,
{
    let mut columns = vec![Vec::<f64>::new(); variable_count];
    let mut current = Vec::<f64>::with_capacity(variable_count);
    let mut current_point_index = None::<usize>;

    for (line_index, raw_line) in lines.by_ref() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }

        let parts = line.split_whitespace().collect::<Vec<_>>();
        if current.is_empty() {
            if parts.len() < 2 {
                return Err(OslError::InvalidInput(format!(
                    "{} line {} should start a point with '<index> <value>'",
                    source_name,
                    line_index + 1
                )));
            }
            current_point_index = Some(parse_usize(parts[0], source_name, line_index + 1)?);
            current.push(parse_f64(parts[1], source_name, line_index + 1)?);
            for value in parts.iter().skip(2) {
                current.push(parse_f64(value, source_name, line_index + 1)?);
            }
        } else {
            for value in parts {
                current.push(parse_f64(value, source_name, line_index + 1)?);
            }
        }

        if current.len() == variable_count {
            for (column, value) in columns.iter_mut().zip(current.drain(..)) {
                column.push(value);
            }
            current_point_index = None;
        } else if current.len() > variable_count {
            return Err(OslError::InvalidInput(format!(
                "{} line {} has too many values for point {:?}",
                source_name,
                line_index + 1,
                current_point_index
            )));
        }
    }

    if !current.is_empty() {
        return Err(OslError::InvalidInput(format!(
            "{} ended in the middle of point {:?}",
            source_name, current_point_index
        )));
    }

    let point_count = columns.first().map(Vec::len).unwrap_or(0);
    if let Some(expected) = expected_point_count
        && point_count != expected
    {
        return Err(OslError::InvalidInput(format!(
            "{} declares {} points but parsed {}",
            source_name, expected, point_count
        )));
    }

    Ok(columns)
}

fn strip_header<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    line.strip_prefix(key).map(str::trim)
}

fn find_section_payload_offset(input: &[u8], marker: &[u8]) -> Option<usize> {
    let mut line_start = 0;
    while line_start < input.len() {
        let line_end = input[line_start..]
            .iter()
            .position(|byte| *byte == b'\n')
            .map(|offset| line_start + offset)
            .unwrap_or(input.len());
        let line = trim_ascii(&input[line_start..line_end]);
        if line == marker {
            return Some((line_end + 1).min(input.len()));
        }
        line_start = line_end.saturating_add(1);
    }
    None
}

fn trim_ascii(mut input: &[u8]) -> &[u8] {
    while let Some((first, rest)) = input.split_first()
        && first.is_ascii_whitespace()
    {
        input = rest;
    }
    while let Some((last, rest)) = input.split_last()
        && last.is_ascii_whitespace()
    {
        input = rest;
    }
    input
}

fn parse_usize(input: &str, source_name: &str, line: usize) -> OslResult<usize> {
    input.parse::<usize>().map_err(|_| {
        OslError::InvalidInput(format!(
            "{} line {} has invalid integer '{}'",
            source_name, line, input
        ))
    })
}

fn parse_f64(input: &str, source_name: &str, line: usize) -> OslResult<f64> {
    input.parse::<f64>().map_err(|_| {
        OslError::InvalidInput(format!(
            "{} line {} has invalid floating point value '{}'",
            source_name, line, input
        ))
    })
}

fn normalize_signal(input: &str) -> String {
    input.trim().to_ascii_lowercase()
}

fn csv_escape(input: &str) -> String {
    if input.contains(',') || input.contains('"') || input.contains('\n') || input.contains('\r') {
        format!("\"{}\"", input.replace('"', "\"\""))
    } else {
        input.to_string()
    }
}

fn format_f64_csv(value: f64) -> String {
    if value.is_finite() {
        value.to_string()
    } else if value.is_nan() {
        "NaN".to_string()
    } else if value.is_sign_positive() {
        "Infinity".to_string()
    } else {
        "-Infinity".to_string()
    }
}

fn f64_json(value: f64) -> String {
    if value.is_finite() {
        value.to_string()
    } else {
        "null".to_string()
    }
}

fn option_f64_json(value: Option<f64>) -> String {
    value.map(f64_json).unwrap_or_else(|| "null".to_string())
}

