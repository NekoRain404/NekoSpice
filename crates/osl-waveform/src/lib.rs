//! Waveform data library — raw file parsing, measurement computation, CSV export, and FFT.

pub mod fft;
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
    /// title。
    pub fn title(&self) -> &str {
        &self.title
    }

    /// plot name。
    pub fn plot_name(&self) -> &str {
        &self.plot_name
    }

    /// variables。
    pub fn variables(&self) -> &[WaveformVariable] {
        &self.variables
    }

    /// point count。
    pub fn point_count(&self) -> usize {
        self.columns.first().map(Vec::len).unwrap_or(0)
    }

    /// signal values。
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

    /// signal values in window。
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

    /// to csv。
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

    /// to summary json。
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

    /// viewport envelope。
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
    /// new。
    pub fn new(signal: impl Into<String>, max_points: usize) -> Self {
        Self {
            signal: signal.into(),
            from: None,
            to: None,
            max_points,
        }
    }

    /// with window。
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
    /// to json。
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
    /// parse。
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
    /// summarize。
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

/// measure。
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

// Raw ngspice parser (binary + ASCII).
include!("raw_parser_impl.rs");

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
