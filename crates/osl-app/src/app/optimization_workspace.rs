use super::NekoSpiceApp;
use super::localization::UiText;
use super::theme::{StudioTheme, StudioThemeMode};
use eframe::egui::{self, RichText};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum OptimizationTab {
    #[default]
    Targets,
    Sweep,
    MonteCarlo,
}

impl OptimizationTab {
    const ALL: [Self; 3] = [Self::Targets, Self::Sweep, Self::MonteCarlo];

    fn text_key(self) -> UiText {
        match self {
            Self::Targets => UiText::Optimization,
            Self::Sweep => UiText::ParametricSweep,
            Self::MonteCarlo => UiText::MonteCarlo,
        }
    }
}

#[derive(Debug, Default)]
pub(super) struct OptimizationWorkspaceState {
    active_tab: OptimizationTab,
}

impl NekoSpiceApp {
    pub(super) fn draw_optimization_center_workspace(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.heading(self.text(UiText::OptimizationStudio));
                    ui.label(StudioTheme::muted_for(
                        mode,
                        self.text(UiText::OptimizationCaption),
                    ));
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                    let _ = ui.button(self.text(UiText::StartSweep));
                });
            });
            ui.add_space(8.0);
            self.draw_optimization_tabs(ui);
            ui.add_space(8.0);
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_width((ui.available_width() * 0.58).max(360.0));
                    self.draw_optimization_main_panel(ui);
                });
                ui.add_space(10.0);
                ui.vertical(|ui| {
                    ui.set_width(ui.available_width().max(240.0));
                    self.draw_optimization_summary_panel(ui);
                });
            });
        });
    }

    pub(super) fn draw_optimization_workspace_panel(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        ui.heading(self.text(UiText::OptimizationStudio));
        ui.label(StudioTheme::muted_for(
            mode,
            self.text(UiText::OptimizationCaption),
        ));
        ui.add_space(8.0);
        metric_card(
            ui,
            mode,
            self.text(UiText::Yield),
            "97.6%",
            self.text(UiText::MonteCarlo),
        );
        metric_card(
            ui,
            mode,
            self.text(UiText::Constraints),
            "6",
            self.text(UiText::Ready),
        );
    }

    fn draw_optimization_tabs(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            for tab in OptimizationTab::ALL {
                let label = self.text(tab.text_key());
                ui.selectable_value(&mut self.optimization_workspace.active_tab, tab, label);
            }
        });
    }

    fn draw_optimization_main_panel(&self, ui: &mut egui::Ui) {
        match self.optimization_workspace.active_tab {
            OptimizationTab::Targets => self.draw_targets_panel(ui),
            OptimizationTab::Sweep => self.draw_sweep_panel(ui),
            OptimizationTab::MonteCarlo => self.draw_monte_carlo_panel(ui),
        }
    }

    fn draw_targets_panel(&self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::Objective),
            ));
            parameter_row(ui, mode, "Vout ripple", "minimize", "< 80 mV");
            parameter_row(ui, mode, "Efficiency", "maximize", "> 90%");
            parameter_row(ui, mode, "I(L1) RMS", "minimize", "< 0.8 A");
            ui.separator();
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::CandidateResults),
            ));
            result_row(ui, mode, "A", "R1=1.0k, C1=1u", "+12.4%");
            result_row(ui, mode, "B", "R1=1.2k, C1=820n", "+9.1%");
            result_row(ui, mode, "C", "R1=820, C1=1.5u", "+6.8%");
        });
    }

    fn draw_sweep_panel(&self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::ParametricSweep),
            ));
            egui::Grid::new("optimization_sweep_grid")
                .num_columns(4)
                .spacing(egui::Vec2::new(14.0, 6.0))
                .striped(true)
                .show(ui, |ui| {
                    ui.strong(self.text(UiText::Parameter));
                    ui.strong(self.text(UiText::Range));
                    ui.strong(self.text(UiText::Samples));
                    ui.strong(self.text(UiText::StatusConsole));
                    ui.end_row();
                    sweep_row(ui, "R1", "680 .. 1.5k", "16", "queued");
                    sweep_row(ui, "C1", "470n .. 2.2u", "12", "queued");
                    sweep_row(ui, "TEMP", "-20 .. 85 C", "8", "ready");
                });
        });
    }

    fn draw_monte_carlo_panel(&self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::MonteCarlo),
            ));
            ui.columns(3, |columns| {
                metric_card(
                    &mut columns[0],
                    mode,
                    self.text(UiText::Samples),
                    "1000",
                    "runs",
                );
                metric_card(
                    &mut columns[1],
                    mode,
                    self.text(UiText::Yield),
                    "97.6%",
                    "pass",
                );
                metric_card(
                    &mut columns[2],
                    mode,
                    self.text(UiText::Tolerance),
                    "5%",
                    "R/C",
                );
            });
            ui.separator();
            parameter_row(ui, mode, "Pass", "976", "green");
            parameter_row(ui, mode, "Fail", "24", "review");
        });
    }

    fn draw_optimization_summary_panel(&self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::Constraints),
            ));
            parameter_row(ui, mode, "V(out)", "4.95 .. 5.05 V", "pass");
            parameter_row(ui, mode, "Ripple", "< 80 mV", "pass");
            parameter_row(ui, mode, "I(L1)", "< 0.8 A", "pass");
        });
    }
}

fn metric_card(ui: &mut egui::Ui, mode: StudioThemeMode, label: &str, value: &str, caption: &str) {
    let palette = StudioTheme::palette(mode);
    StudioTheme::panel_frame_for(mode).show(ui, |ui| {
        ui.label(StudioTheme::muted_for(mode, label));
        ui.label(RichText::new(value).strong().size(18.0).color(palette.text));
        ui.label(RichText::new(caption).size(11.0).color(palette.text_muted));
    });
}

fn parameter_row(ui: &mut egui::Ui, mode: StudioThemeMode, label: &str, value: &str, status: &str) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.label(StudioTheme::muted_for(mode, value));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(StudioTheme::accent_for(mode, status));
        });
    });
}

fn result_row(ui: &mut egui::Ui, mode: StudioThemeMode, rank: &str, values: &str, score: &str) {
    ui.horizontal(|ui| {
        ui.label(StudioTheme::accent_for(mode, rank));
        ui.monospace(values);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(StudioTheme::accent_for(mode, score));
        });
    });
}

fn sweep_row(ui: &mut egui::Ui, parameter: &str, range: &str, samples: &str, status: &str) {
    ui.label(parameter);
    ui.monospace(range);
    ui.monospace(samples);
    ui.label(status);
    ui.end_row();
}
