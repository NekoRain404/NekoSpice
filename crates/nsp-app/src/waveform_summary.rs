//! 波形汇总数据结构。
//!
use nsp_waveform::{
    Waveform, WaveformEnvelopeBucket, WaveformSummary, WaveformViewportQuery, read_ngspice_raw,
};
use std::path::{Path, PathBuf};

const WAVEFORM_RAW_FILE: &str = "waveform.raw";
const MAX_DISPLAY_VARIABLES: usize = 8;
const MAX_PREVIEW_SIGNALS: usize = 32;
const MAX_PREVIEW_BUCKETS: usize = 96;

#[derive(Debug, Clone)]
pub(crate) enum GuiWaveformSummaryState {
    Ready(GuiWaveformSummary),
    Missing { raw_path: PathBuf },
    Error { raw_path: PathBuf, message: String },
}

impl GuiWaveformSummaryState {
    /// from run dir。
    pub(crate) fn from_run_dir(output_dir: &Path) -> Self {
        let raw_path = output_dir.join(WAVEFORM_RAW_FILE);
        if !raw_path.is_file() {
            return Self::Missing { raw_path };
        }

        match summarize_raw(&raw_path) {
            Ok(summary) => Self::Ready(summary),
            Err(message) => Self::Error { raw_path, message },
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct GuiWaveformSummary {
    pub(crate) raw_path: PathBuf,
    pub(crate) title: String,
    pub(crate) plot_name: String,
    pub(crate) point_count: usize,
    pub(crate) variable_count: usize,
    pub(crate) variables: Vec<GuiWaveformVariableSummary>,
    pub(crate) omitted_variable_count: usize,
    pub(crate) previews: Vec<GuiWaveformPreview>,
    pub(crate) omitted_preview_count: usize,
}

impl GuiWaveformSummary {
    /// default signal name。
    pub(crate) fn default_signal_name(&self) -> Option<&str> {
        self.previews.first().map(|preview| preview.signal.as_str())
    }

    /// has preview signal。
    pub(crate) fn has_preview_signal(&self, signal: &str) -> bool {
        self.preview_for_signal(signal).is_some()
    }

    /// preview for signal。
    pub(crate) fn preview_for_signal(&self, signal: &str) -> Option<&GuiWaveformPreview> {
        self.previews
            .iter()
            .find(|preview| same_signal(&preview.signal, signal))
    }

    /// variable summary for signal。
    pub(crate) fn variable_summary_for_signal(
        &self,
        signal: &str,
    ) -> Option<&GuiWaveformVariableSummary> {
        self.variables
            .iter()
            .find(|variable| same_signal(&variable.name, signal))
    }
}

#[derive(Debug, Clone)]
pub(crate) struct GuiWaveformVariableSummary {
    pub(crate) name: String,
    pub(crate) unit: String,
    pub(crate) samples: usize,
    pub(crate) first: f64,
    pub(crate) last: f64,
    pub(crate) min: f64,
    pub(crate) max: f64,
    pub(crate) avg: f64,
    pub(crate) peak_to_peak: f64,
    pub(crate) rms: f64,
}

#[derive(Debug, Clone)]
pub(crate) struct GuiWaveformPreview {
    pub(crate) signal: String,
    pub(crate) unit: String,
    pub(crate) source_points: usize,
    pub(crate) time_min: f64,
    pub(crate) time_max: f64,
    pub(crate) value_min: f64,
    pub(crate) value_max: f64,
    pub(crate) buckets: Vec<GuiWaveformPreviewBucket>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct GuiWaveformPreviewBucket {
    pub(crate) start_time: f64,
    pub(crate) end_time: f64,
    pub(crate) min: f64,
    pub(crate) max: f64,
    pub(crate) samples: usize,
}

fn summarize_raw(raw_path: &Path) -> Result<GuiWaveformSummary, String> {
    let waveform = read_ngspice_raw(raw_path).map_err(|error| error.to_string())?;
    let variable_count = waveform.variables().len();
    let variables = waveform
        .variables()
        .iter()
        .take(MAX_DISPLAY_VARIABLES)
        .map(|variable| {
            let values = waveform
                .signal_values(&variable.name)
                .map_err(|error| error.to_string())?;
            let summary = WaveformSummary::summarize(values).map_err(|error| error.to_string())?;
            Ok(GuiWaveformVariableSummary {
                name: variable.name.clone(),
                unit: variable.unit.clone(),
                samples: summary.samples,
                first: summary.first,
                last: summary.last,
                min: summary.min,
                max: summary.max,
                avg: summary.avg,
                peak_to_peak: summary.peak_to_peak,
                rms: summary.rms,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let preview_candidates = waveform
        .variables()
        .iter()
        .filter(|variable| !is_time_signal(&variable.name))
        .collect::<Vec<_>>();
    let previews = preview_candidates
        .iter()
        .take(MAX_PREVIEW_SIGNALS)
        .filter_map(|variable| preview_for_signal(&waveform, &variable.name, &variable.unit))
        .collect::<Vec<_>>();

    Ok(GuiWaveformSummary {
        raw_path: raw_path.to_path_buf(),
        title: waveform.title().to_string(),
        plot_name: waveform.plot_name().to_string(),
        point_count: waveform.point_count(),
        variable_count,
        omitted_variable_count: variable_count.saturating_sub(variables.len()),
        variables,
        omitted_preview_count: preview_candidates.len().saturating_sub(previews.len()),
        previews,
    })
}

fn preview_for_signal(waveform: &Waveform, signal: &str, unit: &str) -> Option<GuiWaveformPreview> {
    let envelope = waveform
        .viewport_envelope(&WaveformViewportQuery::new(signal, MAX_PREVIEW_BUCKETS))
        .ok()?;
    if envelope.buckets.is_empty() {
        return None;
    }

    let buckets = envelope
        .buckets
        .iter()
        .map(preview_bucket_from_envelope)
        .collect::<Vec<_>>();
    let (time_min, time_max, value_min, value_max) = preview_bounds(&buckets)?;

    Some(GuiWaveformPreview {
        signal: signal.to_string(),
        unit: unit.to_string(),
        source_points: envelope.source_points,
        time_min,
        time_max,
        value_min,
        value_max,
        buckets,
    })
}

fn preview_bucket_from_envelope(bucket: &WaveformEnvelopeBucket) -> GuiWaveformPreviewBucket {
    GuiWaveformPreviewBucket {
        start_time: bucket.start_time,
        end_time: bucket.end_time,
        min: bucket.min,
        max: bucket.max,
        samples: bucket.samples,
    }
}

fn preview_bounds(buckets: &[GuiWaveformPreviewBucket]) -> Option<(f64, f64, f64, f64)> {
    let first = buckets.first()?;
    let mut time_min = first.start_time.min(first.end_time);
    let mut time_max = first.start_time.max(first.end_time);
    let mut value_min = first.min.min(first.max);
    let mut value_max = first.min.max(first.max);
    for bucket in buckets.iter().skip(1) {
        time_min = time_min.min(bucket.start_time.min(bucket.end_time));
        time_max = time_max.max(bucket.start_time.max(bucket.end_time));
        value_min = value_min.min(bucket.min.min(bucket.max));
        value_max = value_max.max(bucket.min.max(bucket.max));
    }
    Some((time_min, time_max, value_min, value_max))
}

fn is_time_signal(signal: &str) -> bool {
    same_signal(signal, "time")
}

fn same_signal(left: &str, right: &str) -> bool {
    left.trim().eq_ignore_ascii_case(right.trim())
}

#[cfg(test)]
mod tests {
    use super::*;
    use nsp_core::write_text;
    use std::fs;

    const SAMPLE_RAW: &str = r#"
Title: gui demo
Plotname: Transient Analysis
Flags: real
No. Variables: 2
No. Points: 2
Variables:
	0	time	time
	1	v(out)	voltage
Values:
 0	0.000000000000000e+00
	2.000000000000000e+00

 1	1.000000000000000e-06
	4.000000000000000e+00
"#;

    #[test]
    fn summarizes_gui_waveform_raw() {
        let output_dir = temp_output_dir("summary");
        write_text(&output_dir.join(WAVEFORM_RAW_FILE), SAMPLE_RAW).unwrap();

        let state = GuiWaveformSummaryState::from_run_dir(&output_dir);

        let GuiWaveformSummaryState::Ready(summary) = state else {
            panic!("expected ready waveform summary");
        };
        assert_eq!(summary.title, "gui demo");
        assert_eq!(summary.point_count, 2);
        assert_eq!(summary.variable_count, 2);
        assert_eq!(summary.variables[1].name, "v(out)");
        assert_eq!(summary.variables[1].last, 4.0);
        assert_eq!(summary.variables[1].avg, 3.0);
        assert_eq!(
            summary.variable_summary_for_signal("V(OUT)").unwrap().max,
            4.0
        );
        assert_eq!(summary.default_signal_name(), Some("v(out)"));
        assert_eq!(summary.previews.len(), 1);
        assert_eq!(summary.previews[0].source_points, 2);
        assert_eq!(summary.previews[0].value_min, 2.0);
        assert_eq!(summary.previews[0].value_max, 4.0);
        let _ = fs::remove_dir_all(output_dir);
    }

    #[test]
    fn reports_missing_gui_waveform_raw() {
        let output_dir = temp_output_dir("missing");

        let state = GuiWaveformSummaryState::from_run_dir(&output_dir);

        let GuiWaveformSummaryState::Missing { raw_path } = state else {
            panic!("expected missing waveform summary");
        };
        assert!(raw_path.ends_with(WAVEFORM_RAW_FILE));
        let _ = fs::remove_dir_all(output_dir);
    }

    fn temp_output_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "nekospice_gui_waveform_{name}_{}_{}",
            std::process::id(),
            nsp_core::now_unix_ms()
        ))
    }
}
