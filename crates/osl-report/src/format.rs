use osl_core::ParameterOverride;
use osl_waveform::WaveformSummary;

/// option f64 json。
pub(crate) fn option_f64_json(value: Option<f64>) -> String {
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

/// option f64 text。
pub(crate) fn option_f64_text(value: Option<f64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "none".to_string())
}

/// summary json。
pub(crate) fn summary_json(summary: Option<WaveformSummary>) -> String {
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

/// summary text。
pub(crate) fn summary_text(summary: Option<WaveformSummary>) -> String {
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

/// parameters text。
pub(crate) fn parameters_text(parameters: &[ParameterOverride]) -> String {
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

/// junit seconds。
pub(crate) fn junit_seconds(duration_ms: u128) -> String {
    format!("{:.6}", duration_ms as f64 / 1000.0)
}

/// xml escape。
pub(crate) fn xml_escape(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());
    for character in input.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            character => escaped.push(character),
        }
    }
    escaped
}

/// cdata escape。
pub(crate) fn cdata_escape(input: &str) -> String {
    input.replace("]]>", "]]]]><![CDATA[>")
}

/// markdown cell。
pub(crate) fn markdown_cell(input: &str) -> String {
    markdown_inline(input)
        .replace('|', "\\|")
        .replace('\n', "<br>")
}

/// markdown link cell。
pub(crate) fn markdown_link_cell(input: &str) -> String {
    input.replace('|', "\\|").replace('\n', "<br>")
}

/// markdown inline。
pub(crate) fn markdown_inline(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    for character in input.chars() {
        match character {
            '\\' => output.push_str("\\\\"),
            '`' => output.push_str("\\`"),
            character => output.push(character),
        }
    }
    output
}

/// markdown link。
pub(crate) fn markdown_link(label: &str, href: &str) -> String {
    format!(
        "[{}]({})",
        markdown_inline(label),
        href.replace(')', "%29").replace(' ', "%20")
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn markdown_cells_escape_table_separators() {
        assert_eq!(super::markdown_cell("a|b\nc"), "a\\|b<br>c");
    }
}
