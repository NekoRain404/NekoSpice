//! Field validation helpers for simulation parameter inputs.
//!
//! Provides visual feedback (colored borders, tooltips) when values
//! are outside expected ranges or have syntax issues.

use eframe::egui;

/// Validation result for a single field value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum FieldValidity {
    /// Value is valid or empty (optional field).
    Ok,
    /// Value is empty but required for this analysis type.
    Required,
    /// Value cannot be parsed as a number.
    InvalidNumber,
    /// Value is outside a reasonable range.
    OutOfRange,
}

impl FieldValidity {
    /// Color indicator for this validation state.
    pub(crate) fn color(self, palette: &crate::app::theme::StudioPalette) -> egui::Color32 {
        match self {
            Self::Ok => palette.border,
            Self::Required => palette.warning,
            Self::InvalidNumber => palette.danger,
            Self::OutOfRange => palette.danger,
        }
    }

    /// Tooltip text for this validation state.
    pub(crate) fn tooltip(self) -> &'static str {
        match self {
            Self::Ok => "",
            Self::Required => "This field is required",
            Self::InvalidNumber => "Cannot parse as a SPICE value",
            Self::OutOfRange => "Value outside expected range",
        }
    }
}

/// Validate a SPICE numeric value string (e.g., "1k", "1e-3", "100u").
#[allow(dead_code)]
pub(crate) fn validate_spice_value(value: &str) -> FieldValidity {
    let v = value.trim();
    if v.is_empty() {
        return FieldValidity::Ok;
    }
    // Try direct parse first (pure numeric)
    if v.parse::<f64>().is_ok() {
        return FieldValidity::Ok;
    }
    // Strip standard SPICE suffixes: k/M/G/T/m/u/n/p/f/a
    let stripped = v.trim_end_matches(|c: char|
        matches!(c, 'k' | 'K' | 'M' | 'G' | 'T' | 'm' | 'u' | 'n' | 'p' | 'f' | 'a')
    );
    if stripped.is_empty() || stripped.parse::<f64>().is_ok() {
        return FieldValidity::Ok;
    }
    FieldValidity::InvalidNumber
}

/// Validate a temperature value (should be reasonable: >= -273.15).
pub(crate) fn validate_temperature(value: &str) -> FieldValidity {
    if value.trim().is_empty() {
        return FieldValidity::Ok;
    }
    match value.trim().parse::<f64>() {
        Ok(t) if t < -273.15 => FieldValidity::OutOfRange,
        Ok(_) => FieldValidity::Ok,
        Err(_) => FieldValidity::InvalidNumber,
    }
}

/// Wrap a TextEdit in a Frame with a colored border for validation feedback.
pub(crate) fn validated_frame(
    validity: FieldValidity,
    palette: &crate::app::theme::StudioPalette,
) -> egui::Frame {
    let color = validity.color(palette);
    let stroke = if validity == FieldValidity::Ok {
        egui::Stroke::NONE
    } else {
        egui::Stroke::new(1.5, color)
    };
    egui::Frame::new()
        .stroke(stroke)
        .corner_radius(3)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spice_value_valid() {
        assert_eq!(validate_spice_value("1k"), FieldValidity::Ok);
        assert_eq!(validate_spice_value("1e-3"), FieldValidity::Ok);
        assert_eq!(validate_spice_value("100u"), FieldValidity::Ok);
        assert_eq!(validate_spice_value("0"), FieldValidity::Ok);
        assert_eq!(validate_spice_value(""), FieldValidity::Ok);
    }

    #[test]
    fn spice_value_invalid() {
        assert_eq!(validate_spice_value("abc"), FieldValidity::InvalidNumber);
    }

    #[test]
    fn temperature_valid() {
        assert_eq!(validate_temperature("27"), FieldValidity::Ok);
        assert_eq!(validate_temperature("-40"), FieldValidity::Ok);
        assert_eq!(validate_temperature("125"), FieldValidity::Ok);
        assert_eq!(validate_temperature(""), FieldValidity::Ok);
    }

    #[test]
    fn temperature_below_absolute_zero() {
        assert_eq!(validate_temperature("-300"), FieldValidity::OutOfRange);
    }
}
