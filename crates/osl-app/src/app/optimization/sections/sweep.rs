//! 参数扫描面板。配置 `.step` 指令参数（线性/倍频/列表扫描），
//! 编辑扫描变量的起止值和采样数。

use crate::app::NekoSpiceApp;
use crate::app::localization::UiText;
use super::super::state::SweepParam;
use crate::app::theme::StudioTheme;
use eframe::egui;

impl NekoSpiceApp {
    /// Draw the parametric sweep panel with editable sweep parameters.
    pub(crate) fn draw_sweep_panel(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(mode, self.text(UiText::ParametricSweep)));

            // Show current .step directive if configured
            match &self.simulation_panel.step_sweep {
                crate::app::simulation::state::StepSweep::None => {
                    ui.label(StudioTheme::muted_for(mode, "No .step directive configured"));
                }
                sweep => {
                    let text = format!("{:?}", sweep);
                    ui.label(StudioTheme::accent_for(mode, &text));
                }
            }

            ui.add_space(6.0);

            let has_params = !self.optimization_workspace.sweep_params.is_empty();
            if has_params {
                egui::Grid::new("optimization_sweep_grid")
                    .num_columns(5)
                    .spacing(egui::Vec2::new(8.0, 6.0))
                    .striped(true)
                    .show(ui, |ui| {
                        ui.strong(self.text(UiText::Parameter));
                        ui.strong("Start");
                        ui.strong("Stop");
                        ui.strong(self.text(UiText::Samples));
                        ui.strong("");
                        ui.end_row();

                        let mut remove_idx = None;
                        for (i, param) in self.optimization_workspace.sweep_params.iter_mut().enumerate() {
                            ui.text_edit_singleline(&mut param.name);
                            ui.text_edit_singleline(&mut param.start);
                            ui.text_edit_singleline(&mut param.stop);
                            ui.text_edit_singleline(&mut param.count);
                            if ui.small_button("X").on_hover_text("Remove").clicked() {
                                remove_idx = Some(i);
                            }
                            ui.end_row();
                        }
                        if let Some(idx) = remove_idx {
                            self.optimization_workspace.sweep_params.remove(idx);
                        }
                    });
            } else {
                ui.label(StudioTheme::muted_for(mode, "No sweep parameters defined. Add one below."));
            }

            ui.add_space(4.0);
            if ui.button("+ Add Parameter").clicked() {
                self.optimization_workspace.sweep_params.push(SweepParam::default());
            }
        });
    }
}
