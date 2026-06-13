//! 原理图工作区中心面板。编排工具栏、文档标签栏、画布区域和底部停靠面板。

use crate::app::NekoSpiceApp;
use crate::app::theme::StudioTheme;
use eframe::egui::{self, Vec2};

impl NekoSpiceApp {
    /// 原理图工作区主入口：布局工具栏、标签栏、画布、检查器和底部停靠面板。
    pub(crate) fn draw_schematic_center_workspace(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            self.draw_schematic_workspace_toolbar(ui);
            ui.add_space(6.0);
            self.draw_schematic_document_tabs(ui);
            ui.add_space(6.0);

            let canvas_height = (ui.available_height() - 220.0).max(280.0);
            let inspector_width = 280.0;

            ui.allocate_ui_with_layout(
                Vec2::new(ui.available_width(), canvas_height),
                egui::Layout::left_to_right(egui::Align::Min),
                |ui| {
                    // 左侧垂直工具面板
                    let _palette_width = self.draw_tool_palette(ui);

                    // 中央画布区域（减去检查器宽度）
                    let canvas_width = (ui.available_width() - inspector_width - 8.0).max(200.0);
                    ui.allocate_ui_with_layout(
                        Vec2::new(canvas_width, canvas_height),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| self.draw_canvas(ui),
                    );

                    // 右侧属性检查器面板
                    ui.add_space(6.0);
                    ui.allocate_ui_with_layout(
                        Vec2::new(inspector_width, canvas_height),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            egui::ScrollArea::vertical()
                                .max_height(canvas_height)
                                .show(ui, |ui| {
                                    self.draw_schematic_inspector_panel(ui);
                                });
                        },
                    );
                },
            );

            ui.add_space(6.0);
            self.draw_schematic_bottom_dock(ui);
        });
    }
}
