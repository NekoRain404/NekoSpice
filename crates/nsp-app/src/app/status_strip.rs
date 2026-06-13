//! 底部状态栏。显示仿真状态、最近运行结果和全局操作按钮。
//!
use super::NekoSpiceApp;
use super::localization::UiText;
use super::theme::{StudioTheme, StudioThemeMode};
use eframe::egui::{self, RichText};
use nsp_core::RunStatus;
use nsp_schema::NspDiagnosticSeverity;

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
    /// studio status snapshot。
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

    /// draw top status strip。
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

    /// draw bottom status strip。
    /// draw bottom status strip with simulation progress and context info.
    pub(super) fn draw_bottom_status_strip(&self, ui: &mut egui::Ui) {
        let snapshot = self.studio_status_snapshot();
        let mode = self.theme_mode();
        let palette = self.theme_palette();
        ui.horizontal(|ui| {
            // Simulation status (prominent when running)
            if self.simulation_panel.active_task.is_some() {
                ui.label(StudioTheme::status_dot(palette.warning));
                ui.label(
                    egui::RichText::new(format!(
                        "Simulating ({})",
                        self.simulation_panel.backend.label()
                    ))
                    .color(palette.warning)
                    .strong(),
                );
                ui.separator();
            } else if let Some(run) = &self.simulation_panel.last_run {
                let color = match run.metadata.status {
                    nsp_core::RunStatus::Passed => palette.success,
                    nsp_core::RunStatus::Failed => palette.danger,
                };
                ui.label(StudioTheme::status_dot(color));
                ui.label(
                    egui::RichText::new(format!(
                        "{} {}ms",
                        run.metadata.status.as_str(),
                        run.metadata.duration_ms,
                    ))
                    .color(color)
                    .small(),
                );
                ui.separator();
            }

            // Cursor world coordinates (schematic workspace)
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
            // Analysis type indicator
            ui.label(StudioTheme::muted_for(
                mode,
                format!(".{}", self.simulation_panel.directive_kind),
            ));
            ui.separator();
            // Waveform status
            ui.label(StudioTheme::muted_for(mode, &snapshot.waveform_status));
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
        let backend = self.simulation_panel.backend.label();
        if self.simulation_panel.active_task.is_some() {
            return format!("{} running", backend);
        }
        if let Some(run) = &self.simulation_panel.last_run {
            return match run.metadata.status {
                RunStatus::Passed => {
                    format!("{} passed in {} ms", backend, run.metadata.duration_ms)
                }
                RunStatus::Failed => {
                    format!("{} failed in {} ms", backend, run.metadata.duration_ms)
                }
            };
        }
        if self.simulation_panel.last_error.is_some() {
            format!("{} error", backend)
        } else {
            format!("{} ready", backend)
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
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).small().color(palette.text_muted));
        ui.label(RichText::new(value).small().strong().color(palette.text));
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

/// severity color。
pub(super) fn severity_color(
    mode: StudioThemeMode,
    severity: NspDiagnosticSeverity,
) -> eframe::egui::Color32 {
    let palette = StudioTheme::palette(mode);
    match severity {
        NspDiagnosticSeverity::Info => palette.accent,
        NspDiagnosticSeverity::Warning => palette.warning,
        NspDiagnosticSeverity::Error => palette.danger,
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
