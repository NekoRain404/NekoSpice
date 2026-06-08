use osl_core::{OslError, OslResult, read_text};
use std::path::Path;

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

pub fn read_ngspice_ascii_raw(path: &Path) -> OslResult<Waveform> {
    let content = read_text(path)?;
    parse_ngspice_ascii_raw(&content, &path.display().to_string())
}

pub fn parse_ngspice_ascii_raw(input: &str, source_name: &str) -> OslResult<Waveform> {
    let mut title = String::new();
    let mut plot_name = String::new();
    let mut expected_variable_count = None;
    let mut expected_point_count = None;
    let mut variables = Vec::new();
    let mut lines = input.lines().enumerate().peekable();

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
        if let Some(value) = strip_header(line, "No. Variables:") {
            expected_variable_count = Some(parse_usize(value, source_name, line_index + 1)?);
            continue;
        }
        if let Some(value) = strip_header(line, "No. Points:") {
            expected_point_count = Some(parse_usize(value, source_name, line_index + 1)?);
            continue;
        }
        if line == "Variables:" {
            parse_variables(
                &mut lines,
                source_name,
                expected_variable_count,
                &mut variables,
            )?;
            break;
        }
    }

    if variables.is_empty() {
        return Err(OslError::InvalidInput(format!(
            "{} does not contain ngspice Variables section",
            source_name
        )));
    }

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
        variables.len(),
        expected_point_count,
    )?;

    Ok(Waveform {
        title,
        plot_name,
        variables,
        columns,
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
        if line == "Values:" {
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
    }
}
