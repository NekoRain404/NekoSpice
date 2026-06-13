//! schema project file (.nsp_pro) parsing.

use crate::json::{json_option, json_u64_option};
use nsp_core::{OslError, OslResult, json_escape};
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub struct NspProject {
    pub source: String,
    pub meta_filename: Option<String>,
    pub meta_version: Option<u64>,
    pub project_name: Option<String>,
    pub schematic_page_layout_descr_file: Option<String>,
    pub sheets: Vec<NspProjectSheet>,
    pub text_variable_count: usize,
}

impl NspProject {
    /// schematic stem candidates。
    pub fn schematic_stem_candidates(&self) -> Vec<String> {
        let mut candidates = Vec::new();
        push_unique_nonempty(&mut candidates, self.project_name.as_deref());
        push_unique_nonempty(
            &mut candidates,
            self.meta_filename
                .as_deref()
                .and_then(path_stem_from_string)
                .as_deref(),
        );
        push_unique_nonempty(
            &mut candidates,
            path_stem_from_string(&self.source).as_deref(),
        );
        candidates
    }

    /// to summary json。
    pub fn to_summary_json(&self) -> String {
        let sheet_names = self
            .sheets
            .iter()
            .map(|sheet| format!("\"{}\"", json_escape(&sheet.name)))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            concat!(
                "{{\n",
                "  \"source\": \"{}\",\n",
                "  \"meta_filename\": {},\n",
                "  \"meta_version\": {},\n",
                "  \"project_name\": {},\n",
                "  \"schematic_page_layout_descr_file\": {},\n",
                "  \"sheet_count\": {},\n",
                "  \"sheet_names\": [{}],\n",
                "  \"text_variable_count\": {}\n",
                "}}"
            ),
            json_escape(&self.source),
            json_option(self.meta_filename.as_deref()),
            json_u64_option(self.meta_version),
            json_option(self.project_name.as_deref()),
            json_option(self.schematic_page_layout_descr_file.as_deref()),
            self.sheets.len(),
            sheet_names,
            self.text_variable_count,
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NspProjectSheet {
    pub uuid: String,
    pub name: String,
}

/// parse schema project。
pub fn parse_project(input: &str, source: &str) -> OslResult<NspProject> {
    let root: serde_json::Value = serde_json::from_str(input).map_err(|error| {
        OslError::InvalidInput(format!(
            "failed to parse schema project JSON {source}: {error}"
        ))
    })?;
    if !root.is_object() {
        return Err(OslError::InvalidInput(format!(
            "expected schema project JSON object in {source}"
        )));
    }

    Ok(NspProject {
        source: source.to_string(),
        meta_filename: json_path_string(&root, &["meta", "filename"]),
        meta_version: json_path_u64(&root, &["meta", "version"]),
        project_name: json_path_string(&root, &["project", "name"]),
        schematic_page_layout_descr_file: json_path_string(
            &root,
            &["schematic", "page_layout_descr_file"],
        ),
        sheets: parse_project_sheets(&root),
        text_variable_count: root
            .get("text_variables")
            .and_then(|value| value.as_object())
            .map(|variables| variables.len())
            .unwrap_or(0),
    })
}

fn json_path_string(root: &serde_json::Value, path: &[&str]) -> Option<String> {
    let mut current = root;
    for key in path {
        current = current.get(*key)?;
    }
    current
        .as_str()
        .map(str::to_string)
        .filter(|value| !value.is_empty())
}

fn json_path_u64(root: &serde_json::Value, path: &[&str]) -> Option<u64> {
    let mut current = root;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_u64()
}

fn parse_project_sheets(root: &serde_json::Value) -> Vec<NspProjectSheet> {
    root.get("sheets")
        .and_then(|value| value.as_array())
        .map(|sheets| {
            sheets
                .iter()
                .filter_map(|sheet| {
                    let values = sheet.as_array()?;
                    let uuid = values.first()?.as_str()?;
                    let name = values.get(1)?.as_str()?;
                    Some(NspProjectSheet {
                        uuid: uuid.to_string(),
                        name: name.to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn path_stem_from_string(path: &str) -> Option<String> {
    Path::new(path)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(str::to_string)
        .filter(|stem| !stem.is_empty())
}

fn push_unique_nonempty(values: &mut Vec<String>, value: Option<&str>) {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return;
    };
    if !values.iter().any(|existing| existing == value) {
        values.push(value.to_string());
    }
}
