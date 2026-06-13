//! Clipboard operations for the schematic editor.
//!
//! Provides Cut, Copy, and Paste functionality. Items are serialized
//! as structured text for the system clipboard via egui's API.

use crate::app::NekoSpiceApp;
use eframe::egui;

/// Internal clipboard buffer storing the last cut/copied item.
#[derive(Debug, Clone)]
pub(crate) struct ClipboardBuffer {
    /// The kind of item (e.g., "Symbol", "Wire", "Label").
    pub(crate) item_kind: String,
    /// The label/reference of the item.
    pub(crate) item_label: String,
    /// The UUID of the source item.
    pub(crate) item_uuid: Option<String>,
    /// Whether this was a cut (for future paste-delete support).
    pub(crate) is_cut: bool,
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
    pub(super) fn paste_from_clipboard(&mut self) {
        let Some(buffer) = &self.clipboard_buffer.clone() else {
            self.status_message = Some("Nothing in clipboard to paste".to_string());
            return;
        };

        if buffer.is_cut {
            // After a cut, the original was deleted — re-paste guidance
            self.status_message = Some(format!(
                "Paste: use Library workspace to place '{}' from library",
                buffer.item_label,
            ));
        } else {
            self.status_message = Some(format!(
                "Paste: use Library workspace to place '{}' from library",
                buffer.item_label,
            ));
        }
    }
}
