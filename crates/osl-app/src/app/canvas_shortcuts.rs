//! 画布键盘快捷键处理。分发键盘事件到对应编辑操作。
//!
//! 本模块仅处理原理图画布编辑相关的快捷键（工具切换、微调、
//! 删除、旋转等）。全局快捷键（F5、Ctrl+S 等）在
//! [`global_shortcuts`] 中统一处理。

use super::EditNudgeDirection;
use super::NekoSpiceApp;
use super::schematic::tools::SchematicTool;
use eframe::egui;

impl NekoSpiceApp {
    /// Handle keyboard shortcuts for the schematic canvas.
    ///
    /// Tool shortcuts: V=Select, W=Wire, L=Label, B=Bus, S=Sheet,
    /// J=Junction, Q=NoConnect, R=Rotate, F=Fit, Del=Delete, Esc=Cancel.
    /// Navigation: Arrow keys for nudge.
    ///
    /// Global shortcuts (F5, Ctrl+S, Ctrl+O, Ctrl+Z, etc.) are handled
    /// by [`super::global_shortcuts`].
    pub(super) fn handle_canvas_shortcuts(&mut self, ui: &egui::Ui) {
        if ui.ctx().text_edit_focused() {
            return;
        }

        // --- Tool switching shortcuts ---
        if ui.input(|input| input.key_pressed(egui::Key::V)) {
            self.activate_schematic_tool_direct(SchematicTool::Select);
        }
        if ui.input(|input| input.key_pressed(egui::Key::W)) {
            self.activate_schematic_tool_direct(SchematicTool::Wire);
        }
        if ui.input(|input| input.key_pressed(egui::Key::L)) {
            self.activate_schematic_tool_direct(SchematicTool::Label);
        }
        if ui.input(|input| input.key_pressed(egui::Key::B)) {
            self.activate_schematic_tool_direct(SchematicTool::Bus);
        }
        if ui.input(|input| input.key_pressed(egui::Key::S)) {
            self.activate_schematic_tool_direct(SchematicTool::Sheet);
        }
        if ui.input(|input| input.key_pressed(egui::Key::J)) {
            self.activate_schematic_tool_direct(SchematicTool::Junction);
        }
        if ui.input(|input| input.key_pressed(egui::Key::Q)) {
            self.activate_schematic_tool_direct(SchematicTool::NoConnect);
        }

        // --- Action shortcuts ---
        if ui.input(|input| input.key_pressed(egui::Key::Escape)) {
            self.cancel_symbol_placement();
            self.cancel_schematic_tool_pending();
            self.activate_schematic_tool_direct(SchematicTool::Select);
        }
        if ui.input(|input| input.key_pressed(egui::Key::Delete)) {
            self.delete_selected();
        }
        // ? key toggles keyboard shortcut help overlay
        if ui.input(|input| input.key_pressed(egui::Key::Slash) && input.modifiers.shift) {
            self.show_shortcuts_overlay = !self.show_shortcuts_overlay;
        }
        if ui.input(|input| input.key_pressed(egui::Key::R)) {
            self.rotate_selected();
        }
        if ui.input(|input| input.key_pressed(egui::Key::F)) {
            self.viewport
                .fit_scene(self.scene.as_ref().and_then(|scene| scene.bounds));
        }

        // --- Arrow key nudging ---
        if ui.input(|input| input.key_pressed(egui::Key::ArrowLeft)) {
            self.nudge_selected(EditNudgeDirection::Left);
        }
        if ui.input(|input| input.key_pressed(egui::Key::ArrowRight)) {
            self.nudge_selected(EditNudgeDirection::Right);
        }
        if ui.input(|input| input.key_pressed(egui::Key::ArrowUp)) {
            self.nudge_selected(EditNudgeDirection::Up);
        }
        if ui.input(|input| input.key_pressed(egui::Key::ArrowDown)) {
            self.nudge_selected(EditNudgeDirection::Down);
        }
    }
}
