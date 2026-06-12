/// File open/save dialog integration.
///
/// Uses the `rfd` crate for native file dialogs on Linux/macOS/Windows.
/// Provides async-safe file picker calls that work with egui's event loop.
use super::NekoSpiceApp;
use super::localization::UiText;
use std::path::PathBuf;

impl NekoSpiceApp {
    /// Open a native file dialog to pick a KiCad schematic file (.kicad_sch).
    ///
    /// Uses `rfd::FileDialog` which blocks the current thread.
    /// Since egui runs single-threaded, this is acceptable for now.
    pub(super) fn open_file_dialog(&mut self) {
        let dialog = rfd::FileDialog::new()
            .add_filter("KiCad Schematic", &["kicad_sch"])
            .add_filter("KiCad Symbol Library", &["kicad_sym"])
            .add_filter("All Files", &["*"]);

        if let Some(path) = dialog.pick_file() {
            self.load_schematic(path);
        }
    }

    /// Open a native file dialog to pick a symbol library table (.kicad_sym or sym-lib-table).
    pub(super) fn open_library_dialog(&mut self) {
        let dialog = rfd::FileDialog::new()
            .add_filter("KiCad Symbol Library", &["kicad_sym", "sym-lib-table"])
            .add_filter("All Files", &["*"]);

        if let Some(path) = dialog.pick_file() {
            self.load_symbol_library(path);
        }
    }

    /// Save the current document. If untitled, prompt for a save location.
    pub(super) fn save_document_with_dialog(&mut self) {
        let Some(document) = &self.document else {
            self.status_message = Some(self.text(UiText::NoDocument).to_string());
            return;
        };

        if document.is_dirty() {
            // Document has been modified, save to current path
            self.save_document();
        } else {
            // Prompt for save location
            let dialog = rfd::FileDialog::new()
                .add_filter("KiCad Schematic", &["kicad_sch"])
                .set_file_name("untitled.kicad_sch");

            if let Some(path) = dialog.save_file() {
                // Re-save to the new path
                self.save_document_to_path(path);
            }
        }
    }

    /// Save the current document to a specific path.
    fn save_document_to_path(&mut self, path: PathBuf) {
        let Some(document) = &mut self.document else {
            self.status_message = Some(self.text(UiText::NoDocument).to_string());
            return;
        };

        match document.save_as(&path) {
            Ok(()) => {
                self.status_message = Some(format!("Saved to {}", path.display()));
                self.load_error = None;
            }
            Err(error) => {
                self.status_message = Some(format!("Save failed: {error}"));
            }
        }
    }
}
