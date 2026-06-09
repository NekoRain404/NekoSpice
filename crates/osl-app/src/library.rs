use osl_kicad::{
    KicadIndexedSymbol, KicadSymbolLibraryIndex, KicadSymbolLibraryIndexQuery,
    read_kicad_symbol_library_index,
};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub(crate) struct KicadGuiLibrary {
    path: PathBuf,
    index: KicadSymbolLibraryIndex,
}

impl KicadGuiLibrary {
    pub(crate) fn load(path: PathBuf) -> Result<Self, String> {
        read_kicad_symbol_library_index(&path)
            .map(|index| Self { path, index })
            .map_err(|error| error.to_string())
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn index(&self) -> &KicadSymbolLibraryIndex {
        &self.index
    }

    pub(crate) fn filtered_index(&self, text: &str) -> KicadSymbolLibraryIndex {
        let text = text.trim();
        let query = KicadSymbolLibraryIndexQuery {
            text: (!text.is_empty()).then(|| text.to_string()),
            library: None,
            footprint: None,
        };
        if query.is_empty() {
            self.index.clone()
        } else {
            self.index.query(&query)
        }
    }

    pub(crate) fn symbol(&self, lib_id: &str) -> Option<&KicadIndexedSymbol> {
        self.index.symbol(lib_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DEFAULT_SYMBOL_LIBRARY_TABLE;

    #[test]
    fn loads_symbol_library_index_for_gui_browser() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let library =
            KicadGuiLibrary::load(workspace_root.join(DEFAULT_SYMBOL_LIBRARY_TABLE)).unwrap();

        assert_eq!(library.index().libraries.len(), 1);
        assert_eq!(library.index().symbols.len(), 3);
        assert!(library.symbol("NekoSpice:R").is_some());

        let filtered = library.filtered_index("NekoSpice:C");
        assert_eq!(filtered.symbols.len(), 1);
        assert_eq!(filtered.symbols[0].id, "NekoSpice:C");
    }
}
