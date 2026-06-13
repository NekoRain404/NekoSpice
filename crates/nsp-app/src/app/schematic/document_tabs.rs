//! 原理图文档标签栏。显示当前加载的原理图和层次化子图纸标签。

use super::workspace_widgets::document_tab;
use crate::app::NekoSpiceApp;
use crate::app::localization::UiText;
use eframe::egui;

impl NekoSpiceApp {
    /// 文档标签栏：显示已加载的原理图和子图纸。
    pub(crate) fn draw_schematic_document_tabs(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let palette = self.theme_palette();
        ui.horizontal_wrapped(|ui| {
            // 根原理图标签
            let active_name = self
                .document
                .as_ref()
                .and_then(|document| document.path().file_name())
                .and_then(|name| name.to_str())
                .unwrap_or(self.text(UiText::NoDocument));
            if document_tab(ui, mode, active_name, true).clicked() {
                self.status_message = Some(format!("Root schematic: {active_name}"));
            }

            // 层次化子图纸标签 — 点击打开子图纸文件
            if let Some(document) = &self.document {
                let scene = document.scene();
                let sheets: Vec<_> = scene
                    .sheets
                    .iter()
                    .map(|s| (s.name.clone(), s.file.clone()))
                    .collect();
                for (name, file) in sheets {
                    let tab_label = if file.is_empty() {
                        name.clone()
                    } else {
                        file.clone()
                    };
                    if document_tab(ui, mode, &tab_label, false).clicked() {
                        if !file.is_empty() {
                            if let Some(document) = &self.document {
                                let base_dir = document
                                    .path()
                                    .parent()
                                    .map(|p| p.to_path_buf())
                                    .unwrap_or_default();
                                let sub_path = base_dir.join(&file);
                                if sub_path.exists() {
                                    self.load_schematic(sub_path);
                                    self.status_message =
                                        Some(format!("Opened sub-sheet: {tab_label}"));
                                } else {
                                    self.status_message =
                                        Some(format!("Sub-sheet not found: {tab_label}"));
                                }
                            }
                        } else {
                            self.status_message = Some(format!("Sub-sheet: {name}"));
                        }
                    }
                }
            }

            // 新建按钮
            if ui
                .add(
                    egui::Button::new(
                        egui::RichText::new("+")
                            .size(14.0)
                            .strong()
                            .color(palette.text),
                    )
                    .fill(palette.panel_soft)
                    .stroke(egui::Stroke::new(1.0, palette.border_strong)),
                )
                .on_hover_text(self.text(UiText::NewSchematic))
                .clicked()
            {
                self.open_file_dialog();
            }
        });
    }
}
