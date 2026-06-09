use osl_waveform::{WaveformSummary, read_ngspice_raw};
use std::path::{Path, PathBuf};

const WAVEFORM_RAW_FILE: &str = "waveform.raw";
const MAX_DISPLAY_VARIABLES: usize = 8;

#[derive(Debug, Clone)]
pub(crate) enum GuiWaveformSummaryState {
    Ready(GuiWaveformSummary),
    Missing { raw_path: PathBuf },
    Error { raw_path: PathBuf, message: String },
}

impl GuiWaveformSummaryState {
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
    pub(crate) peak_to_peak: f64,
    pub(crate) rms: f64,
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
                peak_to_peak: summary.peak_to_peak,
                rms: summary.rms,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    Ok(GuiWaveformSummary {
        raw_path: raw_path.to_path_buf(),
        title: waveform.title().to_string(),
        plot_name: waveform.plot_name().to_string(),
        point_count: waveform.point_count(),
        variable_count,
        omitted_variable_count: variable_count.saturating_sub(variables.len()),
        variables,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use osl_core::write_text;
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
            osl_core::now_unix_ms()
        ))
    }
}
