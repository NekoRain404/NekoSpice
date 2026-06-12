//! 画布右键上下文菜单。根据选中项类型动态构建上下文操作。
//!
use super::NekoSpiceApp;
use super::theme::{StudioTheme, StudioThemeMode};
use eframe::egui::{self, Color32, RichText, Stroke};

/// Context menu action result.
pub(crate) enum ContextMenuAction {
    /// No action (menu was not opened or was dismissed).
    None,
    /// Delete the currently selected item.
    DeleteSelected,
    /// Cut the selected item to clipboard.
    CutSelected,
    /// Copy the selected item to clipboard.
    CopySelected,
    /// Paste from clipboard at cursor position.
    PasteAtCursor,
    /// Rotate the selected item 90 degrees clockwise.
    RotateSelected,
    /// Fit the canvas to show the entire schematic.
    FitToScreen,
    /// Zoom in by one step.
    ZoomIn,
    /// Zoom out by one step.
    ZoomOut,
}

impl NekoSpiceApp {
    /// Handle right-click context menu on the canvas.
    ///
    /// Shows the context menu only when the user right-clicks without dragging
    /// (i.e., a quick tap). If the user is dragging with the right mouse button,
    /// we skip the context menu so panning works smoothly.
    pub(super) fn handle_canvas_context_menu_with_pan(
        &mut self,
        ui: &mut egui::Ui,
        rect: egui::Rect,
        was_right_dragging: bool,
    ) {
        if was_right_dragging {
            return;
        }

        let response = ui.interact(rect, egui::Id::new("canvas_context"), egui::Sense::click());

        if response.secondary_clicked() {
            let action = self.draw_canvas_context_menu(ui);
            match action {
                ContextMenuAction::DeleteSelected => self.delete_selected(),
                ContextMenuAction::RotateSelected => {
                    self.rotate_selected();
                }
                ContextMenuAction::CutSelected => {
                    self.status_message = Some("Cut (clipboard not yet supported)".to_string());
                }
                ContextMenuAction::CopySelected => {
                    self.status_message = Some("Copy (clipboard not yet supported)".to_string());
                }
                ContextMenuAction::PasteAtCursor => {
                    self.status_message = Some("Paste (clipboard not yet supported)".to_string());
                }
                ContextMenuAction::FitToScreen => {
                    self.viewport
                        .fit_scene(self.scene.as_ref().and_then(|scene| scene.bounds));
                }
                ContextMenuAction::ZoomIn => {
                    // Zoom around viewport center for predictable behavior
                    if let Some(rect) = self.last_canvas_rect {
                        let center = rect.center();
                        self.viewport.zoom_around(rect, center, 1.25);
                    }
                }
                ContextMenuAction::ZoomOut => {
                    if let Some(rect) = self.last_canvas_rect {
                        let center = rect.center();
                        self.viewport.zoom_around(rect, center, 1.0 / 1.25);
                    }
                }
                ContextMenuAction::None => {}
            }
        }
    }

    /// Draw the right-click context menu at the pointer position.
    ///
    /// Returns the action the user selected, or `ContextMenuAction::None`.
    fn draw_canvas_context_menu(&mut self, ui: &mut egui::Ui) -> ContextMenuAction {
        let mut action = ContextMenuAction::None;
        let mode = self.theme_mode();
        let palette = self.theme_palette();
        let has_document = self.document.is_some();
        let has_selection = self.selected_hit.is_some();

        ui.set_min_width(180.0);

        // Clipboard operations
        if has_selection {
            context_menu_item(ui, mode, "Cut", "Ctrl+X", true, |a| *a = ContextMenuAction::CutSelected, &mut action);
            context_menu_item(ui, mode, "Copy", "Ctrl+C", true, |a| *a = ContextMenuAction::CopySelected, &mut action);
        }
        context_menu_item(ui, mode, "Paste", "Ctrl+V", has_document, |a| *a = ContextMenuAction::PasteAtCursor, &mut action);

        if has_selection {
            ui.separator();
            context_menu_item(ui, mode, "Delete", "Del", true, |a| *a = ContextMenuAction::DeleteSelected, &mut action);
            context_menu_item(ui, mode, "Rotate 90\u{00B0}", "R", true, |a| *a = ContextMenuAction::RotateSelected, &mut action);
        }

        ui.separator();

        // Tool switching
        ui.label(StudioTheme::muted_for(mode, "Tools"));
        let tools = [
            (super::schematic::tools::SchematicTool::Select, "Select", "V"),
            (super::schematic::tools::SchematicTool::Wire, "Wire", "W"),
            (super::schematic::tools::SchematicTool::Bus, "Bus", "B"),
            (super::schematic::tools::SchematicTool::Label, "Net Label", "L"),
            (super::schematic::tools::SchematicTool::NoConnect, "No Connect", "Q"),
            (super::schematic::tools::SchematicTool::Junction, "Junction", "J"),
        ];

        for (tool, label, shortcut) in tools {
            let selected = self.schematic_tools.active == tool;
            let text = if selected {
                RichText::new(format!("\u{2713} {label}")).color(palette.accent).strong()
            } else {
                RichText::new(format!("   {label}")).color(palette.text)
            };
            let resp = ui.add_sized(
                [ui.available_width(), 22.0],
                egui::Button::new(text).fill(Color32::TRANSPARENT).stroke(Stroke::NONE),
            ).on_hover_text(shortcut);
            if resp.clicked() {
                self.activate_schematic_tool_direct(tool);
            }
        }

        ui.separator();

        // View options
        context_menu_item(ui, mode, "Fit to Screen", "F", true, |a| *a = ContextMenuAction::FitToScreen, &mut action);
        context_menu_item(ui, mode, "Zoom In", "+", true, |a| *a = ContextMenuAction::ZoomIn, &mut action);
        context_menu_item(ui, mode, "Zoom Out", "-", true, |a| *a = ContextMenuAction::ZoomOut, &mut action);

        action
    }
}

/// Helper to draw a context menu item with label and shortcut.
fn context_menu_item(
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
    label: &str,
    shortcut: &str,
    enabled: bool,
    on_click: impl FnOnce(&mut ContextMenuAction),
    action: &mut ContextMenuAction,
) {
    let palette = StudioTheme::palette(mode);
    ui.add_enabled_ui(enabled, |ui| {
        let (rect, response) = ui.allocate_exact_size(
            egui::Vec2::new(ui.available_width(), 24.0),
            egui::Sense::click(),
        );
        if response.hovered() {
            let painter = ui.painter();
            painter.rect_filled(
                rect,
                egui::CornerRadius::same(4),
                palette.panel_hover,
            );
        }
        let painter = ui.painter();
        painter.text(
            rect.left_center() + egui::Vec2::new(8.0, 0.0),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::proportional(13.0),
            palette.text,
        );
        painter.text(
            rect.right_center() + egui::Vec2::new(-8.0, 0.0),
            egui::Align2::RIGHT_CENTER,
            shortcut,
            egui::FontId::proportional(11.0),
            palette.text_muted,
        );
        if response.clicked() {
            on_click(action);
        }
    });
}
