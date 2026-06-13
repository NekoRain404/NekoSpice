//! 全局键盘快捷键处理。从根面板调用，确保快捷键在所有工作区生效。
//!
//! 与 [`canvas_shortcuts`] 不同，本模块不依赖原理图编辑状态，
//! 而是处理文件操作、仿真运行、视口缩放等全局行为。

use super::NekoSpiceApp;
use super::navigation::StudioWorkspace;
use eframe::egui;

impl NekoSpiceApp {
    /// Handle global keyboard shortcuts that should work in every workspace.
    ///
    /// Dispatched from the root panel layout (`panels.rs`) so shortcuts
    /// remain responsive regardless of the active workspace.
    pub(super) fn handle_global_shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.text_edit_focused() {
            return;
        }

        ctx.input(|input| {
            let ctrl = input.modifiers.ctrl;
            let shift = input.modifiers.shift;

            // ── F5: Run simulation ───────────────────────────────────
            if input.key_pressed(egui::Key::F5) {
                self.run_simulation_from_panel();
            }

            // ── Ctrl+S: Save ─────────────────────────────────────────
            if ctrl && !shift && input.key_pressed(egui::Key::S) {
                self.save_document();
            }

            // ── Ctrl+Shift+S: Save As ────────────────────────────────
            if ctrl && shift && input.key_pressed(egui::Key::S) {
                self.save_document_with_dialog();
            }

            // ── Ctrl+O: Open file ────────────────────────────────────
            if ctrl && input.key_pressed(egui::Key::O) {
                self.open_file_dialog();
            }

            // ── Ctrl+Shift+E: Export netlist ─────────────────────────
            if ctrl && shift && input.key_pressed(egui::Key::E) {
                self.export_netlist_dialog();
            }

            // ── Ctrl+Z: Undo ─────────────────────────────────────────
            if ctrl && !shift && input.key_pressed(egui::Key::Z) {
                self.undo();
            }

            // ── Ctrl+Shift+Z: Redo ───────────────────────────────────
            if ctrl && shift && input.key_pressed(egui::Key::Z) {
                self.redo();
            }

            // ── Ctrl+Y: Redo (alternative) ───────────────────────────
            if ctrl && input.key_pressed(egui::Key::Y) {
                self.redo();
            }

            // ── Ctrl+1..9: Switch workspaces ─────────────────────────
            if ctrl && !shift {
                let workspace = match input.key_pressed(egui::Key::Num1) {
                    true => Some(StudioWorkspace::Home),
                    false => None,
                }
                .or_else(|| match input.key_pressed(egui::Key::Num2) {
                    true => Some(StudioWorkspace::Schematic),
                    false => None,
                })
                .or_else(|| match input.key_pressed(egui::Key::Num3) {
                    true => Some(StudioWorkspace::Library),
                    false => None,
                })
                .or_else(|| match input.key_pressed(egui::Key::Num4) {
                    true => Some(StudioWorkspace::Simulation),
                    false => None,
                })
                .or_else(|| match input.key_pressed(egui::Key::Num5) {
                    true => Some(StudioWorkspace::Optimization),
                    false => None,
                })
                .or_else(|| match input.key_pressed(egui::Key::Num6) {
                    true => Some(StudioWorkspace::Review),
                    false => None,
                })
                .or_else(|| match input.key_pressed(egui::Key::Num7) {
                    true => Some(StudioWorkspace::Waveforms),
                    false => None,
                })
                .or_else(|| match input.key_pressed(egui::Key::Num8) {
                    true => Some(StudioWorkspace::Reports),
                    false => None,
                })
                .or_else(|| match input.key_pressed(egui::Key::Num9) {
                    true => Some(StudioWorkspace::Settings),
                    false => None,
                });
                if let Some(ws) = workspace {
                    self.active_workspace = ws;
                }
            }
        });
    }
}
