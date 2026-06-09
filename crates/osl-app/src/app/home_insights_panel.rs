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
            assistant_button(ui, mode, self.text(UiText::CheckLoopStability));
            assistant_button(ui, mode, self.text(UiText::SuggestSimulations));
            assistant_button(ui, mode, self.text(UiText::ExplainWaveform));
            assistant_button(ui, mode, self.text(UiText::FindOptimization));
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
            insight_row(
                ui,
                mode,
                self.text(UiText::StabilityRisk),
                "Phase margin is below 60 deg",
                self.text(UiText::View),
            );
            insight_row(
                ui,
                mode,
                self.text(UiText::HighThd),
                "THD improved by 12 dB vs. last run",
                self.text(UiText::View),
            );
            insight_row(
                ui,
                mode,
                self.text(UiText::MeasurementNoise),
                "Input trace variance is above the recent baseline",
                self.text(UiText::View),
            );
            insight_row(
                ui,
                mode,
                self.text(UiText::PowerCheck),
                self.text(UiText::PowerBudgetOk),
                self.text(UiText::View),
            );
        });

        ui.add_space(8.0);
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            section_header(
                ui,
                mode,
                self.text(UiText::Shortcuts),
                self.text(UiText::Edit),
            );
            shortcut_row(ui, mode, self.text(UiText::WaveformViewer));
            shortcut_row(ui, mode, self.text(UiText::DesignChecklist));
            shortcut_row(ui, mode, self.text(UiText::ImportMeasurementData));
        });
    }
}

fn assistant_button(ui: &mut egui::Ui, mode: StudioThemeMode, text: &str) {
    let response = ui.add_sized(
        [ui.available_width(), 32.0],
        egui::Button::new(format!("@  {}", text)).fill(StudioTheme::palette(mode).panel_soft),
    );
    response.on_hover_text(text);
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
) {
    ui.horizontal(|ui| {
        ui.label(StudioTheme::status_dot(StudioTheme::palette(mode).warning));
        ui.vertical(|ui| {
            ui.label(title);
            ui.label(StudioTheme::muted_for(mode, caption));
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(StudioTheme::accent_for(mode, view_text));
        });
    });
    ui.separator();
}

fn shortcut_row(ui: &mut egui::Ui, mode: StudioThemeMode, title: &str) {
    ui.horizontal(|ui| {
        ui.label(StudioTheme::accent_for(mode, "#"));
        ui.label(title);
    });
    ui.separator();
}
