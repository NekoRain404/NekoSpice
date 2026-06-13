//! 文件对话框集成。封装原生文件打开/保存对话框。
//!
//! 支持 NekoSpice 原生格式 (`.nsp_sch` / `.nsp_sym`) 和
//! 支持 Legacy EDA 格式 (`.kicad_sch` / `.kicad_sym`) 兼容读写。

use super::NekoSpiceApp;
use super::localization::UiText;
use std::path::PathBuf;

/// 支持的原理图文件扩展名（含 Legacy EDA 兼容格式）。
const SCH_FILTER: &[&str] = &["nsp_sch", "kicad_sch"];
/// 支持的符号库文件扩展名（含 Legacy EDA 兼容格式）。
const SYM_FILTER: &[&str] = &["nsp_sym", "kicad_sym", "sym-lib-table"];

impl NekoSpiceApp {
    /// 打开文件对话框，选择原理图文件。
    ///
    /// 支持 `.nsp_sch` 和 `.kicad_sch`（Legacy EDA）格式。
    /// 使用 `rfd::FileDialog` 阻塞当前线程。
    pub(super) fn open_file_dialog(&mut self) {
        let dialog = rfd::FileDialog::new()
            .add_filter("NekoSpice Schematic", SCH_FILTER)
            .add_filter("Symbol Library", SYM_FILTER)
            .add_filter("All Files", &["*"]);

        if let Some(path) = dialog.pick_file() {
            self.load_schematic(path);
        }
    }

    /// 打开文件对话框，选择符号库表文件。
    #[allow(dead_code)]
    pub(super) fn open_library_dialog(&mut self) {
        let dialog = rfd::FileDialog::new()
            .add_filter("Symbol Library", SYM_FILTER)
            .add_filter("All Files", &["*"]);

        if let Some(path) = dialog.pick_file() {
            self.load_symbol_library(path);
        }
    }

    /// 保存当前文档。若未命名则弹出保存对话框。
    pub(super) fn save_document_with_dialog(&mut self) {
        let Some(document) = &self.document else {
            self.status_message = Some(self.text(UiText::NoDocument).to_string());
            return;
        };

        if document.is_dirty() {
            self.save_document();
        } else {
            let dialog = rfd::FileDialog::new()
                .add_filter("NekoSpice Schematic", SCH_FILTER)
                .set_file_name("untitled.nsp_sch");

            if let Some(path) = dialog.save_file() {
                self.save_document_to_path(path);
            }
        }
    }

    /// 保存当前文档到指定路径。
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
