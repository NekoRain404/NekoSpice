use osl_core::{OslError, OslResult, json_escape, read_text};
use std::fs;
use std::path::Path;
use std::str;

#[derive(Debug, Clone)]
pub struct Waveform {
    title: String,
    plot_name: String,
    variables: Vec<WaveformVariable>,
    columns: Vec<Vec<f64>>,
}

impl Waveform {
    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn plot_name(&self) -> &str {
        &self.plot_name
    }

    pub fn variables(&self) -> &[WaveformVariable] {
        &self.variables
    }

    pub fn point_count(&self) -> usize {
        self.columns.first().map(Vec::len).unwrap_or(0)
    }

    pub fn signal_values(&self, signal: &str) -> OslResult<&[f64]> {
        let target = normalize_signal(signal);
        let index = self
            .variables
            .iter()
            .position(|variable| normalize_signal(&variable.name) == target)
            .ok_or_else(|| {
                OslError::InvalidInput(format!(
                    "signal '{}' not found. Available signals: {}",
                    signal,
                    self.variables
                        .iter()
                        .map(|variable| variable.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ))
            })?;

        self.columns.get(index).map(Vec::as_slice).ok_or_else(|| {
            OslError::InvalidInput(format!(
                "waveform is missing values for signal '{}'",
                self.variables[index].name
            ))
        })
    }

    pub fn signal_values_in_window(
        &self,
        signal: &str,
        from: Option<f64>,
        to: Option<f64>,
    ) -> OslResult<Vec<f64>> {
        if from.is_none() && to.is_none() {
            return Ok(self.signal_values(signal)?.to_vec());
        }
        if let (Some(from), Some(to)) = (from, to)
            && from > to
        {
            return Err(OslError::InvalidInput(format!(
                "invalid measurement window: from={} is greater than to={}",
                from, to
            )));
        }

        let time = self.signal_values("time")?;
        let values = self.signal_values(signal)?;
        let selected = time
            .iter()
            .copied()
            .zip(values.iter().copied())
            .filter(|(time, _)| from.is_none_or(|from| *time >= from))
            .filter(|(time, _)| to.is_none_or(|to| *time <= to))
            .map(|(_, value)| value)
            .collect::<Vec<_>>();

        if selected.is_empty() {
            return Err(OslError::InvalidInput(format!(
                "measurement window for '{}' contains no samples",
                signal
            )));
        }

        Ok(selected)
    }

    pub fn to_csv(&self) -> OslResult<String> {
        self.validate_column_lengths()?;

        let mut output = String::new();
        output.push_str(
            &self
                .variables
                .iter()
                .map(|variable| csv_escape(&variable.name))
                .collect::<Vec<_>>()
                .join(","),
        );
        output.push('\n');

        for point_index in 0..self.point_count() {
            let row = self
                .columns
                .iter()
                .map(|column| format_f64_csv(column[point_index]))
                .collect::<Vec<_>>()
                .join(",");
            output.push_str(&row);
            output.push('\n');
        }

        Ok(output)
    }

    pub fn to_summary_json(&self) -> OslResult<String> {
        self.validate_column_lengths()?;

        let variables = self
            .variables
            .iter()
            .zip(self.columns.iter())
            .map(|(variable, values)| {
                let summary = WaveformSummary::summarize(values)?;
                Ok(format!(
                    concat!(
                        "    {{ \"index\": {}, \"name\": \"{}\", \"unit\": \"{}\", ",
                        "\"samples\": {}, \"first\": {}, \"last\": {}, \"min\": {}, ",
                        "\"max\": {}, \"avg\": {}, \"pp\": {}, \"rms\": {} }}"
                    ),
                    variable.index,
                    json_escape(&variable.name),
                    json_escape(&variable.unit),
                    summary.samples,
                    f64_json(summary.first),
                    f64_json(summary.last),
                    f64_json(summary.min),
                    f64_json(summary.max),
                    f64_json(summary.avg),
                    f64_json(summary.peak_to_peak),
                    f64_json(summary.rms)
                ))
            })
            .collect::<OslResult<Vec<_>>>()?
            .join(",\n");

        Ok(format!(
            concat!(
                "{{\n",
                "  \"schema_version\": 1,\n",
                "  \"title\": \"{}\",\n",
                "  \"plot_name\": \"{}\",\n",
                "  \"point_count\": {},\n",
                "  \"variable_count\": {},\n",
                "  \"variables\": [\n",
                "{}\n",
                "  ]\n",
                "}}\n"
            ),
            json_escape(&self.title),
            json_escape(&self.plot_name),
            self.point_count(),
            self.variables.len(),
            variables
        ))
    }

    pub fn viewport_envelope(&self, query: &WaveformViewportQuery) -> OslResult<WaveformEnvelope> {
        self.validate_column_lengths()?;
        if query.max_points == 0 {
            return Err(OslError::InvalidInput(
                "viewport max_points must be greater than 0".to_string(),
            ));
        }
        if let (Some(from), Some(to)) = (query.from, query.to)
            && from > to
        {
            return Err(OslError::InvalidInput(format!(
                "invalid viewport window: from={} is greater than to={}",
                from, to
            )));
        }

        let time = self.signal_values("time")?;
        let values = self.signal_values(&query.signal)?;
        let selected = time
            .iter()
            .copied()
            .zip(values.iter().copied())
            .filter(|(time, _)| query.from.is_none_or(|from| *time >= from))
            .filter(|(time, _)| query.to.is_none_or(|to| *time <= to))
            .collect::<Vec<_>>();

        if selected.is_empty() {
            return Err(OslError::InvalidInput(format!(
                "viewport for '{}' contains no samples",
                query.signal
            )));
        }

        let bucket_count = selected.len().min(query.max_points);
        let mut buckets = Vec::with_capacity(bucket_count);
        for bucket_index in 0..bucket_count {
            let start = bucket_index * selected.len() / bucket_count;
            let end = ((bucket_index + 1) * selected.len() / bucket_count).max(start + 1);
            let slice = &selected[start..end];
            let mut min_value = f64::INFINITY;
            let mut max_value = f64::NEG_INFINITY;
            for (_, value) in slice {
                min_value = min_value.min(*value);
                max_value = max_value.max(*value);
            }
            buckets.push(WaveformEnvelopeBucket {
                start_time: slice[0].0,
                end_time: slice[slice.len() - 1].0,
                min: min_value,
                max: max_value,
                samples: slice.len(),
            });
        }

        Ok(WaveformEnvelope {
            signal: query.signal.clone(),
            from: query.from,
            to: query.to,
            source_points: selected.len(),
            max_points: query.max_points,
            buckets,
        })
    }

    fn validate_column_lengths(&self) -> OslResult<()> {
        if self.variables.len() != self.columns.len() {
            return Err(OslError::InvalidInput(format!(
                "waveform has {} variables but {} value columns",
                self.variables.len(),
                self.columns.len()
            )));
        }

        let Some((first, rest)) = self.columns.split_first() else {
            return Ok(());
        };
        let expected = first.len();
        for (index, column) in rest.iter().enumerate() {
            if column.len() != expected {
                return Err(OslError::InvalidInput(format!(
                    "waveform column {} has {} points but expected {}",
                    index + 1,
                    column.len(),
                    expected
                )));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct WaveformViewportQuery {
    pub signal: String,
    pub from: Option<f64>,
    pub to: Option<f64>,
    pub max_points: usize,
}

impl WaveformViewportQuery {
    pub fn new(signal: impl Into<String>, max_points: usize) -> Self {
        Self {
            signal: signal.into(),
            from: None,
            to: None,
            max_points,
        }
    }

    pub fn with_window(mut self, from: Option<f64>, to: Option<f64>) -> Self {
        self.from = from;
        self.to = to;
        self
    }
}

#[derive(Debug, Clone)]
pub struct WaveformEnvelope {
    pub signal: String,
    pub from: Option<f64>,
    pub to: Option<f64>,
    pub source_points: usize,
    pub max_points: usize,
    pub buckets: Vec<WaveformEnvelopeBucket>,
}

impl WaveformEnvelope {
    pub fn to_json(&self) -> String {
        let buckets = self
            .buckets
            .iter()
            .map(|bucket| {
                format!(
                    concat!(
                        "    {{ \"start_time\": {}, \"end_time\": {}, ",
                        "\"min\": {}, \"max\": {}, \"samples\": {} }}"
                    ),
                    f64_json(bucket.start_time),
                    f64_json(bucket.end_time),
                    f64_json(bucket.min),
                    f64_json(bucket.max),
                    bucket.samples
                )
            })
            .collect::<Vec<_>>()
            .join(",\n");

        format!(
            concat!(
                "{{\n",
                "  \"schema_version\": 1,\n",
                "  \"signal\": \"{}\",\n",
                "  \"from\": {},\n",
                "  \"to\": {},\n",
                "  \"source_points\": {},\n",
                "  \"max_points\": {},\n",
                "  \"bucket_count\": {},\n",
                "  \"buckets\": [\n",
                "{}\n",
                "  ]\n",
                "}}\n"
            ),
            json_escape(&self.signal),
            option_f64_json(self.from),
            option_f64_json(self.to),
            self.source_points,
            self.max_points,
            self.buckets.len(),
            buckets
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub struct WaveformEnvelopeBucket {
    pub start_time: f64,
    pub end_time: f64,
    pub min: f64,
    pub max: f64,
    pub samples: usize,
}

#[derive(Debug, Clone)]
pub struct WaveformVariable {
    pub index: usize,
    pub name: String,
    pub unit: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeasurementKind {
    FinalValue,
    Avg,
    Min,
    Max,
    PeakToPeak,
    Rms,
}

impl MeasurementKind {
    pub fn parse(input: &str) -> OslResult<Self> {
        match input {
            "final_value" => Ok(Self::FinalValue),
            "avg" => Ok(Self::Avg),
            "min" => Ok(Self::Min),
            "max" => Ok(Self::Max),
            "pp" => Ok(Self::PeakToPeak),
            "rms" => Ok(Self::Rms),
            _ => Err(OslError::InvalidInput(format!(
                "unsupported measurement kind '{}'; supported: final_value, avg, min, max, pp, rms",
                input
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct WaveformSummary {
    pub samples: usize,
    pub first: f64,
    pub last: f64,
    pub min: f64,
    pub max: f64,
    pub avg: f64,
    pub peak_to_peak: f64,
    pub rms: f64,
}

impl WaveformSummary {
    pub fn summarize(values: &[f64]) -> OslResult<Self> {
        if values.is_empty() {
            return Err(OslError::InvalidInput(
                "cannot summarize an empty waveform signal".to_string(),
            ));
        }

        Ok(Self {
            samples: values.len(),
            first: values[0],
            last: values[values.len() - 1],
            min: measure(MeasurementKind::Min, values)?,
            max: measure(MeasurementKind::Max, values)?,
            avg: measure(MeasurementKind::Avg, values)?,
            peak_to_peak: measure(MeasurementKind::PeakToPeak, values)?,
            rms: measure(MeasurementKind::Rms, values)?,
        })
    }
}

pub fn measure(kind: MeasurementKind, values: &[f64]) -> OslResult<f64> {
    if values.is_empty() {
        return Err(OslError::InvalidInput(
            "waveform signal has no samples".to_string(),
        ));
    }

    match kind {
        MeasurementKind::FinalValue => values
            .last()
            .copied()
            .ok_or_else(|| OslError::InvalidInput("waveform signal has no samples".to_string())),
        MeasurementKind::Avg => Ok(values.iter().sum::<f64>() / values.len() as f64),
        MeasurementKind::Min => Ok(values
            .iter()
            .copied()
            .fold(f64::INFINITY, |best, value| best.min(value))),
        MeasurementKind::Max => Ok(values
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, |best, value| best.max(value))),
        MeasurementKind::PeakToPeak => {
            let min = measure(MeasurementKind::Min, values)?;
            let max = measure(MeasurementKind::Max, values)?;
            Ok(max - min)
        }
        MeasurementKind::Rms => {
            let mean_square =
                values.iter().map(|value| value * value).sum::<f64>() / values.len() as f64;
            Ok(mean_square.sqrt())
        }
    }
}

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
    if payload.len() % 8 != 0 {
        return Err(OslError::InvalidInput(format!(
            "{} binary payload length {} at byte offset {} is not aligned to f64 values",
            source_name,
            payload.len(),
            payload_offset
        )));
    }

    let value_count = payload.len() / 8;
    if value_count % variable_count != 0 {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ngspice_ascii_raw() {
        let raw = r#"
Title: demo
Plotname: Transient Analysis
Flags: real
No. Variables: 3
No. Points: 2
Variables:
	0	time	time
	1	v(in)	voltage
	2	v(out)	voltage
Values:
 0	0.000000000000000e+00
	1.000000000000000e+00
	2.000000000000000e+00

 1	1.000000000000000e-06
	3.000000000000000e+00
	4.000000000000000e+00
"#;

        let waveform = parse_ngspice_ascii_raw(raw, "test.raw").unwrap();

        assert_eq!(waveform.title(), "demo");
        assert_eq!(waveform.plot_name(), "Transient Analysis");
        assert_eq!(waveform.point_count(), 2);
        assert_eq!(waveform.variables()[1].name, "v(in)");
        assert_eq!(waveform.signal_values("V(OUT)").unwrap(), &[2.0, 4.0]);
        assert_eq!(
            measure(
                MeasurementKind::PeakToPeak,
                waveform.signal_values("v(out)").unwrap()
            )
            .unwrap(),
            2.0
        );
        assert_eq!(
            waveform
                .signal_values_in_window("v(out)", Some(0.5e-6), Some(1.5e-6))
                .unwrap(),
            vec![4.0]
        );
        let summary =
            WaveformSummary::summarize(waveform.signal_values("v(out)").unwrap()).unwrap();
        assert_eq!(summary.samples, 2);
        assert_eq!(summary.first, 2.0);
        assert_eq!(summary.last, 4.0);
        assert_eq!(summary.min, 2.0);
        assert_eq!(summary.max, 4.0);
        assert_eq!(summary.peak_to_peak, 2.0);

        let csv = waveform.to_csv().unwrap();
        assert_eq!(csv, "time,v(in),v(out)\n0,1,2\n0.000001,3,4\n");

        let summary_json = waveform.to_summary_json().unwrap();
        assert!(summary_json.contains("\"point_count\": 2"));
        assert!(summary_json.contains("\"name\": \"v(out)\""));
        assert!(summary_json.contains("\"pp\": 2"));

        let query = WaveformViewportQuery::new("v(out)", 1).with_window(Some(0.0), Some(1.0e-6));
        let envelope = waveform.viewport_envelope(&query).unwrap();
        assert_eq!(envelope.signal, "v(out)");
        assert_eq!(envelope.source_points, 2);
        assert_eq!(envelope.buckets.len(), 1);
        assert_eq!(envelope.buckets[0].start_time, 0.0);
        assert_eq!(envelope.buckets[0].end_time, 1.0e-6);
        assert_eq!(envelope.buckets[0].min, 2.0);
        assert_eq!(envelope.buckets[0].max, 4.0);

        let envelope_json = envelope.to_json();
        assert!(envelope_json.contains("\"bucket_count\": 1"));
        assert!(envelope_json.contains("\"source_points\": 2"));
    }

    #[test]
    fn parses_ngspice_binary_raw() {
        let mut raw = b"Title: demo\n\
Date: today\n\
Plotname: Transient Analysis\n\
Flags: real\n\
No. Variables: 3\n\
No. Points: 2\n\
Variables:\n\
\t0\ttime\ttime\n\
\t1\tv(in)\tvoltage\n\
\t2\tv(out)\tvoltage\n\
Binary:\n"
            .to_vec();
        for value in [0.0_f64, 1.0, 2.0, 1.0e-6, 3.0, 4.0] {
            raw.extend_from_slice(&value.to_le_bytes());
        }

        let waveform = parse_ngspice_raw(&raw, "binary.raw").unwrap();

        assert_eq!(waveform.title(), "demo");
        assert_eq!(waveform.plot_name(), "Transient Analysis");
        assert_eq!(waveform.point_count(), 2);
        assert_eq!(waveform.signal_values("time").unwrap(), &[0.0, 1.0e-6]);
        assert_eq!(waveform.signal_values("v(in)").unwrap(), &[1.0, 3.0]);
        assert_eq!(waveform.signal_values("v(out)").unwrap(), &[2.0, 4.0]);
        assert_eq!(
            waveform
                .signal_values_in_window("v(out)", Some(0.5e-6), None)
                .unwrap(),
            vec![4.0]
        );
    }

    #[test]
    fn rejects_misaligned_binary_payload() {
        let mut raw = b"Title: demo\n\
Plotname: Transient Analysis\n\
Flags: real\n\
No. Variables: 1\n\
No. Points: 1\n\
Variables:\n\
\t0\ttime\ttime\n\
Binary:\n"
            .to_vec();
        raw.extend_from_slice(&[1, 2, 3]);

        let error = parse_ngspice_binary_raw(&raw, "bad.raw").unwrap_err();

        assert!(error.to_string().contains("not aligned to f64 values"));
    }

    #[test]
    fn rejects_empty_viewport() {
        let raw = r#"
Title: demo
Plotname: Transient Analysis
Flags: real
No. Variables: 2
No. Points: 1
Variables:
	0	time	time
	1	v(out)	voltage
Values:
 0	0.000000000000000e+00
	2.000000000000000e+00
"#;
        let waveform = parse_ngspice_ascii_raw(raw, "test.raw").unwrap();
        let query = WaveformViewportQuery::new("v(out)", 10).with_window(Some(1.0), Some(2.0));

        let error = waveform.viewport_envelope(&query).unwrap_err();

        assert!(error.to_string().contains("contains no samples"));
    }
}
