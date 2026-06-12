use super::NekoSpiceApp;
use super::localization::UiText;
use super::theme::{StudioTheme, StudioThemeMode};
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
    #[allow(dead_code)]
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
            .unwrap_or(self.text(UiText::NoProject))
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
                    self.text(UiText::UnsavedChanges)
                } else {
                    self.text(UiText::Saved)
                }
            })
            .unwrap_or(self.text(UiText::NoDocument))
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
            .unwrap_or_else(|| self.text(UiText::NoSelection).to_string());

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
        let mode = self.theme_mode();
        let palette = self.theme_palette();
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(self.text(UiText::StudioTitle))
                    .heading()
                    .color(palette.text),
            );
            ui.separator();
            status_block(ui, mode, self.text(UiText::Project), &snapshot.project_name);
            status_block(ui, mode, self.text(UiText::Solver), &snapshot.solver_status);
            ui.label(StudioTheme::status_dot(
                if self.simulation_panel.active_task.is_some() {
                    palette.warning
                } else {
                    palette.success
                },
            ));
            ui.label(StudioTheme::muted_for(mode, &snapshot.document_state));
        });
    }

    pub(super) fn draw_bottom_status_strip(&self, ui: &mut egui::Ui) {
        let snapshot = self.studio_status_snapshot();
        let mode = self.theme_mode();
        ui.horizontal(|ui| {
            // Cursor world coordinates
            if let Some(cursor) = self.cursor_world {
                ui.label(StudioTheme::muted_for(
                    mode,
                    format!("X:{:.1} Y:{:.1}", cursor.x, cursor.y),
                ));
                ui.separator();
            }
            // Active tool indicator
            ui.label(StudioTheme::muted_for(
                mode,
                format!("[{}]", self.schematic_tools.active.label()),
            ));
            ui.separator();
            // Zoom level
            ui.label(StudioTheme::muted_for(
                mode,
                format!("{:.0}%", self.viewport.zoom * 100.0),
            ));
            ui.separator();
            // Element count from scene
            ui.label(StudioTheme::muted_for(mode, snapshot.selected_item));
            if let Some(message) = &self.status_message {
                ui.separator();
                ui.label(StudioTheme::accent_for(mode, message));
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
            return self.text(UiText::NoWaveform).to_string();
        };
        match &run.waveform {
            crate::waveform_summary::GuiWaveformSummaryState::Ready(summary) => {
                format!("{} signals", summary.variable_count)
            }
            crate::waveform_summary::GuiWaveformSummaryState::Missing { .. } => {
                "No waveform.raw".to_string()
            }
            crate::waveform_summary::GuiWaveformSummaryState::Error { .. } => {
                self.text(UiText::WaveformError).to_string()
            }
        }
    }
}

fn status_block(ui: &mut egui::Ui, mode: StudioThemeMode, label: &str, value: &str) {
    let palette = StudioTheme::palette(mode);
    ui.vertical(|ui| {
        ui.label(RichText::new(label).small().color(palette.text_muted));
        ui.label(RichText::new(value).strong().color(palette.text));
    });
}

#[allow(dead_code)]
fn diagnostic_text(mode: StudioThemeMode, counts: DiagnosticCounts) -> RichText {
    let palette = StudioTheme::palette(mode);
    let color = if counts.errors > 0 {
        palette.danger
    } else if counts.warnings > 0 {
        palette.warning
    } else {
        palette.success
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

pub(super) fn severity_color(
    mode: StudioThemeMode,
    severity: KicadDiagnosticSeverity,
) -> eframe::egui::Color32 {
    let palette = StudioTheme::palette(mode);
    match severity {
        KicadDiagnosticSeverity::Info => palette.accent,
        KicadDiagnosticSeverity::Warning => palette.warning,
        KicadDiagnosticSeverity::Error => palette.danger,
    }
}

#[cfg(test)]
mod tests {
    use super::{DiagnosticCounts, diagnostic_text};
    use crate::app::theme::StudioThemeMode;

    #[test]
    fn diagnostic_summary_counts_all_severities() {
        let text = diagnostic_text(
            StudioThemeMode::Midnight,
            DiagnosticCounts {
                errors: 1,
                warnings: 2,
                info: 3,
            },
        )
        .text()
        .to_string();

        assert!(text.contains("6 diagnostics"));
        assert!(text.contains("1 errors"));
    }
}
