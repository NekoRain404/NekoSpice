/// Right-click context menu for the schematic canvas.
///
/// Provides actions relevant to the clicked position or selected item,
/// including cut/copy/paste, delete, rotate, and tool switching.

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

}

impl NekoSpiceApp {
    /// Draw the right-click context menu at the pointer position.
    ///
    /// Returns the action the user selected, or `ContextMenuAction::None`.
    pub(crate) fn draw_canvas_context_menu(
        &mut self,
        ui: &mut egui::Ui,
    ) -> ContextMenuAction {
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
            (super::schematic_tools::SchematicTool::Select, "Select", "V"),
            (super::schematic_tools::SchematicTool::Wire, "Wire", "W"),
            (super::schematic_tools::SchematicTool::Bus, "Bus", "B"),
            (super::schematic_tools::SchematicTool::Label, "Net Label", "L"),
            (super::schematic_tools::SchematicTool::NoConnect, "No Connect", "Q"),
            (super::schematic_tools::SchematicTool::Junction, "Junction", "J"),
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
            ).on_hover_text(format!("{shortcut}"));
            if resp.clicked() {
                self.activate_schematic_tool_direct(tool);
            }
        }

        ui.separator();

        // View options
        context_menu_item(ui, mode, "Fit to Screen", "F", true, |_a| {
            // Will be handled by caller
        }, &mut action);
        context_menu_item(ui, mode, "Zoom In", "+", true, |_a| {}, &mut action);
        context_menu_item(ui, mode, "Zoom Out", "-", true, |_a| {}, &mut action);

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
        ui.horizontal(|ui| {
            ui.label(RichText::new(label).size(13.0).color(palette.text));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(shortcut).size(11.0).color(palette.text_muted));
            });
        });
        // Make the entire row clickable
        let response = ui.interact(
            ui.clip_rect(),
            egui::Id::new(("ctx", label)),
            egui::Sense::click(),
        );
        if response.clicked() {
            on_click(action);
        }
    });
}
