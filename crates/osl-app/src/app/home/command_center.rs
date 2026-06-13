//! Home command center — metric cards showing project stats and recommended next steps.

use crate::app::NekoSpiceApp;
use super::dashboard::SECTION_GAP;
use crate::app::localization::UiText;
use crate::app::navigation::StudioWorkspace;
use crate::app::theme::{StudioTheme, StudioThemeMode};
use eframe::egui::{self, Color32, RichText, Vec2};

impl NekoSpiceApp {
    /// draw home command center。
    pub(crate) fn draw_home_command_center(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::CommandCenter),
            ));
            ui.add_space(6.0);
            self.draw_home_command_metrics(ui);
            ui.add_space(SECTION_GAP);
            self.draw_home_next_steps(ui);
        });
    }

    fn draw_home_command_metrics(&self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let palette = self.theme_palette();
        let columns = if ui.available_width() >= 760.0 { 4 } else { 2 };
        let spacing = 8.0;
        let width =
            ((ui.available_width() - spacing * (columns - 1) as f32) / columns as f32).max(120.0);

        egui::Grid::new("home_command_metrics")
            .num_columns(columns)
            .spacing(Vec2::new(spacing, spacing))
            .show(ui, |ui| {
                for (index, card) in self.home_command_metric_cards().into_iter().enumerate() {
                    command_metric_card(ui, mode, width, card);
                    if (index + 1) % columns == 0 {
                        ui.end_row();
                    }
                }
            });

        ui.label(StudioTheme::muted_for(
            mode,
            format!(
                "{}: {} / {}: {}",
                self.text(UiText::Renderer),
                "wgpu",
                self.text(UiText::Solver),
                "ngspice"
            ),
        ));
        ui.colored_label(palette.success, self.text(UiText::SystemOperational));
    }

    fn draw_home_next_steps(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        ui.label(StudioTheme::section_title_for(
            mode,
            self.text(UiText::NextSteps),
        ));
        let steps = [
            HomeNextStep {
                label: self.text(UiText::OpenSchematic),
                workspace: StudioWorkspace::Schematic,
            },
            HomeNextStep {
                label: self.text(UiText::RunSimulation),
                workspace: StudioWorkspace::Simulation,
            },
            HomeNextStep {
                label: self.text(UiText::DesignReview),
                workspace: StudioWorkspace::Review,
            },
            HomeNextStep {
                label: self.text(UiText::ModelLibrary),
                workspace: StudioWorkspace::Library,
            },
        ];

        ui.horizontal_wrapped(|ui| {
            for step in steps {
                if next_step_button(ui, mode, step.label) {
                    self.active_workspace = step.workspace;
                }
            }
        });
    }

    fn home_command_metric_cards(&self) -> [HomeMetricCard; 4] {
        let palette = StudioTheme::palette(self.theme_mode());
        let symbol_coverage = self.symbol_coverage_text();
        
        // Real ERC/DRC error count from schematic
        let (error_count, warning_count) = self
            .document
            .as_ref()
            .map(|d| {
                let report = d.check_report();
                (report.error_count(), report.warning_count())
            })
            .unwrap_or((0, 0));
        let issue_text = if error_count + warning_count > 0 {
            format!("{}e / {}w", error_count, warning_count)
        } else {
            self.text(UiText::Ready).to_string()
        };
        let issue_color = if error_count > 0 {
            palette.danger
        } else if warning_count > 0 {
            palette.warning
        } else {
            palette.success
        };

        // Simulation run state
        let (run_state, run_color) = if self.simulation_panel.active_task.is_some() {
            (self.text(UiText::Running).to_string(), palette.warning)
        } else if let Some(run) = &self.simulation_panel.last_run {
            match run.metadata.status {
                osl_core::RunStatus::Passed => (
                    format!("{}ms", run.metadata.duration_ms),
                    palette.success,
                ),
                osl_core::RunStatus::Failed => (
                    self.text(UiText::WaveformError).to_string(),
                    palette.danger,
                ),
            }
        } else {
            (self.text(UiText::Queued).to_string(), palette.text_muted)
        };

        // Backend name
        let backend = self.simulation_panel.backend.label();

        // Waveform signal count
        let waveform_info = self.simulation_panel.last_run.as_ref().map(|run| {
            match &run.waveform {
                crate::waveform_summary::GuiWaveformSummaryState::Ready(s) => {
                    format!("{} signals", s.variable_count)
                }
                _ => self.text(UiText::Missing).to_string(),
            }
        }).unwrap_or_else(|| self.text(UiText::NoWaveform).to_string());

        [
            HomeMetricCard {
                title: self.text(UiText::OpenIssues),
                value: issue_text,
                caption: self.text(UiText::DesignReview).to_string(),
                color: issue_color,
            },
            HomeMetricCard {
                title: self.text(UiText::SymbolCoverage),
                value: symbol_coverage,
                caption: self.text(UiText::LibraryStatus).to_string(),
                color: palette.accent,
            },
            HomeMetricCard {
                title: self.text(UiText::LastRun),
                value: run_state,
                caption: backend.to_string(),
                color: run_color,
            },
            HomeMetricCard {
                title: "Waveforms",
                value: waveform_info,
                caption: self.text(UiText::SimulationQueue).to_string(),
                color: palette.success,
            },
        ]
    }

    fn symbol_coverage_text(&self) -> String {
        let schematic_symbols = self
            .scene
            .as_ref()
            .map(|scene| scene.symbols.len())
            .unwrap_or_default();
        let library_symbols = self
            .library
            .as_ref()
            .map(|library| library.index().symbols.len())
            .unwrap_or_default();
        if schematic_symbols == 0 || library_symbols == 0 {
            return self.text(UiText::Missing).to_string();
        }
        format!("{schematic_symbols} / {library_symbols}")
    }
}

#[derive(Debug)]
struct HomeMetricCard {
    title: &'static str,
    value: String,
    caption: String,
    color: Color32,
}

#[derive(Debug, Clone, Copy)]
struct HomeNextStep {
    label: &'static str,
    workspace: StudioWorkspace,
}

fn command_metric_card(ui: &mut egui::Ui, mode: StudioThemeMode, width: f32, card: HomeMetricCard) {
    let palette = StudioTheme::palette(mode);
    egui::Frame::new()
        .fill(palette.panel_soft)
        .stroke(egui::Stroke::new(1.0, palette.border))
        .corner_radius(6)
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.set_width(width);
            ui.set_min_height(72.0);
            ui.label(StudioTheme::muted_for(mode, card.title));
            ui.label(
                RichText::new(card.value)
                    .size(18.0)
                    .strong()
                    .color(card.color),
            );
            ui.label(StudioTheme::muted_for(mode, card.caption));
        });
}

fn next_step_button(ui: &mut egui::Ui, mode: StudioThemeMode, label: &str) -> bool {
    ui.add(
        egui::Button::new(RichText::new(label).strong())
            .fill(StudioTheme::palette(mode).panel_soft)
            .stroke(egui::Stroke::new(1.0, StudioTheme::palette(mode).border))
            .corner_radius(4),
    )
    .clicked()
}
