use std::error::Error;
use std::fmt::{self, Display};
use std::fs;
use std::io;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub type OslResult<T> = Result<T, OslError>;

#[derive(Debug)]
pub enum OslError {
    Io { action: String, source: io::Error },
    InvalidInput(String),
    Process(String),
}

impl OslError {
    pub fn io(action: impl Into<String>, source: io::Error) -> Self {
        Self::Io {
            action: action.into(),
            source,
        }
    }
}

impl Display for OslError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OslError::Io { action, source } => write!(f, "{action}: {source}"),
            OslError::InvalidInput(message) => write!(f, "{message}"),
            OslError::Process(message) => write!(f, "{message}"),
        }
    }
}

impl Error for OslError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            OslError::Io { source, .. } => Some(source),
            OslError::InvalidInput(_) | OslError::Process(_) => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunStatus {
    Passed,
    Failed,
}

impl RunStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            RunStatus::Passed => "passed",
            RunStatus::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Artifact {
    pub path: String,
    pub kind: String,
}

#[derive(Debug, Clone)]
pub struct RunMetadata {
    pub schema_version: u32,
    pub run_id: String,
    pub backend: String,
    pub backend_executable: String,
    pub source_netlist: String,
    pub working_netlist: String,
    pub output_dir: String,
    pub status: RunStatus,
    pub exit_code: Option<i32>,
    pub duration_ms: u128,
    pub started_unix_ms: u128,
    pub artifacts: Vec<Artifact>,
}

impl RunMetadata {
    pub fn to_json(&self) -> String {
        let artifacts = self
            .artifacts
            .iter()
            .map(|artifact| {
                format!(
                    "    {{ \"path\": \"{}\", \"kind\": \"{}\" }}",
                    json_escape(&artifact.path),
                    json_escape(&artifact.kind)
                )
            })
            .collect::<Vec<_>>()
            .join(",\n");

        format!(
            concat!(
                "{{\n",
                "  \"schema_version\": {},\n",
                "  \"run_id\": \"{}\",\n",
                "  \"backend\": \"{}\",\n",
                "  \"backend_executable\": \"{}\",\n",
                "  \"source_netlist\": \"{}\",\n",
                "  \"working_netlist\": \"{}\",\n",
                "  \"output_dir\": \"{}\",\n",
                "  \"status\": \"{}\",\n",
                "  \"exit_code\": {},\n",
                "  \"duration_ms\": {},\n",
                "  \"started_unix_ms\": {},\n",
                "  \"artifacts\": [\n",
                "{}\n",
                "  ]\n",
                "}}\n"
            ),
            self.schema_version,
            json_escape(&self.run_id),
            json_escape(&self.backend),
            json_escape(&self.backend_executable),
            json_escape(&self.source_netlist),
            json_escape(&self.working_netlist),
            json_escape(&self.output_dir),
            self.status.as_str(),
            option_i32_json(self.exit_code),
            self.duration_ms,
            self.started_unix_ms,
            artifacts
        )
    }
}

pub fn now_unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

pub fn make_run_id(prefix: &str) -> String {
    format!("{prefix}-{}", now_unix_ms())
}

pub fn write_text(path: &Path, content: &str) -> OslResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| OslError::io(format!("create {}", parent.display()), err))?;
    }
    fs::write(path, content).map_err(|err| OslError::io(format!("write {}", path.display()), err))
}

pub fn read_text(path: &Path) -> OslResult<String> {
    fs::read_to_string(path).map_err(|err| OslError::io(format!("read {}", path.display()), err))
}

pub fn json_escape(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());
    for character in input.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            '\u{08}' => escaped.push_str("\\b"),
            '\u{0c}' => escaped.push_str("\\f"),
            character if character.is_control() => {
                escaped.push_str(&format!("\\u{:04x}", character as u32));
            }
            character => escaped.push(character),
        }
    }
    escaped
}

pub fn html_escape(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());
    for character in input.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            character => escaped.push(character),
        }
    }
    escaped
}

fn option_i32_json(value: Option<i32>) -> String {
    match value {
        Some(value) => value.to_string(),
        None => "null".to_string(),
    }
}
