//! 测试辅助工具。
//!
use crate::DEFAULT_SCHEMATIC;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// workspace root。
pub(crate) fn workspace_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
}

/// `TempSchematic` 类型定义。
pub(crate) struct TempSchematic {
    path: PathBuf,
}

impl TempSchematic {
    /// path。
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempSchematic {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

/// temp schematic copy。
pub(crate) fn temp_schematic_copy(prefix: &str) -> TempSchematic {
    let source = workspace_root().join(DEFAULT_SCHEMATIC);
    let temp_path = std::env::temp_dir().join(format!(
        "nekospice_{prefix}_{}.kicad_sch",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::copy(&source, &temp_path).unwrap();
    TempSchematic { path: temp_path }
}
