use crate::json::kicad_bounding_box_value;
use crate::util::{case_insensitive_contains, kicad_wildcard_match, resolve_kicad_uri};
use crate::{
    KicadBoundingBox, KicadDiagnosticSeverity, KicadPinAlternate, KicadResolvedSymbolDef,
    KicadSymbolLibraryTable, kicad_pin_alternate_value, read_kicad_symbol_library,
    resolve_symbol_definition,
};
use osl_core::json_escape;
use std::collections::BTreeSet;
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub struct KicadSymbolLibraryIndex {
    pub source: String,
    pub libraries: Vec<KicadIndexedLibrary>,
    pub symbols: Vec<KicadIndexedSymbol>,
    pub diagnostics: Vec<KicadLibraryDiagnostic>,
}

impl KicadSymbolLibraryIndex {
    /// from table。
    pub fn from_table(table: KicadSymbolLibraryTable, base_dir: &Path) -> Self {
        let mut libraries = Vec::new();
        let mut symbols = Vec::new();
        let mut diagnostics = Vec::new();

        for row in table.libraries {
            if row.disabled {
                diagnostics.push(KicadLibraryDiagnostic {
                    library: row.name.clone(),
                    severity: KicadDiagnosticSeverity::Info,
                    message: "library row is disabled".to_string(),
                });
                continue;
            }
            if !row.library_type.eq_ignore_ascii_case("KiCad") {
                diagnostics.push(KicadLibraryDiagnostic {
                    library: row.name.clone(),
                    severity: KicadDiagnosticSeverity::Warning,
                    message: format!("unsupported symbol library type '{}'", row.library_type),
                });
                continue;
            }

            let resolved_path = resolve_kicad_uri(&row.uri, base_dir);
            match read_kicad_symbol_library(&resolved_path) {
                Ok(library) => {
                    let symbol_count = library.symbols.len();
                    for symbol in &library.symbols {
                        let resolved_symbol = resolve_symbol_definition(symbol, &library.symbols)
                            .unwrap_or_else(|| KicadResolvedSymbolDef::from_symbol(symbol));
                        symbols.push(KicadIndexedSymbol {
                            id: format!("{}:{}", row.name, symbol.local_name()),
                            library: row.name.clone(),
                            name: symbol.local_name().to_string(),
                            source: resolved_path.display().to_string(),
                            description: resolved_symbol.description().map(str::to_string),
                            keywords: resolved_symbol.keywords().map(str::to_string),
                            footprint_filters: resolved_symbol.footprint_filters(),
                            pin_count: resolved_symbol.pins.len(),
                            graphic_count: resolved_symbol.graphics.len(),
                            unit_count: resolved_symbol.unit_count(),
                            units: resolved_symbol.indexed_units(),
                            body_styles: resolved_symbol.indexed_body_styles(),
                            pins: resolved_symbol.indexed_pins(),
                            extends: symbol.extends.clone(),
                            power: symbol.power.map(|power| power.as_str().to_string()),
                            bounding_box: resolved_symbol.bounding_box(),
                        });
                    }
                    libraries.push(KicadIndexedLibrary {
                        name: row.name,
                        source: resolved_path.display().to_string(),
                        symbol_count,
                    });
                }
                Err(error) => {
                    diagnostics.push(KicadLibraryDiagnostic {
                        library: row.name,
                        severity: KicadDiagnosticSeverity::Error,
                        message: format!("failed to load {}: {}", resolved_path.display(), error),
                    });
                }
            }
        }

        Self {
            source: table.source,
            libraries,
            symbols,
            diagnostics,
        }
    }

    /// symbol。
    pub fn symbol(&self, lib_id: &str) -> Option<&KicadIndexedSymbol> {
        self.symbols.iter().find(|symbol| symbol.id == lib_id)
    }

    /// query。
    pub fn query(&self, query: &KicadSymbolLibraryIndexQuery) -> Self {
        let symbols = self
            .symbols
            .iter()
            .filter(|symbol| query.matches(symbol))
            .cloned()
            .collect::<Vec<_>>();
        let libraries_with_symbols = symbols
            .iter()
            .map(|symbol| symbol.library.as_str())
            .collect::<BTreeSet<_>>();
        let libraries = self
            .libraries
            .iter()
            .filter(|library| {
                query.matches_library_name(&library.name)
                    && libraries_with_symbols.contains(library.name.as_str())
            })
            .cloned()
            .map(|mut library| {
                library.symbol_count = symbols
                    .iter()
                    .filter(|symbol| symbol.library == library.name)
                    .count();
                library
            })
            .collect();

        Self {
            source: self.source.clone(),
            libraries,
            symbols,
            diagnostics: self.diagnostics.clone(),
        }
    }

    /// to json。
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(&self.to_json_value())
            .expect("KiCad symbol library index JSON should serialize")
    }

    fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "source": self.source,
            "library_count": self.libraries.len(),
            "symbol_count": self.symbols.len(),
            "unit_count": self.unit_count(),
            "extended_symbol_count": self.extended_symbol_count(),
            "power_symbol_count": self.power_symbol_count(),
            "described_symbol_count": self.described_symbol_count(),
            "keyword_symbol_count": self.keyword_symbol_count(),
            "footprint_filter_count": self.footprint_filter_count(),
            "diagnostic_count": self.diagnostics.len(),
            "libraries": self.libraries.iter().map(KicadIndexedLibrary::to_json_value).collect::<Vec<_>>(),
            "symbols": self.symbols.iter().map(KicadIndexedSymbol::to_json_value).collect::<Vec<_>>(),
            "diagnostics": self.diagnostics.iter().map(KicadLibraryDiagnostic::to_json_value).collect::<Vec<_>>(),
        })
    }

    /// to summary json。
    pub fn to_summary_json(&self) -> String {
        let diagnostics = self
            .diagnostics
            .iter()
            .map(|diagnostic| {
                format!(
                    "    {{ \"severity\": \"{}\", \"library\": \"{}\", \"message\": \"{}\" }}",
                    diagnostic.severity.as_str(),
                    json_escape(&diagnostic.library),
                    json_escape(&diagnostic.message)
                )
            })
            .collect::<Vec<_>>()
            .join(",\n");

        format!(
            concat!(
                "{{\n",
                "  \"source\": \"{}\",\n",
                "  \"library_count\": {},\n",
                "  \"symbol_count\": {},\n",
                "  \"unit_count\": {},\n",
                "  \"extended_symbol_count\": {},\n",
                "  \"power_symbol_count\": {},\n",
                "  \"described_symbol_count\": {},\n",
                "  \"keyword_symbol_count\": {},\n",
                "  \"footprint_filter_count\": {},\n",
                "  \"diagnostic_count\": {},\n",
                "  \"diagnostics\": [\n",
                "{}\n",
                "  ]\n",
                "}}"
            ),
            json_escape(&self.source),
            self.libraries.len(),
            self.symbols.len(),
            self.unit_count(),
            self.extended_symbol_count(),
            self.power_symbol_count(),
            self.described_symbol_count(),
            self.keyword_symbol_count(),
            self.footprint_filter_count(),
            self.diagnostics.len(),
            diagnostics
        )
    }

    fn unit_count(&self) -> usize {
        self.symbols
            .iter()
            .map(|symbol| symbol.unit_count)
            .sum::<usize>()
    }

    fn extended_symbol_count(&self) -> usize {
        self.symbols
            .iter()
            .filter(|symbol| symbol.extends.is_some())
            .count()
    }

    fn power_symbol_count(&self) -> usize {
        self.symbols
            .iter()
            .filter(|symbol| symbol.power.is_some())
            .count()
    }

    fn described_symbol_count(&self) -> usize {
        self.symbols
            .iter()
            .filter(|symbol| symbol.description.is_some())
            .count()
    }

    fn keyword_symbol_count(&self) -> usize {
        self.symbols
            .iter()
            .filter(|symbol| symbol.keywords.is_some())
            .count()
    }

    fn footprint_filter_count(&self) -> usize {
        self.symbols
            .iter()
            .map(|symbol| symbol.footprint_filters.len())
            .sum::<usize>()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct KicadSymbolLibraryIndexQuery {
    pub text: Option<String>,
    pub library: Option<String>,
    pub footprint: Option<String>,
}

impl KicadSymbolLibraryIndexQuery {
    /// is empty。
    pub fn is_empty(&self) -> bool {
        self.text.as_deref().is_none_or(str::is_empty)
            && self.library.as_deref().is_none_or(str::is_empty)
            && self.footprint.as_deref().is_none_or(str::is_empty)
    }

    fn matches(&self, symbol: &KicadIndexedSymbol) -> bool {
        self.matches_library_name(&symbol.library)
            && self.matches_text(symbol)
            && self.matches_footprint(symbol)
    }

    fn matches_library_name(&self, library: &str) -> bool {
        self.library
            .as_deref()
            .map(|filter| case_insensitive_contains(library, filter))
            .unwrap_or(true)
    }

    fn matches_text(&self, symbol: &KicadIndexedSymbol) -> bool {
        let Some(text) = self.text.as_deref().filter(|text| !text.is_empty()) else {
            return true;
        };

        case_insensitive_contains(&symbol.id, text)
            || case_insensitive_contains(&symbol.name, text)
            || symbol
                .description
                .as_deref()
                .is_some_and(|description| case_insensitive_contains(description, text))
            || symbol
                .keywords
                .as_deref()
                .is_some_and(|keywords| case_insensitive_contains(keywords, text))
    }

    fn matches_footprint(&self, symbol: &KicadIndexedSymbol) -> bool {
        let Some(footprint) = self
            .footprint
            .as_deref()
            .filter(|footprint| !footprint.is_empty())
        else {
            return true;
        };

        symbol
            .footprint_filters
            .iter()
            .any(|filter| kicad_wildcard_match(filter, footprint))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadIndexedLibrary {
    pub name: String,
    pub source: String,
    pub symbol_count: usize,
}

impl KicadIndexedLibrary {
    fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "name": self.name,
            "source": self.source,
            "symbol_count": self.symbol_count,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadIndexedSymbol {
    pub id: String,
    pub library: String,
    pub name: String,
    pub source: String,
    pub description: Option<String>,
    pub keywords: Option<String>,
    pub footprint_filters: Vec<String>,
    pub pin_count: usize,
    pub graphic_count: usize,
    pub unit_count: usize,
    pub units: Vec<KicadIndexedSymbolUnit>,
    pub body_styles: Vec<KicadIndexedSymbolBodyStyle>,
    pub pins: Vec<KicadIndexedSymbolPin>,
    pub extends: Option<String>,
    pub power: Option<String>,
    pub bounding_box: Option<KicadBoundingBox>,
}

impl KicadIndexedSymbol {
    fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "id": self.id,
            "library": self.library,
            "name": self.name,
            "source": self.source,
            "description": self.description,
            "keywords": self.keywords,
            "footprint_filters": self.footprint_filters,
            "pin_count": self.pin_count,
            "graphic_count": self.graphic_count,
            "unit_count": self.unit_count,
            "units": self.units.iter().map(KicadIndexedSymbolUnit::to_json_value).collect::<Vec<_>>(),
            "body_styles": self.body_styles.iter().map(KicadIndexedSymbolBodyStyle::to_json_value).collect::<Vec<_>>(),
            "pins": self.pins.iter().map(KicadIndexedSymbolPin::to_json_value).collect::<Vec<_>>(),
            "extends": self.extends,
            "power": self.power,
            "bounding_box": self.bounding_box.map(kicad_bounding_box_value),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KicadIndexedSymbolUnit {
    pub unit: u32,
    pub name: Option<String>,
}

impl KicadIndexedSymbolUnit {
    fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "unit": self.unit,
            "name": self.name,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KicadIndexedSymbolBodyStyle {
    pub body_style: u32,
    pub name: Option<String>,
}

impl KicadIndexedSymbolBodyStyle {
    fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "body_style": self.body_style,
            "name": self.name,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KicadIndexedSymbolPin {
    pub number: String,
    pub name: String,
    pub electrical_type: String,
    pub shape: String,
    pub unit: u32,
    pub body_style: u32,
    pub alternates: Vec<KicadPinAlternate>,
}

impl KicadIndexedSymbolPin {
    fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "number": self.number,
            "name": self.name,
            "electrical_type": self.electrical_type,
            "shape": self.shape,
            "unit": self.unit,
            "body_style": self.body_style,
            "alternate_count": self.alternates.len(),
            "alternates": self.alternates.iter().map(kicad_pin_alternate_value).collect::<Vec<_>>(),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadLibraryDiagnostic {
    pub library: String,
    pub severity: KicadDiagnosticSeverity,
    pub message: String,
}

impl KicadLibraryDiagnostic {
    fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "severity": self.severity.as_str(),
            "library": self.library,
            "message": self.message,
        })
    }
}
