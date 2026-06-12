//! 原理图工作区顶部工具栏。包含文件操作、编辑操作、缩放控制、绘图工具切换、仿真状态、DRC 状态。

use crate::app::NekoSpiceApp;
use super::workspace_widgets::{canvas_toolbar_button, toolbar_icon_button_active};
use crate::app::theme::StudioTheme;
use crate::app::navigation::StudioWorkspace;
use eframe::egui;

impl NekoSpiceApp {
    /// 工具栏行：文件操作、绘制工具、缩放控制、仿真状态、DRC 状态。
    pub(crate) fn draw_schematic_workspace_toolbar(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let palette = self.theme_palette();

        ui.horizontal(|ui| {
            // ── 文件操作组 ──────────────────────────────────────
            if canvas_toolbar_button(ui, mode, "\u{25B6} Open", true).clicked() {
                self.open_file_dialog();
            }
            if canvas_toolbar_button(ui, mode, "\u{2913} Save", self.document.is_some()).clicked() {
                self.save_document_with_dialog();
            }
            ui.add_space(2.0);
            if canvas_toolbar_button(ui, mode, "\u{21A9} Undo", self.history.can_undo()).clicked() {
                self.undo();
            }
            if canvas_toolbar_button(ui, mode, "\u{21AA} Redo", self.history.can_redo()).clicked() {
                self.redo();
            }
            ui.add_space(2.0);
            if canvas_toolbar_button(ui, mode, "\u{2316} Fit", true).clicked() {
                self.viewport
                    .fit_scene(self.scene.as_ref().and_then(|scene| scene.bounds));
            }

            // ── Run 按钮（带仿真状态指示） ──────────────────────
            let running = self.simulation_panel.active_task.is_some();
            let run_color = if running {
                palette.warning
            } else if let Some(run) = &self.simulation_panel.last_run {
                match run.metadata.status {
                    osl_core::RunStatus::Passed => palette.success,
                    osl_core::RunStatus::Failed => palette.danger,
                }
            } else {
                palette.text_muted
            };
            let run_label = if running {
                "\u{25B6} Running..."
            } else {
                "\u{25B6} Run"
            };
            let run_tooltip = format!(
                "Run {} {} via {}",
                self.simulation_panel.directive_kind,
                self.simulation_panel.analysis_params.to_body().trim(),
                self.simulation_panel.backend.label(),
            );
            if canvas_toolbar_button(ui, mode, run_label, self.document.is_some())
                .on_hover_text(run_tooltip)
                .clicked()
            {
                self.run_simulation_from_panel();
            }
            // Simulation status dot after Run button
            if self.simulation_panel.last_run.is_some() || running {
                ui.label(StudioTheme::status_dot(run_color));
            }

            // ── 分隔符 ─────────────────────────────────────────
            ui.add_space(6.0);
            ui.separator();
            ui.add_space(6.0);

            // ── 绘图工具切换 ───────────────────────────────────
            use super::tools::SchematicTool;
            let tools: &[(&str, &str, SchematicTool)] = &[
                ("\u{250C}", "Wire (W)", SchematicTool::Wire),
                ("\u{2190}", "Label (L)", SchematicTool::Label),
                ("\u{2550}", "Bus (B)", SchematicTool::Bus),
                ("\u{25A3}", "Sheet (S)", SchematicTool::Sheet),
                ("\u{2B24}", "Junction (J)", SchematicTool::Junction),
                ("\u{2716}", "NoConn (Q)", SchematicTool::NoConnect),
            ];
            for &(icon, tooltip, tool) in tools {
                let is_active = self.schematic_tools.active == tool;
                if toolbar_icon_button_active(ui, mode, icon, tooltip, true, is_active).clicked() {
                    self.activate_schematic_tool_direct(tool);
                }
            }

            // ── 分隔符 ─────────────────────────────────────────
            ui.add_space(6.0);
            ui.separator();
            ui.add_space(6.0);

            // ── 缩放控制 ───────────────────────────────────────
            if canvas_toolbar_button(ui, mode, "-", true).clicked() {
                if let Some(rect) = self.last_canvas_rect {
                    self.viewport.zoom_around(rect, rect.center(), 0.8);
                } else {
                    self.viewport.zoom = (self.viewport.zoom * 0.8).max(1.0);
                }
            }
            ui.label(StudioTheme::accent_for(
                mode,
                format!("{:.0}%", self.viewport.zoom * 100.0),
            ));
            if canvas_toolbar_button(ui, mode, "+", true).clicked() {
                if let Some(rect) = self.last_canvas_rect {
                    self.viewport.zoom_around(rect, rect.center(), 1.25);
                } else {
                    self.viewport.zoom = (self.viewport.zoom * 1.25).min(180.0);
                }
            }

            // ── 分隔符 ─────────────────────────────────────────
            ui.add_space(6.0);
            ui.separator();

            // ── 后端选择器（交互式） ─────────────────────────────
            ui.label(StudioTheme::muted_for(mode, self.simulation_panel.backend.label()));
            ui.separator();

            // ── DRC 状态 ──────────────────────────────────────
            ui.label(StudioTheme::muted_for(mode, "DRC"));
            let report = self.document.as_ref().map(|doc| doc.check_report());
            let (dot_color, drc_text) = match report {
                Some(r) if r.error_count() > 0 => (palette.danger, format!("{} errors", r.error_count())),
                Some(r) if r.warning_count() > 0 => (palette.warning, format!("{} warnings", r.warning_count())),
                Some(_) => (palette.success, "Clean".to_string()),
                None => (palette.text_muted, "No doc".to_string()),
            };
            ui.label(
                egui::RichText::new(format!("\u{25CF} {drc_text}"))
                    .color(dot_color)
                    .size(12.0),
            );

            // ── 波形快捷链接 ──────────────────────────────────
            if self.simulation_panel.last_run.is_some() {
                ui.separator();
                if ui.small_button("Waveforms").on_hover_text("Open waveform analysis workspace").clicked() {
                    self.active_workspace = StudioWorkspace::Waveforms;
                }
            }

            // ── 状态消息 ──────────────────────────────────────
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(msg) = &self.status_message {
                    ui.label(StudioTheme::accent_for(mode, msg));
                }
            });
        });
    }
}
