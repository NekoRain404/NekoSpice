use osl_kicad::{
    KicadCanvasScene, KicadIndexedSymbol, KicadSymbolDef, KicadSymbolLibraryIndex,
    KicadSymbolLibraryIndexQuery, read_kicad_symbol_library, read_kicad_symbol_library_index,
};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub(crate) struct KicadGuiLibrary {
    path: PathBuf,
    index: KicadSymbolLibraryIndex,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct KicadGuiSymbolDefinition {
    pub(crate) id: String,
    pub(crate) definition: KicadSymbolDef,
    pub(crate) library_symbols: Vec<KicadSymbolDef>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct KicadGuiSymbolPreview {
    pub(crate) id: String,
    pub(crate) scene: KicadCanvasScene,
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

    pub(crate) fn symbol_definition(
        &self,
        lib_id: &str,
    ) -> Result<KicadGuiSymbolDefinition, String> {
        let symbol = self
            .symbol(lib_id)
            .ok_or_else(|| format!("KiCad symbol '{lib_id}' is not loaded in the library index"))?;
        let library = read_kicad_symbol_library(Path::new(&symbol.source))
            .map_err(|error| error.to_string())?;
        let definition = library
            .symbol(&symbol.id)
            .or_else(|| library.symbol_by_name_or_local_name(&symbol.name))
            .cloned()
            .ok_or_else(|| {
                format!(
                    "KiCad symbol definition '{}' was not found in {}",
                    symbol.id, symbol.source
                )
            })?;
        Ok(KicadGuiSymbolDefinition {
            id: symbol.id.clone(),
            definition,
            library_symbols: library.symbols,
        })
    }

    pub(crate) fn symbol_preview(&self, lib_id: &str) -> Result<KicadGuiSymbolPreview, String> {
        let symbol = self.symbol_definition(lib_id)?;
        let scene = KicadCanvasScene::from_symbol_definition(
            symbol.id.clone(),
            &symbol.definition,
            &symbol.library_symbols,
            Some(1),
            None,
        );
        Ok(KicadGuiSymbolPreview {
            id: symbol.id,
            scene,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DEFAULT_SYMBOL_LIBRARY_TABLE;

    #[test]
    fn loads_symbol_library_index_for_gui_browser() {
        let library = KicadGuiLibrary::load(
            crate::test_support::workspace_root().join(DEFAULT_SYMBOL_LIBRARY_TABLE),
        )
        .unwrap();

        assert_eq!(library.index().libraries.len(), 1);
        assert_eq!(library.index().symbols.len(), 3);
        assert!(library.symbol("NekoSpice:R").is_some());

        let filtered = library.filtered_index("NekoSpice:C");
        assert_eq!(filtered.symbols.len(), 1);
        assert_eq!(filtered.symbols[0].id, "NekoSpice:C");
    }

    #[test]
    fn loads_symbol_definition_for_gui_placement() {
        let library = KicadGuiLibrary::load(
            crate::test_support::workspace_root().join(DEFAULT_SYMBOL_LIBRARY_TABLE),
        )
        .unwrap();

        let symbol = library.symbol_definition("NekoSpice:R").unwrap();

        assert_eq!(symbol.id, "NekoSpice:R");
        assert_eq!(symbol.definition.name, "NekoSpice:R");
        assert_eq!(symbol.library_symbols.len(), 3);
        assert_eq!(symbol.definition.property("Reference"), Some("R"));
    }

    #[test]
    fn builds_symbol_preview_scene_for_gui_browser() {
        let library = KicadGuiLibrary::load(
            crate::test_support::workspace_root().join(DEFAULT_SYMBOL_LIBRARY_TABLE),
        )
        .unwrap();

        let preview = library.symbol_preview("NekoSpice:R").unwrap();

        assert_eq!(preview.id, "NekoSpice:R");
        assert_eq!(preview.scene.symbols.len(), 1);
        assert_eq!(preview.scene.symbols[0].lib_id, "NekoSpice:R");
        assert_eq!(preview.scene.symbols[0].pins.len(), 2);
        assert!(preview.scene.bounds.is_some());
    }
}
