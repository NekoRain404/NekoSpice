use super::NekoSpiceApp;
use super::localization::UiText;
use super::theme::{StudioTheme, StudioThemeMode};
use eframe::egui;

impl NekoSpiceApp {
    pub(super) fn draw_home_insights_panel(&mut self, ui: &mut egui::Ui) {
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
                self.active_workspace = super::navigation::StudioWorkspace::Review;
            }
            if assistant_button(ui, mode, &sim_suggest) {
                self.active_workspace = super::navigation::StudioWorkspace::Simulation;
            }
            if assistant_button(ui, mode, &explain) {
                if let Some(run) = &self.simulation_panel.last_run {
                    self.status_message = Some(format!("Last: {} ({}ms)", run.metadata.status.as_str(), run.metadata.duration_ms));
                } else {
                    self.status_message = Some("No simulation data".to_string());
                }
            }
            if assistant_button(ui, mode, &optimize) {
                self.active_workspace = super::navigation::StudioWorkspace::Optimization;
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
            let power_ok = self.text(UiText::PowerBudgetOk);

            if insight_row(ui, mode, &stability_title,
                    "Phase margin is below 60 deg", &view_label) {
                self.active_workspace = super::navigation::StudioWorkspace::Review;
            }
            if insight_row(ui, mode, &thd_title,
                    "THD improved by 12 dB vs. last run", &view_label) {
                self.active_workspace = super::navigation::StudioWorkspace::Waveforms;
            }
            if insight_row(ui, mode, &noise_title,
                    "Input trace variance is above the recent baseline", &view_label) {
                self.active_workspace = super::navigation::StudioWorkspace::Waveforms;
            }
            if insight_row(ui, mode, &power_title,
                    &power_ok, &view_label) {
                self.active_workspace = super::navigation::StudioWorkspace::Reports;
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
                self.active_workspace = super::navigation::StudioWorkspace::Waveforms;
            }
            if shortcut_row(ui, mode, &design_title) {
                self.active_workspace = super::navigation::StudioWorkspace::Review;
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
