//! 画布键盘快捷键处理。分发键盘事件到对应编辑操作。
//!
use super::EditNudgeDirection;
use super::NekoSpiceApp;
use super::schematic::tools::SchematicTool;
use eframe::egui;

impl NekoSpiceApp {
    /// Handle keyboard shortcuts for the schematic canvas.
    ///
    /// Tool shortcuts: V=Select, W=Wire, L=Label, B=Bus, S=Sheet,
    /// J=Junction, Q=NoConnect, R=Rotate, F=Fit, Del=Delete, Esc=Cancel.
    /// Navigation: Arrow keys for nudge, Ctrl+Z/Y for undo/redo.
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

        // --- Undo / Redo ---
        if ui.input(|input| input.modifiers.ctrl && input.key_pressed(egui::Key::Z)) {
            if ui.input(|input| input.modifiers.shift) {
                self.redo();
            } else {
                self.undo();
            }
        }
        // F5 = Run simulation
        if ui.input(|input| input.key_pressed(egui::Key::F5)) {
            self.run_simulation_from_panel();
        }
        // Ctrl+S = Save
        if ui.input(|input| input.modifiers.ctrl && input.key_pressed(egui::Key::S)) {
            self.save_document();
        }
        // Ctrl+Y as alternative redo shortcut
        if ui.input(|input| input.modifiers.ctrl && input.key_pressed(egui::Key::Y)) {
            self.redo();
        }
        // Ctrl+O = Open file
        if ui.input(|input| input.modifiers.ctrl && input.key_pressed(egui::Key::O)) {
            self.open_file_dialog();
        }
        // Ctrl+Shift+S = Save As
        if ui.input(|input| input.modifiers.ctrl && input.modifiers.shift && input.key_pressed(egui::Key::S)) {
            self.save_document_with_dialog();
        }
        // Ctrl+Shift+E = Export netlist
        if ui.input(|input| input.modifiers.ctrl && input.modifiers.shift && input.key_pressed(egui::Key::E)) {
            self.export_netlist_dialog();
        }
    }
}
