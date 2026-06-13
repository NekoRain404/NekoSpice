//! Waveform workspace — center panel for waveform analysis.
//!
//! Provides analysis tabs (Time Domain, Bode, FFT, Noise, Eye) and
//! displays simulation waveform data with cursor overlay and measurements.

use crate::app::NekoSpiceApp;
use crate::app::localization::UiText;
use crate::app::theme::StudioTheme;
use eframe::egui;
use std::collections::HashSet;

/// Analysis tabs available in the waveform workspace.
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

/// Viewport state for interactive waveform viewing.
///
/// Tracks the visible time/frequency range and supports zoom/pan
/// via mouse scroll and drag interactions.
#[derive(Debug, Clone)]
pub(crate) struct WaveformViewport {
    pub(crate) x_min: f64,
    pub(crate) x_max: f64,
    pub(crate) y_min: f64,
    pub(crate) y_max: f64,
    pub(crate) user_modified: bool,
}

impl Default for WaveformViewport {
    fn default() -> Self {
        Self {
            x_min: 0.0,
            x_max: 1.0,
            y_min: -1.0,
            y_max: 1.0,
            user_modified: false,
        }
    }
}

impl WaveformViewport {
    /// Zoom in/out by a factor around the given center point.
    pub(crate) fn zoom(&mut self, factor: f64, center_x: f64) {
        let x_range = self.x_max - self.x_min;
        let y_range = self.y_max - self.y_min;
        let new_x_range = x_range / factor;
        let new_y_range = y_range / factor;
        let x_ratio = ((center_x - self.x_min) / x_range).clamp(0.0, 1.0);
        self.x_min = center_x - new_x_range * x_ratio;
        self.x_max = center_x + new_x_range * (1.0 - x_ratio);
        self.y_min += (y_range - new_y_range) * 0.5;
        self.y_max -= (y_range - new_y_range) * 0.5;
        self.user_modified = true;
    }

    /// Pan by a fraction of the visible range.
    pub(crate) fn pan(&mut self, dx: f64, dy: f64) {
        let x_range = self.x_max - self.x_min;
        let y_range = self.y_max - self.y_min;
        self.x_min += dx * x_range;
        self.x_max += dx * x_range;
        self.y_min -= dy * y_range;
        self.y_max -= dy * y_range;
        self.user_modified = true;
    }

    /// Reset viewport to fit the given data range.
    pub(crate) fn fit_to_data(&mut self, x_min: f64, x_max: f64, y_min: f64, y_max: f64) {
        let x_margin = (x_max - x_min).abs() * 0.05;
        let y_margin = (y_max - y_min).abs().max(1.0) * 0.1;
        self.x_min = x_min - x_margin;
        self.x_max = x_max + x_margin;
        self.y_min = y_min - y_margin;
        self.y_max = y_max + y_margin;
        self.user_modified = false;
    }
}

/// Persistent state for the waveform workspace.
#[derive(Debug, Default)]
pub(crate) struct WaveformWorkspaceState {
    /// Currently active analysis tab.
    pub(crate) analysis_tab: WaveformAnalysisTab,
    /// Whether cursor overlay is enabled.
    pub(crate) cursor_overlay: bool,
    /// Interactive viewport for zoom/pan.
    pub(crate) viewport: WaveformViewport,
    /// Cursor position in data coordinates (if cursor overlay is active).
    pub(crate) cursor_x: Option<f64>,
    pub(crate) cursor_y: Option<f64>,
    /// Whether the user is currently dragging to pan.
    pub(crate) is_panning: bool,
    /// Drag start position for pan calculation.
    pub(crate) pan_start: Option<egui::Pos2>,
    /// When true, all visible signals are drawn in overlay on a single lane.
    pub(crate) overlay_mode: bool,
    /// Set of signal names currently visible in overlay mode.
    /// If empty and overlay_mode is true, all signals are shown.
    pub(crate) visible_signals: HashSet<String>,
}

impl WaveformWorkspaceState {
    /// Toggle a signal's visibility in overlay mode.
    pub(crate) fn toggle_signal(&mut self, signal: &str) {
        if self.visible_signals.contains(signal) {
            self.visible_signals.remove(signal);
        } else {
            self.visible_signals.insert(signal.to_string());
        }
    }

    /// Check if a signal should be displayed.
    /// In overlay mode, checks the visible_signals set.
    /// If the set is empty (all toggled off), shows everything.
    pub(crate) fn is_signal_visible(&self, signal: &str) -> bool {
        if !self.overlay_mode {
            return true;
        }
        self.visible_signals.is_empty() || self.visible_signals.contains(signal)
    }
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
                if let Some(run) = &self.simulation_panel.last_run
                    && let crate::waveform_summary::GuiWaveformSummaryState::Ready(summary) =
                        &run.waveform
                {
                    ui.label(StudioTheme::muted_for(
                        mode,
                        format!("{} signals", summary.variable_count),
                    ));
                    ui.separator();
                }
            });
        });
    }
}
