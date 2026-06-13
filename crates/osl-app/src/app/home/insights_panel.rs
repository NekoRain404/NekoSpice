//! Home insights panel — recent simulation results, waveform summaries, and alerts.

use crate::app::NekoSpiceApp;
use crate::app::localization::UiText;
use crate::app::theme::{StudioTheme, StudioThemeMode};
use eframe::egui;

impl NekoSpiceApp {
    /// draw home insights panel。
    pub(crate) fn draw_home_insights_panel(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        ui.heading(self.text(UiText::AiAssistant));
        ui.label(StudioTheme::muted_for(
            mode,
            self.text(UiText::AssistantPrompt),
        ));
        ui.add_space(8.0);
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            let stability = self.text(UiText::CheckLoopStability);
            let sim_suggest = self.text(UiText::SuggestSimulations);
            let explain = self.text(UiText::ExplainWaveform);
            let optimize = self.text(UiText::FindOptimization);
            if assistant_button(ui, mode, &stability) {
                self.active_workspace = crate::app::navigation::StudioWorkspace::Review;
            }
            if assistant_button(ui, mode, &sim_suggest) {
                self.active_workspace = crate::app::navigation::StudioWorkspace::Simulation;
            }
            if assistant_button(ui, mode, &explain) {
                if let Some(run) = &self.simulation_panel.last_run {
                    self.status_message = Some(format!("Last: {} ({}ms)", run.metadata.status.as_str(), run.metadata.duration_ms));
                } else {
                    self.status_message = Some("No simulation data".to_string());
                }
            }
            if assistant_button(ui, mode, &optimize) {
                self.active_workspace = crate::app::navigation::StudioWorkspace::Optimization;
            }
            ui.separator();
            ui.label(StudioTheme::muted_for(mode, self.text(UiText::AskCircuit)));
        });

        ui.add_space(8.0);
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            section_header(
                ui,
                mode,
                self.text(UiText::Insights),
                self.text(UiText::ViewAll),
            );
            let view_label = self.text(UiText::View);
            let stability_title = self.text(UiText::StabilityRisk);
            let thd_title = self.text(UiText::HighThd);
            let noise_title = self.text(UiText::MeasurementNoise);
            let power_title = self.text(UiText::PowerCheck);

            // Generate insights from real data
            let has_doc = self.document.is_some();
            let has_run = self.simulation_panel.last_run.is_some();
            let has_errors = self.simulation_panel.last_error.is_some();
            let report = self.document.as_ref().map(|d| d.check_report());
            let error_count = report.as_ref().map(|r| r.error_count()).unwrap_or(0);
            let warning_count = report.as_ref().map(|r| r.warning_count()).unwrap_or(0);
            let waveform_vars = self.simulation_panel.last_run.as_ref()
                .and_then(|run| match &run.waveform {
                    crate::waveform_summary::GuiWaveformSummaryState::Ready(s) => Some(s.variable_count),
                    _ => None,
                })
                .unwrap_or(0);

            // ERC findings
            if error_count > 0 {
                if insight_row(ui, mode, &stability_title,
                        &format!("{error_count} ERC errors found — review schematic"), &view_label) {
                    self.active_workspace = crate::app::navigation::StudioWorkspace::Schematic;
                }
            } else if warning_count > 0 {
                if insight_row(ui, mode, &stability_title,
                        &format!("{warning_count} ERC warnings — consider fixing"), &view_label) {
                    self.active_workspace = crate::app::navigation::StudioWorkspace::Review;
                }
            }

            // Simulation results
            if has_run && waveform_vars > 0 {
                if insight_row(ui, mode, &thd_title,
                        &format!("Simulation complete — {waveform_vars} signals captured"), &view_label) {
                    self.active_workspace = crate::app::navigation::StudioWorkspace::Waveforms;
                }
            } else if has_errors {
                if insight_row(ui, mode, &thd_title,
                        "Last simulation failed — check log for errors", &view_label) {
                    self.active_workspace = crate::app::navigation::StudioWorkspace::Simulation;
                }
            }

            // Schematic status
            if has_doc && !has_run {
                if insight_row(ui, mode, &noise_title,
                        "Ready to simulate — run a simulation to validate", &view_label) {
                    self.active_workspace = crate::app::navigation::StudioWorkspace::Simulation;
                }
            } else if has_run {
                if insight_row(ui, mode, &noise_title,
                        &self.simulation_panel.backend.label().to_string(), &view_label) {
                    self.active_workspace = crate::app::navigation::StudioWorkspace::Simulation;
                }
            }

            // System health
            let threads = std::thread::available_parallelism().map_or(1, |n| n.get());
            if insight_row(ui, mode, &power_title,
                    &format!("System: {threads} threads, wgpu renderer active"), &view_label) {
                self.active_workspace = crate::app::navigation::StudioWorkspace::Settings;
            }
        });

        ui.add_space(8.0);
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            section_header(
                ui,
                mode,
                self.text(UiText::Shortcuts),
                self.text(UiText::Edit),
            );
            let wav_title = self.text(UiText::WaveformViewer);
            let design_title = self.text(UiText::DesignChecklist);
            let import_title = self.text(UiText::ImportMeasurementData);
            if shortcut_row(ui, mode, &wav_title) {
                self.active_workspace = crate::app::navigation::StudioWorkspace::Waveforms;
            }
            if shortcut_row(ui, mode, &design_title) {
                self.active_workspace = crate::app::navigation::StudioWorkspace::Review;
            }
            if shortcut_row(ui, mode, &import_title) {
                self.open_file_dialog();
            }
        });
    }
}

fn assistant_button(ui: &mut egui::Ui, mode: StudioThemeMode, text: &str) -> bool {
    let response = ui.add_sized(
        [ui.available_width(), 32.0],
        egui::Button::new(format!("@  {}", text))
            .fill(StudioTheme::palette(mode).panel_soft)
            .stroke(egui::Stroke::new(1.0, StudioTheme::palette(mode).border))
            .corner_radius(egui::CornerRadius::same(4)),
    ).on_hover_text(text);
    response.clicked()
}

fn section_header(ui: &mut egui::Ui, mode: StudioThemeMode, title: &str, action: &str) {
    ui.horizontal(|ui| {
        ui.label(StudioTheme::section_title_for(mode, title));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(StudioTheme::accent_for(mode, action));
        });
    });
}

fn insight_row(
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
    title: &str,
    caption: &str,
    view_text: &str,
) -> bool {
    let resp = ui.horizontal(|ui| {
        ui.label(StudioTheme::status_dot(StudioTheme::palette(mode).warning));
        ui.vertical(|ui| {
            ui.label(title);
            ui.label(StudioTheme::muted_for(mode, caption));
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(StudioTheme::accent_for(mode, view_text));
        });
    }).response;
    ui.separator();
    resp.interact(egui::Sense::click()).clicked()
}

fn shortcut_row(ui: &mut egui::Ui, mode: StudioThemeMode, title: &str) -> bool {
    let response = ui.horizontal(|ui| {
        ui.label(StudioTheme::accent_for(mode, "#"));
        ui.label(title);
    }).response;
    ui.separator();
    response.interact(egui::Sense::click()).clicked()
}
