//! 优化工作区中心区段。按面板类型拆分为子模块：
//! - `targets` — 优化目标编辑面板
//! - `sweep` — 参数扫描面板
//! - `monte_carlo` — 蒙特卡洛分析面板

mod targets;
mod sweep;
mod monte_carlo;

use crate::app::localization::UiText;
use crate::app::NekoSpiceApp;
use crate::app::theme::StudioTheme;
use eframe::egui;

impl NekoSpiceApp {
    /// Draw the optimization sub-tab selector.
    pub(crate) fn draw_optimization_tabs(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            for tab in super::state::OptimizationTab::ALL {
                let label = self.text(tab.text_key());
                ui.selectable_value(&mut self.optimization_workspace.active_tab, tab, label);
            }
        });
    }

    /// Dispatch to the correct sub-panel based on active tab.
    pub(crate) fn draw_optimization_main_panel(&mut self, ui: &mut egui::Ui) {
        match self.optimization_workspace.active_tab {
            super::state::OptimizationTab::Targets => self.draw_targets_panel(ui),
            super::state::OptimizationTab::Sweep => self.draw_sweep_panel(ui),
            super::state::OptimizationTab::MonteCarlo => self.draw_monte_carlo_panel(ui),
        }
    }

    /// Draw optimization summary panel with real or empty state.
    pub(crate) fn draw_optimization_summary_panel(&self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let ws = &self.optimization_workspace;
        let total = ws.mc_completed;
        let passed = ws.mc_passed;
        let yield_ratio = if total > 0 { passed as f32 / total as f32 } else { 0.0 };

        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(mode, self.text(UiText::StatisticalSummary)));

            if total > 0 {
                ui.horizontal(|ui| {
                    super::widgets::mini_donut(ui, mode, yield_ratio);
                    ui.vertical(|ui| {
                        let yield_pct = format!("{:.1}%", yield_ratio * 100.0);
                        super::widgets::status_chip(ui, "PASS", StudioTheme::palette(mode).success);
                        super::widgets::parameter_row(ui, mode, self.text(UiText::Yield),
                            &format!("{} / {}", passed, total), &yield_pct);
                        super::widgets::parameter_row(ui, mode, self.text(UiText::Completed),
                            &total.to_string(), &format!("{}%", 100));
                    });
                });
            } else {
                ui.label(StudioTheme::muted_for(mode, "Run Monte Carlo analysis to see results"));
            }
        });
    }
}
