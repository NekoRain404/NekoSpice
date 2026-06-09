use super::NekoSpiceApp;
use super::theme::StudioTheme;
use eframe::egui::{self, RichText};
use osl_core::RunStatus;
use osl_kicad::KicadDiagnosticSeverity;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StudioStatusSnapshot {
    pub(super) project_name: String,
    pub(super) source_path: String,
    pub(super) solver_status: String,
    pub(super) document_state: String,
    pub(super) diagnostics: DiagnosticCounts,
    pub(super) selected_item: String,
    pub(super) waveform_status: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct DiagnosticCounts {
    pub(super) errors: usize,
    pub(super) warnings: usize,
    pub(super) info: usize,
}

impl DiagnosticCounts {
    fn total(self) -> usize {
        self.errors + self.warnings + self.info
    }
}

impl NekoSpiceApp {
    pub(super) fn studio_status_snapshot(&self) -> StudioStatusSnapshot {
        let project_name = self
            .document
            .as_ref()
            .and_then(|document| document.path().file_stem())
            .and_then(|stem| stem.to_str())
            .unwrap_or("No project")
            .to_string();
        let source_path = self
            .document
            .as_ref()
            .map(|document| document.path().display().to_string())
            .unwrap_or_else(|| self.schematic_path.clone());
        let document_state = self
            .document
            .as_ref()
            .map(|document| {
                if document.is_dirty() {
                    "Unsaved changes"
                } else {
                    "Saved"
                }
            })
            .unwrap_or("No document")
            .to_string();
        let diagnostics = self
            .document
            .as_ref()
            .map(|document| {
                let report = document.check_report();
                DiagnosticCounts {
                    errors: report.error_count(),
                    warnings: report.warning_count(),
                    info: report.info_count(),
                }
            })
            .unwrap_or_default();
        let selected_item = self
            .selected_hit
            .as_ref()
            .map(|hit| format!("{}: {}", hit.kind, hit.label))
            .unwrap_or_else(|| "No selection".to_string());

        StudioStatusSnapshot {
            project_name,
            source_path,
            solver_status: self.solver_status_text(),
            document_state,
            diagnostics,
            selected_item,
            waveform_status: self.waveform_status_text(),
        }
    }

    pub(super) fn draw_top_status_strip(&self, ui: &mut egui::Ui) {
        let snapshot = self.studio_status_snapshot();
        ui.horizontal(|ui| {
            ui.heading("NekoSpice Studio");
            ui.separator();
            status_block(ui, "Project", &snapshot.project_name);
            status_block(ui, "Solver", &snapshot.solver_status);
            ui.label(StudioTheme::status_dot(
                if self.simulation_panel.active_task.is_some() {
                    StudioTheme::WARNING
                } else {
                    StudioTheme::SUCCESS
                },
            ));
            ui.label(StudioTheme::muted(&snapshot.document_state));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(StudioTheme::muted("wgpu"));
                ui.separator();
                ui.label(StudioTheme::muted(&snapshot.waveform_status));
            });
        });
    }

    pub(super) fn draw_bottom_status_strip(&self, ui: &mut egui::Ui) {
        let snapshot = self.studio_status_snapshot();
        ui.horizontal(|ui| {
            ui.label(StudioTheme::muted(format!(
                "Workspace: {}",
                snapshot.source_path
            )));
            ui.separator();
            ui.label(diagnostic_text(snapshot.diagnostics));
            ui.separator();
            ui.label(StudioTheme::muted(snapshot.selected_item));
            if let Some(message) = &self.status_message {
                ui.separator();
                ui.label(StudioTheme::accent(message));
            }
        });
    }

    fn solver_status_text(&self) -> String {
        if self.simulation_panel.active_task.is_some() {
            return "ngspice running".to_string();
        }
        if let Some(run) = &self.simulation_panel.last_run {
            return match run.metadata.status {
                RunStatus::Passed => format!("ngspice passed in {} ms", run.metadata.duration_ms),
                RunStatus::Failed => format!("ngspice failed in {} ms", run.metadata.duration_ms),
            };
        }
        if self.simulation_panel.last_error.is_some() {
            "ngspice error".to_string()
        } else {
            "ngspice ready".to_string()
        }
    }

    fn waveform_status_text(&self) -> String {
        let Some(run) = &self.simulation_panel.last_run else {
            return "No waveform".to_string();
        };
        match &run.waveform {
            crate::waveform_summary::GuiWaveformSummaryState::Ready(summary) => {
                format!("{} signals", summary.variable_count)
            }
            crate::waveform_summary::GuiWaveformSummaryState::Missing { .. } => {
                "No waveform.raw".to_string()
            }
            crate::waveform_summary::GuiWaveformSummaryState::Error { .. } => {
                "Waveform error".to_string()
            }
        }
    }
}

fn status_block(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.vertical(|ui| {
        ui.label(RichText::new(label).small().color(StudioTheme::TEXT_MUTED));
        ui.label(RichText::new(value).strong().color(StudioTheme::TEXT));
    });
}

fn diagnostic_text(counts: DiagnosticCounts) -> RichText {
    let color = if counts.errors > 0 {
        StudioTheme::DANGER
    } else if counts.warnings > 0 {
        StudioTheme::WARNING
    } else {
        StudioTheme::SUCCESS
    };
    RichText::new(format!(
        "{} diagnostics ({} errors, {} warnings, {} info)",
        counts.total(),
        counts.errors,
        counts.warnings,
        counts.info
    ))
    .color(color)
}

pub(super) fn severity_color(severity: KicadDiagnosticSeverity) -> eframe::egui::Color32 {
    match severity {
        KicadDiagnosticSeverity::Info => StudioTheme::ACCENT,
        KicadDiagnosticSeverity::Warning => StudioTheme::WARNING,
        KicadDiagnosticSeverity::Error => StudioTheme::DANGER,
    }
}

#[cfg(test)]
mod tests {
    use super::{DiagnosticCounts, diagnostic_text};

    #[test]
    fn diagnostic_summary_counts_all_severities() {
        let text = diagnostic_text(DiagnosticCounts {
            errors: 1,
            warnings: 2,
            info: 3,
        })
        .text()
        .to_string();

        assert!(text.contains("6 diagnostics"));
        assert!(text.contains("1 errors"));
    }
}
