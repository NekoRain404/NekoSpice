//! Schematic check diagnostics — error/warning/info reports for ERC.

use crate::json::json_option;
use nsp_core::json_escape;

#[derive(Debug, Clone, PartialEq)]
pub struct NspSchematicCheckReport {
    pub source: String,
    pub symbol_count: usize,
    pub sheet_count: usize,
    pub net_count: usize,
    pub spice_directive_count: usize,
    pub diagnostics: Vec<NspSchematicDiagnostic>,
}

impl NspSchematicCheckReport {
    /// error count。
    pub fn error_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == NspDiagnosticSeverity::Error)
            .count()
    }

    /// warning count。
    pub fn warning_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == NspDiagnosticSeverity::Warning)
            .count()
    }

    /// info count。
    pub fn info_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == NspDiagnosticSeverity::Info)
            .count()
    }

    /// to json。
    pub fn to_json(&self) -> String {
        let diagnostics = self
            .diagnostics
            .iter()
            .map(|diagnostic| {
                format!(
                    concat!(
                        "    {{ \"severity\": \"{}\", \"code\": \"{}\", ",
                        "\"message\": \"{}\", \"item\": {}, \"net\": {}, \"pin\": {} }}"
                    ),
                    diagnostic.severity.as_str(),
                    json_escape(&diagnostic.code),
                    json_escape(&diagnostic.message),
                    json_option(diagnostic.item.as_deref()),
                    json_option(diagnostic.net.as_deref()),
                    json_option(diagnostic.pin.as_deref())
                )
            })
            .collect::<Vec<_>>()
            .join(",\n");

        format!(
            concat!(
                "{{\n",
                "  \"source\": \"{}\",\n",
                "  \"symbol_count\": {},\n",
                "  \"sheet_count\": {},\n",
                "  \"net_count\": {},\n",
                "  \"spice_directive_count\": {},\n",
                "  \"diagnostic_count\": {},\n",
                "  \"error_count\": {},\n",
                "  \"warning_count\": {},\n",
                "  \"info_count\": {},\n",
                "  \"diagnostics\": [\n",
                "{}\n",
                "  ]\n",
                "}}"
            ),
            json_escape(&self.source),
            self.symbol_count,
            self.sheet_count,
            self.net_count,
            self.spice_directive_count,
            self.diagnostics.len(),
            self.error_count(),
            self.warning_count(),
            self.info_count(),
            diagnostics
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NspHierarchyNetlist {
    pub netlist: String,
    pub diagnostics: Vec<NspSchematicDiagnostic>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NspSchematicDiagnostic {
    pub severity: NspDiagnosticSeverity,
    pub code: String,
    pub message: String,
    pub item: Option<String>,
    pub net: Option<String>,
    pub pin: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NspDiagnosticSeverity {
    Info,
    Warning,
    Error,
}

impl NspDiagnosticSeverity {
    /// as str。
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }
}

/// schema schematic diagnostic。
pub(crate) fn schema_diagnostic(
    severity: NspDiagnosticSeverity,
    code: &str,
    message: &str,
    item: Option<String>,
    net: Option<String>,
    pin: Option<String>,
) -> NspSchematicDiagnostic {
    NspSchematicDiagnostic {
        severity,
        code: code.to_string(),
        message: message.to_string(),
        item,
        net,
        pin,
    }
}
