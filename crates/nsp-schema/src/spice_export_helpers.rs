//! SPICE export helper functions.
//!
//! Pure utility functions used by [`super::spice_export`] for symbol name
//! resolution, parameter conversion, value parsing, and path expansion.
//! Extracted to keep the main export module focused on the core logic.

use std::collections::BTreeMap;

use crate::NspSymbolInstance;
use crate::connectivity::normalize_net_name;

// ── Identifier Utilities ──────────────────────────────────────────────

/// Sanitize a string for use as a SPICE identifier.
///
/// Replaces non-alphanumeric characters with underscores and collapses
/// consecutive underscores.  Empty input yields `"item"`.
pub(crate) fn sanitize_spice_identifier(value: &str) -> String {
    let sanitized: String = value
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let sanitized = sanitized
        .split('_')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("_")
        .trim_matches('_')
        .to_string();
    if sanitized.is_empty() {
        "item".to_string()
    } else {
        sanitized
    }
}

/// Infer SPICE primitive prefix from the symbol's lib_id string.
///
/// When `Sim.Device` / `Spice_Primitive` are not explicitly set, the reference
/// designator prefix is used.  However KiCad uses `Q` for all transistors
/// including MOSFETs, while SPICE requires `M`.  This helper inspects the
/// `lib_id` (e.g. `Device:Q_NMOS_DGS`) to determine the correct SPICE prefix.
pub(crate) fn infer_spice_primitive_from_lib_id(lib_id: &str) -> Option<&'static str> {
    let lower = lib_id.to_ascii_lowercase();
    if lower.contains("nmos") || lower.contains("pmos") || lower.contains("mosfet") {
        Some("M")
    } else if lower.contains("njfet") || lower.contains("pjfet") || lower.contains("jfet") {
        Some("J")
    } else if lower.contains("npn") || lower.contains("pnp") {
        Some("Q")
    } else if lower.contains("diode") || lower.contains("_d_") || lower.ends_with(":d") {
        Some("D")
    } else {
        None
    }
}

/// Map a device type string to its SPICE primitive prefix.
///
/// Recognizes common aliases (e.g. "NMOS" -> "M", "DIODE" -> "D").
/// Single-character inputs are returned as-is.
pub(crate) fn spice_primitive_for_device(device: &str) -> Option<String> {
    let device = device.to_ascii_uppercase();
    let primitive = match device.as_str() {
        "R" | "RES" | "RESISTOR" => "R",
        "C" | "CAP" | "CAPACITOR" => "C",
        "L" | "IND" | "INDUCTOR" => "L",
        "V" | "VSOURCE" | "VOLTAGE" => "V",
        "I" | "ISOURCE" | "CURRENT" => "I",
        "D" | "DIODE" => "D",
        "NPN" | "PNP" | "BJT" => "Q",
        "NJFET" | "PJFET" | "JFET" => "J",
        "NMOS" | "PMOS" | "NMES" | "PMES" | "MOSFET" => "M",
        "SW" | "SWITCH" => "S",
        "CSW" | "CURRENT_SWITCH" => "W",
        "VCVS" => "E",
        "CCCS" => "F",
        "VCCS" => "G",
        "CCVS" => "H",
        "TLINE" | "TRANSMISSION_LINE" => "T",
        "K" | "COUPLED_INDUCTOR" => "K",
        "SUBCKT" => "X",
        "SPICE" => "SPICE",
        "" => return None,
        other if other.len() == 1 => other,
        _ => return None,
    };
    Some(primitive.to_string())
}

// ── SPICE Parameter Conversion ────────────────────────────────────────

/// Convert named SPICE parameters to positional SPICE format.
///
/// Some EDA tools use named parameters like `dc=2 ampl=1 f=1k td=0 theta=0 phase=0`
/// which ngspice does not accept directly.
///
/// The `Sim.Type` property determines which SPICE source type to use.
pub(crate) fn convert_spice_params(sim_type: Option<&str>, params: &str) -> String {
    let Some(sim_type) = sim_type else {
        return params.to_string();
    };
    let tokens: Vec<&str> = params
        .split(|c: char| c.is_ascii_whitespace())
        .filter(|t| !t.is_empty())
        .collect();
    let mut named = BTreeMap::new();
    for token in &tokens {
        if let Some((key, val)) = token.split_once('=') {
            named.insert(key.trim().to_lowercase(), val.trim().to_string());
        }
    }
    match sim_type.to_ascii_uppercase().as_str() {
        "PULSE" => {
            let v1 = named
                .get("v1")
                .or(named.get("dc"))
                .cloned()
                .unwrap_or_default();
            let v2 = named
                .get("v2")
                .or(named.get("ampl"))
                .cloned()
                .unwrap_or_default();
            let td = named.get("td").cloned().unwrap_or_else(|| "0".into());
            let tr = named.get("tr").cloned().unwrap_or_else(|| "1n".into());
            let tf = named.get("tf").cloned().unwrap_or_else(|| "1n".into());
            let pw = named.get("pw").cloned().unwrap_or_else(|| "0.5m".into());
            let per = named
                .get("per")
                .or(named.get("freq"))
                .cloned()
                .unwrap_or_else(|| "1m".into());
            format!("pulse({v1} {v2} {td} {tr} {tf} {pw} {per})")
        }
        "SIN" => {
            let voffs = named
                .get("voffs")
                .or(named.get("dc"))
                .cloned()
                .unwrap_or_default();
            let vamp = named
                .get("vamp")
                .or(named.get("ampl"))
                .cloned()
                .unwrap_or_default();
            let freq = named.get("freq").cloned().unwrap_or_default();
            let td = named.get("td").cloned().unwrap_or_else(|| "0".into());
            let theta = named.get("theta").cloned().unwrap_or_else(|| "0".into());
            let phase = named.get("phase").cloned().unwrap_or_else(|| "0".into());
            format!("sin({voffs} {vamp} {freq} {td} {theta} {phase})")
        }
        "DC" => named
            .get("v1")
            .or(named.get("dc"))
            .cloned()
            .unwrap_or_else(|| params.to_string()),
        _ => params.to_string(),
    }
}

/// Extract a named parameter as f64, using a default if not found.
pub(crate) fn extract_named_f64(params: &str, name: &str, default: f64) -> f64 {
    params
        .split(|c: char| c.is_ascii_whitespace())
        .find_map(|token| {
            let (key, val) = token.split_once('=')?;
            if key.trim().eq_ignore_ascii_case(name) {
                parse_spice_value(val.trim())
            } else {
                None
            }
        })
        .unwrap_or(default)
}

/// Extract a named frequency parameter, supporting SI suffixes (k, M, G, u, n, p).
pub(crate) fn extract_named_freq(params: &str, name: &str, default: f64) -> f64 {
    params
        .split(|c: char| c.is_ascii_whitespace())
        .find_map(|token| {
            let (key, val) = token.split_once('=')?;
            if key.trim().eq_ignore_ascii_case(name) {
                parse_spice_value(val.trim())
            } else {
                None
            }
        })
        .unwrap_or(default)
}

/// Parse a SPICE value with SI suffixes (1k = 1000, 1u = 1e-6, etc.)
pub(crate) fn parse_spice_value(value: &str) -> Option<f64> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    let (num_str, suffix) = if let Some(pos) = value.find(|c: char| c.is_ascii_alphabetic()) {
        (&value[..pos], &value[pos..])
    } else {
        (value, "")
    };
    let num: f64 = num_str.parse().ok()?;
    let multiplier = match suffix.to_ascii_lowercase().as_str() {
        "t" | "ter" => 1e12,
        "g" | "gig" => 1e9,
        "meg" => 1e6,
        "m" | "mil" => 1e-3,
        "k" | "kil" => 1e3,
        "u" | "mic" => 1e-6,
        "n" | "nan" => 1e-9,
        "p" | "pic" => 1e-12,
        "f" | "fem" => 1e-15,
        "" => 1.0,
        _ => 1.0,
    };
    Some(num * multiplier)
}

/// Convert Spice_Model source value templates to proper SPICE values.
///
/// Some tools use `dc(1)`, `pulse(...)`, `sin(...)` etc. in the `Spice_Model`
/// property. For DC sources, `dc(N)` should become just `N`. For other
/// source types, the function call format is kept as-is.
pub(crate) fn normalize_source_value(model: &str) -> String {
    let trimmed = model.trim();
    if let Some(inner) = trimmed
        .strip_prefix("dc(")
        .and_then(|s| s.strip_suffix(')'))
    {
        inner.trim().to_string()
    } else {
        trimmed.to_string()
    }
}

/// Expand `${VAR}` style environment variables in a path string.
pub(crate) fn expand_path_env_vars(path: &str) -> String {
    let mut result = path.to_string();
    while let Some(start) = result.find("${") {
        if let Some(end) = result[start + 2..].find('}') {
            let var_name = &result[start + 2..start + 2 + end];
            if let Ok(val) = std::env::var(var_name) {
                result = format!("{}{}{}", &result[..start], val, &result[start + 3 + end..]);
            } else {
                break;
            }
        } else {
            break;
        }
    }
    result
}

// ── SPICE Name Construction ───────────────────────────────────────────

/// Compose the effective model value from optional model name and parameters.
pub(crate) fn compose_spice_model_value(
    model: Option<&str>,
    params: Option<&str>,
    fallback: Option<&str>,
) -> String {
    match (
        model.filter(|value| !value.is_empty()),
        params.filter(|value| !value.is_empty()),
    ) {
        (Some(model), Some(params)) => format!("{model} {params}"),
        (Some(model), None) => model.to_string(),
        (None, Some(params)) => params.to_string(),
        (None, None) => fallback.unwrap_or_default().to_string(),
    }
}

/// Build the SPICE instance name from the schematic reference and target prefix.
///
/// When the reference already carries a different SPICE prefix (e.g. `Q3` for
/// a MOSFET that should be `M`), the prefix is **replaced** — yielding `M3`
/// rather than `MQ3`.  This handles KiCad's convention of using `Q` for all
/// transistors regardless of type.
pub(crate) fn spice_item_name(reference: &str, primitive: &str) -> String {
    let Some(first) = primitive.chars().next() else {
        return reference.to_string();
    };
    if reference
        .chars()
        .next()
        .is_some_and(|character| character.eq_ignore_ascii_case(&first))
    {
        reference.to_string()
    } else {
        let known_prefixes = [
            'R', 'C', 'L', 'V', 'I', 'D', 'Q', 'J', 'M', 'S', 'W', 'E', 'F', 'G', 'H', 'T', 'K',
        ];
        if let Some(ref_first) = reference.chars().next()
            && known_prefixes
                .iter()
                .any(|p| p.eq_ignore_ascii_case(&ref_first))
        {
            format!("{}{}", first, &reference[ref_first.len_utf8()..])
        } else {
            format!("{first}{reference}")
        }
    }
}

/// Expand `${REFERENCE}` and `${N1}`..`${Nn}` placeholders in a SPICE template.
pub(crate) fn expand_spice_template(template: &str, reference: &str, nodes: &[String]) -> String {
    let mut expanded = template.replace("${REFERENCE}", reference);
    for (index, node) in nodes.iter().enumerate() {
        expanded = expanded.replace(&format!("${{N{}}}", index + 1), node);
    }
    expanded
}

/// Wrap a path in quotes if it contains spaces or quotes.
pub(crate) fn quote_spice_path(path: &str) -> String {
    if path
        .bytes()
        .any(|byte| byte.is_ascii_whitespace() || byte == b'"')
    {
        format!("\"{}\"", path.replace('"', "\\\""))
    } else {
        format!("\"{}\"", path)
    }
}

// ── Hierarchy Helpers ─────────────────────────────────────────────────

/// Count lines that look like SPICE directives (start with `.`).
pub(crate) fn count_spice_directive_lines(netlist: &str) -> usize {
    netlist
        .lines()
        .filter(|line| {
            let line = line.trim_start();
            line.starts_with('.') && !line.eq_ignore_ascii_case(".end")
        })
        .count()
}

/// Check if a diagnostic is non-fatal for the hierarchy root sheet.
pub(crate) fn is_hierarchy_root_nonfatal_diagnostic(
    diagnostic: &crate::NspSchematicDiagnostic,
    has_spice_directive: bool,
    has_analysis_directive: bool,
) -> bool {
    matches!(
        diagnostic.code.as_str(),
        "hierarchical-sheet-unsupported" | "simulation-disabled-sheet"
    ) || (diagnostic.code == "missing-spice-directive" && has_spice_directive)
        || (diagnostic.code == "missing-analysis-directive" && has_analysis_directive)
}

/// Check if a diagnostic belongs to a child sheet (non-fatal).
pub(crate) fn is_child_sheet_nonfatal_diagnostic(
    diagnostic: &crate::NspSchematicDiagnostic,
) -> bool {
    matches!(
        diagnostic.code.as_str(),
        "hierarchical-sheet-unsupported"
            | "simulation-disabled-sheet"
            | "missing-spice-directive"
            | "missing-analysis-directive"
            | "missing-ground"
    )
}

/// Scope a net name for hierarchical sheets.
pub(crate) fn scoped_net_name(
    scope: &str,
    net: &str,
    aliases: &BTreeMap<String, String>,
) -> String {
    if net == "0" || net.eq_ignore_ascii_case("gnd") {
        return "0".to_string();
    }
    if let Some(alias) = aliases.get(&normalize_net_name(net)) {
        return alias.clone();
    }
    if scope == "root" || net == "unconnected" {
        return net.to_string();
    }
    format!(
        "{}_{}",
        sanitize_spice_identifier(scope),
        sanitize_spice_identifier(net)
    )
}

/// Determine the scope prefix for a child hierarchical sheet.
pub(crate) fn child_sheet_scope(parent_scope: &str, sheet: &crate::NspSheet) -> String {
    let sheet_name = sheet
        .sheet_name()
        .or_else(|| sheet.sheet_file())
        .unwrap_or("sheet");
    let sanitized = sanitize_spice_identifier(sheet_name);
    if parent_scope.is_empty() || parent_scope == "root" {
        sanitized
    } else {
        format!("{}_{}", parent_scope, sanitized)
    }
}

/// Create a scoped copy of a symbol instance for hierarchical export.
///
/// Skips root scope, power symbols (#*), and empty references.
pub(crate) fn scoped_symbol_instance(symbol: &NspSymbolInstance, scope: &str) -> NspSymbolInstance {
    if scope == "root" {
        return symbol.clone();
    }
    let mut symbol = symbol.clone();
    if let Some(reference) = symbol.reference().map(str::to_string)
        && !reference.trim().is_empty()
        && !reference.starts_with('#')
    {
        let scoped_ref = scoped_reference(&reference, scope);
        if let Some(property) = symbol
            .properties
            .iter_mut()
            .find(|property| property.name == "Reference")
        {
            property.value = scoped_ref;
        }
    }
    symbol
}

/// Scope a reference designator by inserting scope after the prefix character.
///
/// For example, `Rgain` with scope `stage1` becomes `Rstage1_gain`.
pub(crate) fn scoped_reference(reference: &str, scope: &str) -> String {
    let mut chars = reference.chars();
    let Some(prefix) = chars.next() else {
        return reference.to_string();
    };
    let suffix = chars.collect::<String>();
    format!(
        "{}{}_{}",
        prefix,
        sanitize_spice_identifier(scope),
        sanitize_spice_identifier(&suffix)
    )
}
