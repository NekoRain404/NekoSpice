use crate::edit::validate_at;
use crate::{KicadAt, KicadEditSummary, KicadSchematic, KicadTextItem};
use osl_core::{OslError, OslResult};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum KicadSimulationDirectiveKind {
    Tran,
    Ac,
    Dc,
    Op,
    Save,
    Include,
    Lib,
    Param,
    Model,
    Subckt,
    Control,
    Endc,
    Step,
    Noise,
    Measure,
    Disto,
    Sens,
    Other,
}

impl KicadSimulationDirectiveKind {
    /// keyword。
    pub fn keyword(self) -> Option<&'static str> {
        match self {
            Self::Tran => Some(".tran"),
            Self::Ac => Some(".ac"),
            Self::Dc => Some(".dc"),
            Self::Op => Some(".op"),
            Self::Save => Some(".save"),
            Self::Include => Some(".include"),
            Self::Lib => Some(".lib"),
            Self::Param => Some(".param"),
            Self::Model => Some(".model"),
            Self::Subckt => Some(".subckt"),
            Self::Control => Some(".control"),
            Self::Endc => Some(".endc"),
            Self::Step => Some(".step"),
            Self::Noise => Some(".noise"),
            Self::Measure => Some(".measure"),
            Self::Disto => Some(".disto"),
            Self::Sens => Some(".sens"),
            Self::Other => None,
        }
    }

    /// from directive text。
    pub fn from_directive_text(text: &str) -> Option<Self> {
        let keyword = directive_keyword(text)?;
        Some(match keyword.as_str() {
            ".tran" => Self::Tran,
            ".ac" => Self::Ac,
            ".dc" => Self::Dc,
            ".op" => Self::Op,
            ".save" => Self::Save,
            ".include" => Self::Include,
            ".inc" => Self::Include,
            ".lib" => Self::Lib,
            ".param" => Self::Param,
            ".model" => Self::Model,
            ".subckt" => Self::Subckt,
            ".control" => Self::Control,
            ".endc" => Self::Endc,
            ".step" => Self::Step,
            ".noise" => Self::Noise,
            ".measure" | ".meas" => Self::Measure,
            ".disto" => Self::Disto,
            ".sens" => Self::Sens,
            _ => Self::Other,
        })
    }

    /// is analysis。
    pub fn is_analysis(self) -> bool {
        matches!(self, Self::Tran | Self::Ac | Self::Dc | Self::Op | Self::Noise | Self::Disto | Self::Sens)
    }
}

impl fmt::Display for KicadSimulationDirectiveKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Tran => "tran",
            Self::Ac => "ac",
            Self::Dc => "dc",
            Self::Op => "op",
            Self::Save => "save",
            Self::Include => "include",
            Self::Lib => "lib",
            Self::Param => "param",
            Self::Model => "model",
            Self::Subckt => "subckt",
            Self::Control => "control",
            Self::Endc => "endc",
            Self::Step => "step",
            Self::Noise => "noise",
            Self::Measure => "measure",
            Self::Disto => "disto",
            Self::Sens => "sens",
            Self::Other => "other",
        })
    }
}

impl FromStr for KicadSimulationDirectiveKind {
    type Err = OslError;

    fn from_str(value: &str) -> OslResult<Self> {
        let normalized = value
            .trim()
            .trim_start_matches('.')
            .replace(['-', '_'], "")
            .to_ascii_lowercase();
        match normalized.as_str() {
            "tran" | "transient" => Ok(Self::Tran),
            "ac" => Ok(Self::Ac),
            "dc" => Ok(Self::Dc),
            "op" | "operatingpoint" => Ok(Self::Op),
            "save" => Ok(Self::Save),
            "include" | "inc" => Ok(Self::Include),
            "lib" => Ok(Self::Lib),
            "param" => Ok(Self::Param),
            "model" => Ok(Self::Model),
            "subckt" => Ok(Self::Subckt),
            "control" => Ok(Self::Control),
            "endc" => Ok(Self::Endc),
            "step" => Ok(Self::Step),
            "noise" => Ok(Self::Noise),
            "measure" | "meas" => Ok(Self::Measure),
            "disto" => Ok(Self::Disto),
            "sens" => Ok(Self::Sens),
            "other" | "raw" => Ok(Self::Other),
            _ => Err(OslError::InvalidInput(format!(
                "unsupported KiCad simulation directive kind '{value}'"
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadSimulationDirective {
    pub text: String,
    pub kind: KicadSimulationDirectiveKind,
    pub at: Option<KicadAt>,
    pub uuid: Option<String>,
}

impl KicadSimulationDirective {
    /// from text item。
    pub fn from_text_item(item: &KicadTextItem) -> Option<Self> {
        let text = item.text.trim();
        if !is_spice_directive_text(text) {
            return None;
        }

        Some(Self {
            text: text.to_string(),
            kind: KicadSimulationDirectiveKind::from_directive_text(text)
                .unwrap_or(KicadSimulationDirectiveKind::Other),
            at: item.at,
            uuid: item.uuid.clone(),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadSimulationDirectiveUpdate {
    pub kind: KicadSimulationDirectiveKind,
    pub body: String,
    pub at: Option<KicadAt>,
    pub uuid: Option<String>,
}

impl KicadSimulationDirectiveUpdate {
    /// normalized text。
    pub fn normalized_text(&self) -> OslResult<String> {
        normalize_simulation_directive_text(self.kind, &self.body)
    }
}

/// simulation directives from text items。
pub(crate) fn simulation_directives_from_text_items(
    text_items: &[KicadTextItem],
) -> Vec<KicadSimulationDirective> {
    text_items
        .iter()
        .filter_map(KicadSimulationDirective::from_text_item)
        .collect()
}

impl KicadSchematic {
    /// simulation directives。
    pub fn simulation_directives(&self) -> Vec<KicadSimulationDirective> {
        simulation_directives_from_text_items(&self.text_items)
    }

    /// spice directives。
    pub fn spice_directives(&self) -> Vec<&KicadTextItem> {
        self.text_items
            .iter()
            .filter(|item| is_spice_directive_text(&item.text))
            .collect()
    }

    /// set simulation directive。
    pub fn set_simulation_directive(
        &mut self,
        update: KicadSimulationDirectiveUpdate,
    ) -> OslResult<KicadEditSummary> {
        let text = update.normalized_text()?;
        if let Some(at) = update.at {
            validate_at(at, "simulation directive")?;
        }

        if let Some(index) = find_simulation_directive_index(&self.text_items, update.kind) {
            let current_uuid = self.text_items[index].uuid.as_deref();
            let requested_uuid = update
                .uuid
                .as_deref()
                .filter(|uuid| !uuid.trim().is_empty());
            if current_uuid.is_none()
                && let Some(uuid) = requested_uuid
                && self.used_uuids().contains(uuid)
            {
                return Err(OslError::InvalidInput(format!(
                    "KiCad UUID '{uuid}' is already used in this schematic"
                )));
            }

            let directive = &mut self.text_items[index];
            directive.text = text.clone();
            if let Some(at) = update.at {
                directive.at = Some(at);
            }
            if directive.uuid.is_none()
                && let Some(uuid) = requested_uuid
            {
                directive.uuid = Some(uuid.to_string());
            }
            return Ok(KicadEditSummary {
                operation: "set-simulation-directive".to_string(),
                target: text,
            });
        }

        let at = update
            .at
            .unwrap_or_else(|| default_simulation_directive_at(self));
        validate_at(at, "simulation directive")?;
        let payload = format!("{}@{},{},{}", text, at.x, at.y, at.rotation);
        let uuid = Some(self.edit_uuid(update.uuid, "simulation-directive", &payload)?);
        self.text_items.push(KicadTextItem {
            text: text.clone(),
            at: Some(at),
            uuid,
            effects: None,
        });

        Ok(KicadEditSummary {
            operation: "set-simulation-directive".to_string(),
            target: text,
        })
    }
}

/// find simulation directive index。
pub(crate) fn find_simulation_directive_index(
    text_items: &[KicadTextItem],
    kind: KicadSimulationDirectiveKind,
) -> Option<usize> {
    if kind == KicadSimulationDirectiveKind::Other {
        return None;
    }
    text_items.iter().position(|item| {
        KicadSimulationDirectiveKind::from_directive_text(&item.text) == Some(kind)
    })
}

/// normalize simulation directive text。
pub(crate) fn normalize_simulation_directive_text(
    kind: KicadSimulationDirectiveKind,
    body: &str,
) -> OslResult<String> {
    let body = body.trim();
    if body.is_empty() {
        return match kind {
            KicadSimulationDirectiveKind::Op => Ok(".op".to_string()),
            _ => Err(OslError::InvalidInput(format!(
                "KiCad simulation directive '{kind}' body must not be empty"
            ))),
        };
    }

    if body.starts_with('.') {
        let parsed_kind = KicadSimulationDirectiveKind::from_directive_text(body)
            .unwrap_or(KicadSimulationDirectiveKind::Other);
        if kind != KicadSimulationDirectiveKind::Other && parsed_kind != kind {
            return Err(OslError::InvalidInput(format!(
                "KiCad simulation directive text '{}' does not match requested kind '{}'",
                body, kind
            )));
        }
        return Ok(body.to_string());
    }

    let Some(keyword) = kind.keyword() else {
        return Err(OslError::InvalidInput(
            "KiCad raw simulation directives must start with '.'".to_string(),
        ));
    };
    Ok(format!("{keyword} {body}"))
}

/// is spice analysis directive text。
pub(crate) fn is_spice_analysis_directive_text(text: &str) -> bool {
    KicadSimulationDirectiveKind::from_directive_text(text).is_some_and(|kind| kind.is_analysis())
}

/// is spice directive text。
pub(crate) fn is_spice_directive_text(text: &str) -> bool {
    text.trim_start().starts_with('.')
}

fn directive_keyword(text: &str) -> Option<String> {
    let text = text.trim_start();
    let keyword = text
        .split_whitespace()
        .next()
        .filter(|keyword| keyword.starts_with('.'))?;
    Some(keyword.to_ascii_lowercase())
}

fn default_simulation_directive_at(schematic: &KicadSchematic) -> KicadAt {
    let mut y = 20.32;
    for directive in schematic.simulation_directives() {
        if let Some(at) = directive.at {
            y = f64::max(y, at.y + 5.08);
        }
    }
    KicadAt {
        x: 20.32,
        y,
        rotation: 0.0,
    }
}
