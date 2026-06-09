use crate::{ImportDiagnostic, ImportSeverity};
use osl_core::{OslError, OslResult, read_text};
use osl_kicad::{
    KicadDiagnosticSeverity, KicadProject, KicadSchematicDiagnostic, read_kicad_project,
};
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) fn resolve_import_source_path(path: &Path) -> OslResult<PathBuf> {
    if path.is_dir() {
        let hints = kicad_project_source_hints_for_dir(path);
        return discover_kicad_project_source(path, &hints);
    }
    if is_kicad_project_file(path) {
        let hints = kicad_project_source_hints_for_file(path);
        return discover_kicad_project_source(
            path.parent().unwrap_or_else(|| Path::new(".")),
            &hints,
        );
    }
    Ok(path.to_path_buf())
}

pub(crate) fn is_kicad_schematic(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("kicad_sch"))
}

pub(crate) fn kicad_schematic_diagnostic_to_import(
    diagnostic: &KicadSchematicDiagnostic,
) -> ImportDiagnostic {
    let detail = [
        diagnostic.item.as_ref().map(|item| format!("item={item}")),
        diagnostic.net.as_ref().map(|net| format!("net={net}")),
        diagnostic.pin.as_ref().map(|pin| format!("pin={pin}")),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(", ");
    let message = if detail.is_empty() {
        diagnostic.message.clone()
    } else {
        format!("{} ({detail})", diagnostic.message)
    };
    let suggestion = if diagnostic.code == "hierarchical-sheet-unsupported" {
        "Flatten or expand the KiCad hierarchy before import, or import a leaf schematic until hierarchical sheet expansion is implemented.".to_string()
    } else {
        "Run `osl kicad-check <file.kicad_sch>` and fix the schematic before relying on the generated SPICE netlist.".to_string()
    };

    ImportDiagnostic {
        line: 0,
        severity: match diagnostic.severity {
            KicadDiagnosticSeverity::Error => ImportSeverity::Error,
            KicadDiagnosticSeverity::Warning => ImportSeverity::Warning,
            KicadDiagnosticSeverity::Info => ImportSeverity::Info,
        },
        code: format!("kicad-{}", diagnostic.code),
        message,
        suggestion,
    }
}

fn is_kicad_project_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("kicad_pro"))
}

fn discover_kicad_project_source(
    project_dir: &Path,
    hints: &KicadProjectSourceHints,
) -> OslResult<PathBuf> {
    discover_kicad_project_schematic(project_dir, hints)
        .or_else(|_| discover_kicad_project_netlist(project_dir))
}

fn discover_kicad_project_schematic(
    project_dir: &Path,
    hints: &KicadProjectSourceHints,
) -> OslResult<PathBuf> {
    let mut candidates = Vec::new();
    collect_kicad_schematic_candidates(project_dir, project_dir, &mut candidates)?;
    candidates.sort_by(|left, right| {
        kicad_schematic_candidate_score(project_dir, hints, right)
            .cmp(&kicad_schematic_candidate_score(project_dir, hints, left))
            .then_with(|| left.display().to_string().cmp(&right.display().to_string()))
    });
    candidates.into_iter().next().ok_or_else(|| {
        OslError::InvalidInput(format!(
            "{} does not contain an importable KiCad schematic (.kicad_sch)",
            project_dir.display()
        ))
    })
}

fn discover_kicad_project_netlist(project_dir: &Path) -> OslResult<PathBuf> {
    let mut candidates = Vec::new();
    collect_kicad_netlist_candidates(project_dir, project_dir, &mut candidates)?;
    candidates.sort_by(|left, right| {
        kicad_candidate_score(right)
            .cmp(&kicad_candidate_score(left))
            .then_with(|| left.display().to_string().cmp(&right.display().to_string()))
    });
    candidates.into_iter().next().ok_or_else(|| {
        OslError::InvalidInput(format!(
            "{} does not contain an importable KiCad SPICE netlist (.cir, .spice, .sp)",
            project_dir.display()
        ))
    })
}

fn collect_kicad_schematic_candidates(
    root: &Path,
    dir: &Path,
    candidates: &mut Vec<PathBuf>,
) -> OslResult<()> {
    let entries =
        fs::read_dir(dir).map_err(|err| OslError::io(format!("read {}", dir.display()), err))?;
    for entry in entries {
        let entry = entry.map_err(|err| OslError::io(format!("read {}", dir.display()), err))?;
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().and_then(|name| name.to_str()) == Some("project") {
                continue;
            }
            if path
                .strip_prefix(root)
                .ok()
                .is_some_and(|relative| relative.components().count() > 3)
            {
                continue;
            }
            collect_kicad_schematic_candidates(root, &path, candidates)?;
        } else if is_kicad_schematic(&path) {
            candidates.push(path);
        }
    }
    Ok(())
}

fn collect_kicad_netlist_candidates(
    root: &Path,
    dir: &Path,
    candidates: &mut Vec<PathBuf>,
) -> OslResult<()> {
    let entries =
        fs::read_dir(dir).map_err(|err| OslError::io(format!("read {}", dir.display()), err))?;
    for entry in entries {
        let entry = entry.map_err(|err| OslError::io(format!("read {}", dir.display()), err))?;
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().and_then(|name| name.to_str()) == Some("project") {
                continue;
            }
            if path
                .strip_prefix(root)
                .ok()
                .is_some_and(|relative| relative.components().count() > 3)
            {
                continue;
            }
            collect_kicad_netlist_candidates(root, &path, candidates)?;
        } else if is_spice_netlist_file(&path) {
            candidates.push(path);
        }
    }
    Ok(())
}

fn is_spice_netlist_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "cir" | "spice" | "sp"
            )
        })
        .unwrap_or(false)
}

fn kicad_candidate_score(path: &Path) -> usize {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let mut score = 0;
    if name.contains("kicad") {
        score += 20;
    }
    if name.ends_with(".cir") {
        score += 5;
    }
    if let Ok(content) = read_text(path) {
        let lowered = content.to_ascii_lowercase();
        if lowered.contains("kicad") || lowered.contains("eeschema") {
            score += 50;
        }
        if lowered.contains(".tran")
            || lowered.contains(".op")
            || lowered.contains(".ac")
            || lowered.contains(".dc")
        {
            score += 10;
        }
    }
    score
}

#[derive(Debug, Default)]
struct KicadProjectSourceHints {
    schematic_stems: Vec<String>,
}

impl KicadProjectSourceHints {
    fn push_stem(&mut self, value: Option<&str>) {
        let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
            return;
        };
        let normalized = value.to_ascii_lowercase();
        if !self.schematic_stems.contains(&normalized) {
            self.schematic_stems.push(normalized);
        }
    }

    fn push_path_stem(&mut self, path: &Path) {
        self.push_stem(path.file_stem().and_then(|stem| stem.to_str()));
    }

    fn push_project(&mut self, project: &KicadProject) {
        for candidate in project.schematic_stem_candidates() {
            self.push_stem(Some(&candidate));
        }
    }
}

fn kicad_project_source_hints_for_file(path: &Path) -> KicadProjectSourceHints {
    let mut hints = KicadProjectSourceHints::default();
    hints.push_path_stem(path);
    if let Ok(project) = read_kicad_project(path) {
        hints.push_project(&project);
    }
    hints
}

fn kicad_project_source_hints_for_dir(project_dir: &Path) -> KicadProjectSourceHints {
    let mut hints = KicadProjectSourceHints::default();
    hints.push_path_stem(project_dir);
    if let Ok(entries) = fs::read_dir(project_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if is_kicad_project_file(&path) {
                hints.push_path_stem(&path);
                if let Ok(project) = read_kicad_project(&path) {
                    hints.push_project(&project);
                }
            }
        }
    }
    hints
}

fn kicad_schematic_candidate_score(
    project_dir: &Path,
    hints: &KicadProjectSourceHints,
    path: &Path,
) -> usize {
    let project_name = project_dir
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let stem = path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let mut score = 0;
    if !project_name.is_empty() && stem == project_name {
        score += 50;
    }
    if hints.schematic_stems.iter().any(|hint| hint == &stem) {
        score += 120;
    }
    if path.parent() == Some(project_dir) {
        score += 10;
    }
    if let Ok(content) = read_text(path) {
        let lowered = content.to_ascii_lowercase();
        if lowered.contains("(kicad_sch") {
            score += 50;
        }
        if lowered.contains("(sheet ") {
            score += 5;
        }
        if lowered.contains("(symbol") {
            score += 10;
        }
        if lowered.contains(".tran")
            || lowered.contains(".op")
            || lowered.contains(".ac")
            || lowered.contains(".dc")
        {
            score += 10;
        }
    }
    score
}
