//! Waveform workspace — center panel for waveform analysis.
//!
//! Provides analysis tabs (Time Domain, Bode, FFT, Noise, Eye) and
//! displays simulation waveform data with cursor overlay and measurements.

use crate::app::NekoSpiceApp;
use crate::app::localization::UiText;
use crate::app::theme::StudioTheme;
use eframe::egui;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum WaveformAnalysisTab {
    #[default]
    TimeDomain,
    Bode,
    Fft,
    Noise,
    Eye,
}

impl WaveformAnalysisTab {
    pub(crate) const ALL: [Self; 5] = [
        Self::TimeDomain,
        Self::Bode,
        Self::Fft,
        Self::Noise,
        Self::Eye,
    ];

    pub(crate) fn text_key(self) -> UiText {
        match self {
            Self::TimeDomain => UiText::TimeDomain,
            Self::Bode => UiText::BodePlot,
            Self::Fft => UiText::FftAnalysis,
            Self::Noise => UiText::NoiseAnalysis,
            Self::Eye => UiText::EyeDiagram,
        }
    }

    /// Description of what this analysis tab shows.
    pub(crate) fn description(self) -> &'static str {
        match self {
            Self::TimeDomain => "Voltage/current vs. time",
            Self::Bode => "Magnitude and phase vs. frequency",
            Self::Fft => "Frequency spectrum of time-domain signals",
            Self::Noise => "Noise spectral density",
            Self::Eye => "Eye diagram for signal integrity",
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct WaveformWorkspaceState {
    pub(crate) analysis_tab: WaveformAnalysisTab,
    pub(crate) cursor_overlay: bool,
}

impl NekoSpiceApp {
    /// Draw waveform center workspace.
    pub(crate) fn draw_waveform_center_workspace(&mut self, ui: &mut egui::Ui) {
        self.poll_simulation_task();
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("waveform_center_scroll")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    self.draw_waveform_workspace_header(ui);
                    ui.add_space(10.0);
                    self.draw_waveform_workspace_toolbar(ui);
                    ui.add_space(10.0);
                    self.draw_waveform_plot_section(ui);
                    ui.add_space(10.0);
                    self.draw_waveform_detail_sections(ui);
                });
        });
    }

    fn draw_waveform_workspace_header(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.heading(self.text(UiText::WaveformAnalysis));
                // Show analysis tab description as subtitle
                let tab_desc = self.waveform_workspace.analysis_tab.description();
                ui.label(StudioTheme::muted_for(mode, tab_desc));
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
                // Show signal count if available
                if let Some(run) = &self.simulation_panel.last_run {
                    if let crate::waveform_summary::GuiWaveformSummaryState::Ready(summary) =
                        &run.waveform
                    {
                        ui.label(StudioTheme::muted_for(
                            mode,
                            format!("{} signals", summary.variable_count),
                        ));
                        ui.separator();
                    }
                }
            });
        });
    }
}
