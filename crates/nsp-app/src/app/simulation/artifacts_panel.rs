//! Simulation artifacts panel — lists output files, logs, and reports from a completed run.

use crate::simulation::GuiSimulationRun;
use eframe::egui;
use nsp_core::Artifact;
use std::fs;
use std::path::Path;

/// draw simulation artifacts panel。
pub(crate) fn draw_simulation_artifacts_panel(ui: &mut egui::Ui, run: &GuiSimulationRun) {
    if run.metadata.artifacts.is_empty() {
        return;
    }

    ui.label(format!("{} artifacts", run.metadata.artifacts.len()));
    egui::Grid::new("simulation_run_artifacts")
        .num_columns(3)
        .spacing(egui::Vec2::new(8.0, 2.0))
        .striped(true)
        .show(ui, |ui| {
            ui.strong("Kind");
            ui.strong("File");
            ui.strong("Size");
            ui.end_row();

            for artifact in &run.metadata.artifacts {
                draw_artifact_row(ui, &run.output_dir, artifact);
                ui.end_row();
            }
        });
}

fn draw_artifact_row(ui: &mut egui::Ui, output_dir: &Path, artifact: &Artifact) {
    ui.label(&artifact.kind);

    if artifact.path == "report.html" {
        ui.strong(&artifact.path);
    } else {
        ui.monospace(&artifact.path);
    }

    ui.label(artifact_size_text(&output_dir.join(&artifact.path)));
}

fn artifact_size_text(path: &Path) -> String {
    let Ok(metadata) = fs::metadata(path) else {
        return "-".to_string();
    };
    format_bytes(metadata.len())
}

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;

    if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

#[cfg(test)]
mod tests {
    use super::format_bytes;

    #[test]
    fn formats_artifact_size() {
        assert_eq!(format_bytes(32), "32 B");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(3 * 1024 * 1024), "3.0 MB");
    }
}
