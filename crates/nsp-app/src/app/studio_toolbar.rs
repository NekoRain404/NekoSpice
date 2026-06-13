//! 顶部工具栏。绘制文件操作、编辑操作和视图切换的图标按钮行。
//!
use super::localization::UiText;
use super::navigation::StudioWorkspace;
use super::{NekoSpiceApp, theme::StudioTheme};
use eframe::egui;

impl NekoSpiceApp {
    /// draw studio top bar。
    pub(super) fn draw_studio_top_bar(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        ui.horizontal(|ui| {
            self.draw_top_status_strip(ui);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(StudioTheme::muted_for(mode, "wgpu"));
                ui.separator();
                if ui
                    .button(self.text(UiText::Settings))
                    .on_hover_text(self.text(UiText::Settings))
                    .clicked()
                {
                    self.active_workspace = StudioWorkspace::Settings;
                }
                if ui
                    .button(self.locale().short_code())
                    .on_hover_text(self.text(UiText::Language))
                    .clicked()
                {
                    self.toggle_locale();
                }
                if ui
                    .button(self.theme_mode_label(self.theme_mode()))
                    .on_hover_text(self.text(UiText::Theme))
                    .clicked()
                {
                    self.toggle_theme_mode();
                }
                ui.separator();
                if ui
                    .add_enabled(
                        self.document.is_some(),
                        egui::Button::new(self.text(UiText::Run)),
                    )
                    .on_hover_text(self.text(UiText::RunHint))
                    .clicked()
                {
                    self.run_simulation_from_panel();
                    self.active_workspace = StudioWorkspace::Simulation;
                }
                if ui
                    .button(self.text(UiText::Fit))
                    .on_hover_text(self.text(UiText::FitHint))
                    .clicked()
                {
                    self.viewport
                        .fit_scene(self.scene.as_ref().and_then(|scene| scene.bounds));
                }
                if ui
                    .add_enabled(
                        self.document.is_some(),
                        egui::Button::new(self.text(UiText::Save)),
                    )
                    .on_hover_text(self.text(UiText::SaveHint))
                    .clicked()
                {
                    self.save_document();
                }
                if ui
                    .button(self.text(UiText::Open))
                    .on_hover_text(self.text(UiText::OpenHint))
                    .clicked()
                {
                    self.open_file_dialog();
                }
            });
        });
    }
}
