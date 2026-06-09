use osl_kicad::{
    KicadCanvasScene, KicadEditSummary, KicadPoint, KicadSchematic, KicadSchematicEdit,
    read_kicad_schematic_with_libraries, write_kicad_schematic,
};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub(crate) struct KicadGuiDocument {
    path: PathBuf,
    schematic: KicadSchematic,
    dirty: bool,
}

impl KicadGuiDocument {
    pub(crate) fn load(path: PathBuf) -> Result<Self, String> {
        read_kicad_schematic_with_libraries(&path)
            .map(|schematic| Self {
                path,
                schematic,
                dirty: false,
            })
            .map_err(|error| error.to_string())
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub(crate) fn scene(&self) -> KicadCanvasScene {
        self.schematic.canvas_scene()
    }

    pub(crate) fn delete_item(&mut self, uuid: &str) -> Result<KicadEditSummary, String> {
        self.schematic
            .apply_edit(KicadSchematicEdit::DeleteItem {
                uuid: uuid.to_string(),
            })
            .inspect(|_| {
                self.dirty = true;
            })
            .map_err(|error| error.to_string())
    }

    pub(crate) fn move_item(
        &mut self,
        uuid: &str,
        delta: KicadPoint,
    ) -> Result<KicadEditSummary, String> {
        self.schematic
            .apply_edit(KicadSchematicEdit::MoveItem {
                uuid: uuid.to_string(),
                delta,
            })
            .inspect(|_| {
                self.dirty = true;
            })
            .map_err(|error| error.to_string())
    }

    pub(crate) fn save(&mut self) -> Result<(), String> {
        write_kicad_schematic(&self.path, &self.schematic)
            .inspect(|_| {
                self.dirty = false;
            })
            .map_err(|error| error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DEFAULT_SCHEMATIC;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn document_deletes_selected_uuid_and_saves_schematic() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let source = workspace_root.join(DEFAULT_SCHEMATIC);
        let temp_path = std::env::temp_dir().join(format!(
            "nekospice_gui_delete_{}.kicad_sch",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::copy(&source, &temp_path).unwrap();

        let mut document = KicadGuiDocument::load(temp_path.clone()).unwrap();
        assert!(!document.is_dirty());
        assert_eq!(document.scene().wires.len(), 3);

        let summary = document
            .delete_item("22222222-2222-2222-2222-222222222222")
            .unwrap();
        assert_eq!(summary.operation, "delete-wire");
        assert!(document.is_dirty());
        assert_eq!(document.scene().wires.len(), 2);

        document.save().unwrap();
        assert!(!document.is_dirty());
        let saved = fs::read_to_string(&temp_path).unwrap();
        assert!(!saved.contains("22222222-2222-2222-2222-222222222222"));

        fs::remove_file(temp_path).unwrap();
    }

    #[test]
    fn document_moves_selected_uuid_and_keeps_canvas_hit_addressable() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let source = workspace_root.join(DEFAULT_SCHEMATIC);
        let temp_path = std::env::temp_dir().join(format!(
            "nekospice_gui_move_{}.kicad_sch",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::copy(&source, &temp_path).unwrap();

        let mut document = KicadGuiDocument::load(temp_path.clone()).unwrap();
        let original_hit = document
            .scene()
            .item_hit_by_uuid("22222222-2222-2222-2222-222222222222")
            .unwrap();

        let summary = document
            .move_item(
                "22222222-2222-2222-2222-222222222222",
                KicadPoint { x: 2.54, y: 0.0 },
            )
            .unwrap();
        assert_eq!(summary.operation, "move-wire");
        assert!(document.is_dirty());

        let moved_hit = document
            .scene()
            .item_hit_by_uuid("22222222-2222-2222-2222-222222222222")
            .unwrap();
        assert!((moved_hit.bounds.min.x - original_hit.bounds.min.x - 2.54).abs() < 1e-6);
        assert_eq!(moved_hit.kind, "wire");

        document.save().unwrap();
        assert!(!document.is_dirty());
        let reloaded_scene = read_kicad_schematic_with_libraries(&temp_path)
            .unwrap()
            .canvas_scene();
        let saved_hit = reloaded_scene
            .item_hit_by_uuid("22222222-2222-2222-2222-222222222222")
            .unwrap();
        assert_eq!(saved_hit.kind, "wire");
        assert!((saved_hit.bounds.min.x - original_hit.bounds.min.x - 2.54).abs() < 1e-6);
        assert!((saved_hit.bounds.min.y - original_hit.bounds.min.y).abs() < 1e-6);

        fs::remove_file(temp_path).unwrap();
    }
}
