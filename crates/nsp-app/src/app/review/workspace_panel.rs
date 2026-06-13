//! 审查工作空间侧边面板。提供快速操作入口和风险概览。

use super::state::ReviewSeverityFilter;
use super::widgets::{review_filter_row, review_stat_row};
use crate::app::NekoSpiceApp;
use crate::app::localization::UiText;
use crate::app::navigation::StudioWorkspace;
use crate::app::theme::StudioTheme;
use eframe::egui;

impl NekoSpiceApp {
    /// 审查工作空间侧边面板 — 提供快速操作按钮和风险快照。
    pub(crate) fn draw_review_workspace_panel(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let palette = self.theme_palette();
        let locale = self.locale();
        ui.heading("Review");
        ui.label(StudioTheme::muted_for(
            mode,
            "Launch the schematic audit, simulation, or optimization workspace.",
        ));
        ui.add_space(10.0);

        // 快速操作按钮
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(mode, "Review Actions"));
            ui.add_space(4.0);
            if ui.button(self.text(UiText::OpenSchematic)).clicked() {
                self.active_workspace = StudioWorkspace::Schematic;
            }
            if ui.button(self.text(UiText::RunSimulation)).clicked() {
                self.active_workspace = StudioWorkspace::Simulation;
            }
            if ui.button(self.text(UiText::FindOptimization)).clicked() {
                self.active_workspace = StudioWorkspace::Optimization;
            }
            if ui.button(self.text(UiText::ExplainWaveform)).clicked() {
                if let Some(run) = &self.simulation_panel.last_run {
                    let status = run.metadata.status.as_str();
                    let ms = run.metadata.duration_ms;
                    let backend = &run.metadata.backend;
                    let signals = match &run.waveform {
                        crate::waveform_summary::GuiWaveformSummaryState::Ready(s) => {
                            s.variable_count
                        }
                        _ => 0,
                    };
                    self.status_message = Some(format!(
                        "Run: {} ({}ms, {}) — {} signals",
                        status, ms, backend, signals
                    ));
                } else {
                    self.status_message = Some("No simulation data yet".to_string());
                }
            }
        });

        // 风险快照过滤器
        ui.add_space(10.0);
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(mode, "Risk Snapshot"));
            review_filter_row(
                ui,
                mode,
                &mut self.review_workspace.severity_filter,
                ReviewSeverityFilter::Critical,
                ReviewSeverityFilter::Critical.label(locale),
                "3",
                palette.danger,
            );
            review_filter_row(
                ui,
                mode,
                &mut self.review_workspace.severity_filter,
                ReviewSeverityFilter::Major,
                ReviewSeverityFilter::Major.label(locale),
                "5",
                palette.warning,
            );
            review_filter_row(
                ui,
                mode,
                &mut self.review_workspace.severity_filter,
                ReviewSeverityFilter::Minor,
                ReviewSeverityFilter::Minor.label(locale),
                "8",
                palette.accent,
            );
        });

        // 审查摘要
        ui.add_space(10.0);
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(mode, "Review Summary"));
            ui.add_space(4.0);
            review_stat_row(ui, mode, "Total Issues", "10", palette.text);
            review_stat_row(ui, mode, "Critical", "3", palette.danger);
            review_stat_row(ui, mode, "Review Score", "72/100", palette.success);
        });

        // 最近上下文
        ui.add_space(10.0);
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(mode, "Recent Context"));
            ui.label(StudioTheme::muted_for(
                mode,
                self.simulation_panel
                    .last_run
                    .as_ref()
                    .map(|run| format!("Run: {}", run.output_dir.display()))
                    .unwrap_or_else(|| "No recent simulation run".to_string()),
            ));
            ui.label(StudioTheme::muted_for(
                mode,
                self.load_error
                    .as_ref()
                    .map(|error| format!("Load warning: {}", error))
                    .unwrap_or_else(|| "Schematic load is healthy".to_string()),
            ));
        });
    }
}
