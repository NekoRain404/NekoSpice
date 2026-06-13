//! 优化目标编辑面板。允许用户添加、编辑和删除优化目标，
//! 设置每个目标的名称、优化方向（minimize/maximize）和约束条件。

use super::super::state::OptimizationTarget;
use super::super::widgets::result_row;
use crate::app::NekoSpiceApp;
use crate::app::localization::UiText;
use crate::app::theme::StudioTheme;
use eframe::egui;

impl NekoSpiceApp {
    /// Draw the optimization targets panel with editable rows.
    pub(crate) fn draw_targets_panel(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::Objective),
            ));

            let has_targets = !self.optimization_workspace.targets.is_empty();
            if has_targets {
                ui.horizontal(|ui| {
                    ui.label(StudioTheme::muted_for(mode, "Name"));
                    ui.label(StudioTheme::muted_for(mode, "Goal"));
                    ui.label(StudioTheme::muted_for(mode, "Constraint"));
                });
                ui.separator();

                let mut remove_idx = None;
                for (i, target) in self.optimization_workspace.targets.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(&mut target.name);
                        egui::ComboBox::from_id_salt(format!("target_goal_{i}"))
                            .selected_text(&target.goal)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut target.goal,
                                    "minimize".to_string(),
                                    "minimize",
                                );
                                ui.selectable_value(
                                    &mut target.goal,
                                    "maximize".to_string(),
                                    "maximize",
                                );
                            });
                        ui.text_edit_singleline(&mut target.constraint);
                        if ui
                            .small_button("X")
                            .on_hover_text("Remove target")
                            .clicked()
                        {
                            remove_idx = Some(i);
                        }
                    });
                }
                if let Some(idx) = remove_idx {
                    self.optimization_workspace.targets.remove(idx);
                }
            } else {
                ui.label(StudioTheme::muted_for(
                    mode,
                    "No optimization targets defined. Add one below.",
                ));
            }

            ui.add_space(4.0);
            if ui.button("+ Add Target").clicked() {
                self.optimization_workspace
                    .targets
                    .push(OptimizationTarget::default());
            }

            ui.add_space(8.0);
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::CandidateResults),
            ));

            let run_count = self
                .simulation_panel
                .last_run
                .as_ref()
                .map(|_| 1)
                .unwrap_or(0);
            if run_count > 0 {
                result_row(ui, mode, "Best", "from last run", "current");
            } else {
                ui.label(StudioTheme::muted_for(
                    mode,
                    "Run a simulation to see results",
                ));
            }
        });
    }
}
