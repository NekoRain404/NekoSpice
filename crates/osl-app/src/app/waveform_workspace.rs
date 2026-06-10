use super::NekoSpiceApp;
use super::localization::UiText;
use super::theme::StudioTheme;
use eframe::egui;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum WaveformAnalysisTab {
    #[default]
    TimeDomain,
    Bode,
    Fft,
    Noise,
    Eye,
}

impl WaveformAnalysisTab {
    pub(super) const ALL: [Self; 5] = [
        Self::TimeDomain,
        Self::Bode,
        Self::Fft,
        Self::Noise,
        Self::Eye,
    ];

    pub(super) fn text_key(self) -> UiText {
        match self {
            Self::TimeDomain => UiText::TimeDomain,
            Self::Bode => UiText::BodePlot,
            Self::Fft => UiText::FftAnalysis,
            Self::Noise => UiText::NoiseAnalysis,
            Self::Eye => UiText::EyeDiagram,
        }
    }
}

#[derive(Debug, Default)]
pub(super) struct WaveformWorkspaceState {
    pub(super) analysis_tab: WaveformAnalysisTab,
    pub(super) cursor_overlay: bool,
}

impl NekoSpiceApp {
    pub(super) fn draw_waveform_center_workspace(&mut self, ui: &mut egui::Ui) {
        self.poll_simulation_task();
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("waveform_center_scroll")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    self.draw_waveform_workspace_header(ui);
                    ui.add_space(8.0);
                    self.draw_waveform_workspace_toolbar(ui);
                    ui.add_space(8.0);
                    self.draw_waveform_plot_section(ui);
                    ui.add_space(8.0);
                    self.draw_waveform_detail_sections(ui);
                });
        });
    }

    fn draw_waveform_workspace_header(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.heading(self.text(UiText::WaveformAnalysis));
                ui.label(StudioTheme::muted_for(
                    mode,
                    self.text(UiText::WaveformAnalysisCaption),
                ));
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                let running = self.simulation_panel.active_task.is_some();
                if ui
                    .add_enabled(
                        self.document.is_some() && !running,
                        egui::Button::new(self.text(UiText::RunSimulation)),
                    )
                    .clicked()
                {
                    self.run_simulation_from_panel();
                }
            });
        });
    }
}
