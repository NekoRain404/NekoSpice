use crate::app::NekoSpiceApp;
use crate::app::localization::UiText;
use crate::app::navigation::StudioWorkspace;
use super::state::ReviewSeverityFilter;
use super::widgets::{
    REVIEW_FINDINGS, review_filter_row, review_issue_row, review_metric_row,
    review_recommendation_row, review_stat_row, severity_color,
};
use crate::app::theme::StudioTheme;
use crate::app::waveform::preview::draw_stacked_waveform_preview;
use crate::waveform_summary::GuiWaveformSummaryState;
use eframe::egui;

impl NekoSpiceApp {
    /// draw review center workspace。
    pub(crate) fn draw_review_center_workspace(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        ui.heading("Design Review");
        ui.label(StudioTheme::muted_for(
            mode,
            "Triaged schematic risks, recent run context, and recommended next steps.",
        ));
        ui.add_space(10.0);
        egui::ScrollArea::vertical()
            .id_salt("review_center_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let spacing = 10.0;
                let width = ui.available_width();
                if width < 560.0 {
                    ui.vertical(|ui| {
                        self.draw_review_score_overview(ui);
                        ui.add_space(spacing);
                        self.draw_review_action_queue(ui);
                    });
                } else {
                    let score_width = ((width - spacing) * 0.42).max(280.0);
                    let action_width = (width - score_width - spacing).max(320.0);
                    ui.horizontal_top(|ui| {
                        ui.vertical(|ui| {
                            ui.set_width(score_width);
                            self.draw_review_score_overview(ui);
                        });
                        ui.add_space(spacing);
                        ui.vertical(|ui| {
                            ui.set_width(action_width);
                            self.draw_review_action_queue(ui);
                        });
                    });
                }

                ui.add_space(10.0);
                self.draw_review_risk_summary(ui);
                ui.add_space(10.0);
                self.draw_review_checklist_board(ui);
                ui.add_space(10.0);
                self.draw_review_recommendation_board(ui);
                ui.add_space(10.0);
                self.draw_review_waveform_board(ui);
            });
    }

    /// draw review workspace panel。
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
                        crate::waveform_summary::GuiWaveformSummaryState::Ready(s) => s.variable_count,
                        _ => 0,
                    };
                    self.status_message = Some(
                        format!("Run: {} ({}ms, {}) — {} signals", status, ms, backend, signals)
                    );
                } else {
                    self.status_message = Some("No simulation data yet".to_string());
                }
            }
        });

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
            review_stat_row(
                ui,
                mode,
                self.text(UiText::Suggestions),
                "2",
                palette.success,
            );
        });

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

    fn draw_review_score_overview(&self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let palette = self.theme_palette();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::DesignReview),
            ));
            ui.horizontal(|ui| {
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(78.0, 78.0), egui::Sense::hover());
                let painter = ui.painter_at(rect);
                painter.circle_stroke(rect.center(), 30.0, egui::Stroke::new(8.0, palette.border));
                painter.circle_stroke(rect.center(), 30.0, egui::Stroke::new(8.0, palette.success));
                painter.text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "72",
                    egui::FontId::proportional(20.0),
                    palette.text,
                );
                ui.vertical(|ui| {
                    review_metric_row(ui, mode, self.text(UiText::OverallScore), "Good");
                    review_metric_row(ui, mode, self.text(UiText::Critical), "3");
                    review_metric_row(ui, mode, self.text(UiText::Major), "5");
                    review_metric_row(ui, mode, self.text(UiText::Minor), "8");
                    review_metric_row(ui, mode, self.text(UiText::Suggestions), "2");
                });
            });
        });
    }

    fn draw_review_action_queue(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::AiAssistant),
            ));
            ui.label(StudioTheme::muted_for(
                mode,
                "Use the current canvas state to branch into simulation or optimization.",
            ));
            ui.add_space(6.0);
            if ui.button(self.text(UiText::ExplainWaveform)).clicked() {
                if let Some(run) = &self.simulation_panel.last_run {
                    let status = run.metadata.status.as_str();
                    let ms = run.metadata.duration_ms;
                    self.status_message = Some(
                        format!("Last run: {} ({}ms)", status, ms)
                    );
                } else {
                    self.status_message = Some("No simulation data yet".to_string());
                }
            }
            if ui.button(self.text(UiText::FindOptimization)).clicked() {
                self.active_workspace = StudioWorkspace::Optimization;
            }
            if ui.button(self.text(UiText::VerifyStability)).clicked() {
                self.active_workspace = StudioWorkspace::Simulation;
            }
            if ui.button("Open Inspector").clicked() {
                self.active_workspace = StudioWorkspace::Schematic;
            }
        });
    }

    fn draw_review_risk_summary(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let palette = self.theme_palette();
        let locale = self.locale();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::Issues),
            ));
            ui.horizontal_wrapped(|ui| {
                for filter in ReviewSeverityFilter::ALL {
                    ui.selectable_value(
                        &mut self.review_workspace.severity_filter,
                        filter,
                        filter.label(locale),
                    );
                }
            });
            ui.separator();
            let filter = self.review_workspace.severity_filter;
            for finding in REVIEW_FINDINGS {
                if filter.matches(finding.severity) {
                    review_issue_row(
                        ui,
                        mode,
                        finding.severity.label(locale),
                        finding.title,
                        finding.detail,
                        severity_color(palette, finding.severity),
                    );
                }
            }
        });
    }

    fn draw_review_recommendation_board(&self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let palette = self.theme_palette();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::TopRecommendations),
            ));
            review_recommendation_row(
                ui,
                mode,
                "Improve Stability Margin",
                "Increase compensation or lower gain",
                "High",
                palette.danger,
            );
            review_recommendation_row(
                ui,
                mode,
                "Increase Power Supply Decoupling",
                "Add local 100 nF and bulk capacitors",
                "High",
                palette.danger,
            );
            review_recommendation_row(
                ui,
                mode,
                "Add Output Isolation",
                "Place small series resistor before capacitive load",
                "Medium",
                palette.warning,
            );
        });
    }

    fn draw_review_waveform_board(&self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::WaveformInsight),
            ));
            match &self.simulation_panel.last_run {
                Some(run) => match &run.waveform {
                    GuiWaveformSummaryState::Ready(summary) => {
                        draw_stacked_waveform_preview(ui, mode, summary, None, 170.0);
                    }
                    GuiWaveformSummaryState::Missing { raw_path } => {
                        ui.label(StudioTheme::muted_for(
                            mode,
                            format!("Waveform raw file not found: {}", raw_path.display()),
                        ));
                    }
                    GuiWaveformSummaryState::Error { raw_path, message } => {
                        ui.label(StudioTheme::muted_for(
                            mode,
                            format!("Could not read {}: {}", raw_path.display(), message),
                        ));
                    }
                },
                None => {
                    ui.label(StudioTheme::muted_for(
                        mode,
                        "No recent simulation data. Run ngspice to populate the preview.",
                    ));
                }
            }
        });
    }
}
