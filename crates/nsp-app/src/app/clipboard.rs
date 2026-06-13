//! Clipboard operations for the schematic editor.
//!
//! Provides Cut, Copy, and Paste functionality. Items are serialized
//! as structured text for the system clipboard via egui's API.

use crate::app::NekoSpiceApp;
use eframe::egui;

/// Internal clipboard buffer storing the last cut/copied item.
///
/// Tracks the item kind, label, UUID, and whether it was cut or copied.
/// Used by paste to provide placement guidance and by the status bar
/// to display clipboard contents.
#[derive(Debug, Clone)]
pub(crate) struct ClipboardBuffer {
    /// The kind of item (e.g., "Symbol", "Wire", "Label").
    pub(crate) item_kind: String,
    /// The label/reference of the item.
    pub(crate) item_label: String,
    /// The UUID of the source item (used for duplication tracking).
    pub(crate) item_uuid: Option<String>,
    /// Whether this was a cut (for future paste-delete support).
    pub(crate) is_cut: bool,
}

impl ClipboardBuffer {
    /// Returns a short description for status bar display.
    fn description(&self) -> String {
        let uuid_info = self
            .item_uuid
            .as_ref()
            .map(|u| format!(" [{}]", &u[..8.min(u.len())]))
            .unwrap_or_default();
        format!("{} '{}'{}", self.item_kind, self.item_label, uuid_info)
    }
}

impl NekoSpiceApp {
    /// Copy the selected item's info to the system clipboard.
    pub(super) fn copy_selected_to_clipboard(&mut self, ctx: &egui::Context) {
        let Some(hit) = &self.selected_hit else {
            self.status_message = Some("No item selected to copy".to_string());
            return;
        };

        let uuid = hit.uuid.clone().unwrap_or_default();
        let text = format!(
            "NekoSpice\nKind: {}\nLabel: {}\nUUID: {}",
            hit.kind, hit.label, uuid,
        );

        ctx.copy_text(text);

        self.clipboard_buffer = Some(ClipboardBuffer {
            item_kind: hit.kind.clone(),
            item_label: hit.label.clone(),
            item_uuid: hit.uuid.clone(),
            is_cut: false,
        });

        self.status_message = Some(format!("Copied: {} ({})", hit.kind, hit.label));
    }

    /// Cut the selected item: copy to clipboard then delete.
    pub(super) fn cut_selected_to_clipboard(&mut self, ctx: &egui::Context) {
        let hit_kind = self.selected_hit.as_ref().map(|h| h.kind.clone());
        let hit_label = self.selected_hit.as_ref().map(|h| h.label.clone());

        self.copy_selected_to_clipboard(ctx);

        if let Some(buffer) = &mut self.clipboard_buffer {
            buffer.is_cut = true;
        }

        self.delete_selected();

        if let (Some(kind), Some(label)) = (hit_kind, hit_label) {
            self.status_message = Some(format!("Cut: {} ({})", kind, label));
        }
    }

    /// Paste: provide guidance for placing the copied item.
    ///
    /// For cut items, suggests re-placement from the original UUID context.
    /// For copied items, guides the user to place a duplicate from the library.
    pub(super) fn paste_from_clipboard(&mut self) {
        let Some(buffer) = &self.clipboard_buffer.clone() else {
            self.status_message = Some("Nothing in clipboard to paste".to_string());
            return;
        };

        if buffer.is_cut {
            // After a cut, the original was deleted — guide re-placement with UUID context
            self.status_message = Some(format!(
                "Paste: re-place {} from library",
                buffer.description(),
            ));
        } else {
            // Copy — guide placing a duplicate
            self.status_message = Some(format!(
                "Paste: duplicate {} from library",
                buffer.description(),
            ));
        }
    }
}
