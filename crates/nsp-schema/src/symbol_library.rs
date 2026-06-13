//! schema symbol library table (.nsp_sym) parsing.

use crate::json::json_option;
use crate::sexpr::{
    Sexp, child, child_value, direct_children, expect_root_list, expect_root_list_any, list_items,
    sexpr_atom_or_string, sexpr_string,
};
use crate::symbols::parse_symbol_def;
use crate::{NspGraphic, NspSymbolDef, parse_sexpr};
use nsp_core::{OslResult, json_escape};

#[derive(Debug, Clone, PartialEq)]
pub struct NspSymbolLibrary {
    pub source: String,
    pub version: Option<String>,
    pub generator: Option<String>,
    pub generator_version: Option<String>,
    pub symbols: Vec<NspSymbolDef>,
}

impl NspSymbolLibrary {
    /// symbol。
    pub fn symbol(&self, name: &str) -> Option<&NspSymbolDef> {
        self.symbols.iter().find(|symbol| symbol.name == name)
    }

    /// symbol by name or local name。
    pub fn symbol_by_name_or_local_name(&self, name: &str) -> Option<&NspSymbolDef> {
        self.symbols
            .iter()
            .find(|symbol| symbol.name == name || symbol.local_name() == name)
    }

    /// to schema symbol library sexpr。
    pub fn to_symbol_library_sexpr(&self) -> String {
        let mut output = String::new();
        output.push_str("(nsp_symbol_lib\n");
        if let Some(version) = &self.version {
            output.push_str(&format!("  (version {})\n", sexpr_atom_or_string(version)));
        }
        if let Some(generator) = &self.generator {
            output.push_str(&format!("  (generator {})\n", sexpr_string(generator)));
        }
        if let Some(generator_version) = &self.generator_version {
            output.push_str(&format!(
                "  (generator_version {})\n",
                sexpr_string(generator_version)
            ));
        }
        for symbol in &self.symbols {
            symbol.write_symbol_sexpr(&mut output, 2);
        }
        output.push_str(")\n");
        output
    }

    /// to summary json。
    pub fn to_summary_json(&self) -> String {
        let pin_count = self
            .symbols
            .iter()
            .map(|symbol| symbol.pins.len())
            .sum::<usize>();
        let pin_display_setting_count = self
            .symbols
            .iter()
            .map(|symbol| {
                usize::from(symbol.pin_names.is_some()) + usize::from(symbol.pin_numbers.is_some())
            })
            .sum::<usize>();
        let unit_name_count = self
            .symbols
            .iter()
            .map(|symbol| symbol.unit_names.len())
            .sum::<usize>();
        let pin_text_effect_count = self
            .symbols
            .iter()
            .flat_map(|symbol| &symbol.pins)
            .map(|pin| {
                usize::from(pin.name_effects().is_some())
                    + usize::from(pin.number_effects().is_some())
            })
            .sum::<usize>();
        let pin_alternate_count = self
            .symbols
            .iter()
            .flat_map(|symbol| &symbol.pins)
            .map(|pin| pin.alternates.len())
            .sum::<usize>();
        let power_symbol_count = self
            .symbols
            .iter()
            .filter(|symbol| symbol.power.is_some())
            .count();
        let symbol_in_bom_setting_count = self
            .symbols
            .iter()
            .filter(|symbol| symbol.in_bom.is_some())
            .count();
        let symbol_on_board_setting_count = self
            .symbols
            .iter()
            .filter(|symbol| symbol.on_board.is_some())
            .count();
        let symbol_in_pos_files_setting_count = self
            .symbols
            .iter()
            .filter(|symbol| symbol.in_pos_files.is_some())
            .count();
        let duplicate_pin_numbers_are_jumpers_count = self
            .symbols
            .iter()
            .filter(|symbol| symbol.duplicate_pin_numbers_are_jumpers == Some(true))
            .count();
        let extended_symbol_count = self
            .symbols
            .iter()
            .filter(|symbol| symbol.extends.is_some())
            .count();
        let described_symbol_count = self
            .symbols
            .iter()
            .filter(|symbol| symbol.description().is_some())
            .count();
        let keyword_symbol_count = self
            .symbols
            .iter()
            .filter(|symbol| symbol.keywords().is_some())
            .count();
        let footprint_filter_count = self
            .symbols
            .iter()
            .map(|symbol| symbol.footprint_filters().len())
            .sum::<usize>();
        let body_style_symbol_count = self
            .symbols
            .iter()
            .filter(|symbol| symbol.body_styles.is_some())
            .count();
        let jumper_pin_group_count = self
            .symbols
            .iter()
            .map(|symbol| symbol.jumper_pin_groups.len())
            .sum::<usize>();
        let embedded_font_symbol_count = self
            .symbols
            .iter()
            .filter(|symbol| symbol.embedded_fonts.is_some())
            .count();
        let symbol_graphic_text_effect_count = self
            .symbols
            .iter()
            .flat_map(|symbol| &symbol.graphics)
            .filter(|graphic| {
                matches!(
                    &graphic.graphic,
                    NspGraphic::Text {
                        effects: Some(_),
                        ..
                    }
                )
            })
            .count();
        let unit_scoped_item_count = self
            .symbols
            .iter()
            .map(|symbol| {
                symbol
                    .graphics
                    .iter()
                    .filter(|graphic| graphic.unit != 0)
                    .count()
                    + symbol.pins.iter().filter(|pin| pin.unit != 0).count()
            })
            .sum::<usize>();
        let body_style_scoped_item_count = self
            .symbols
            .iter()
            .map(|symbol| {
                symbol
                    .graphics
                    .iter()
                    .filter(|graphic| graphic.body_style != 0)
                    .count()
                    + symbol.pins.iter().filter(|pin| pin.body_style != 0).count()
            })
            .sum::<usize>();

        format!(
            concat!(
                "{{\n",
                "  \"source\": \"{}\",\n",
                "  \"version\": {},\n",
                "  \"generator\": {},\n",
                "  \"generator_version\": {},\n",
                "  \"symbol_count\": {},\n",
                "  \"graphic_count\": {},\n",
                "  \"symbol_graphic_text_effect_count\": {},\n",
                "  \"unit_scoped_item_count\": {},\n",
                "  \"body_style_scoped_item_count\": {},\n",
                "  \"pin_count\": {},\n",
                "  \"pin_display_setting_count\": {},\n",
                "  \"unit_name_count\": {},\n",
                "  \"pin_text_effect_count\": {},\n",
                "  \"pin_alternate_count\": {},\n",
                "  \"power_symbol_count\": {},\n",
                "  \"symbol_in_bom_setting_count\": {},\n",
                "  \"symbol_on_board_setting_count\": {},\n",
                "  \"symbol_in_pos_files_setting_count\": {},\n",
                "  \"duplicate_pin_numbers_are_jumpers_count\": {},\n",
                "  \"extended_symbol_count\": {},\n",
                "  \"described_symbol_count\": {},\n",
                "  \"keyword_symbol_count\": {},\n",
                "  \"footprint_filter_count\": {},\n",
                "  \"body_style_symbol_count\": {},\n",
                "  \"jumper_pin_group_count\": {},\n",
                "  \"embedded_font_symbol_count\": {}\n",
                "}}"
            ),
            json_escape(&self.source),
            json_option(self.version.as_deref()),
            json_option(self.generator.as_deref()),
            json_option(self.generator_version.as_deref()),
            self.symbols.len(),
            self.symbols
                .iter()
                .map(|symbol| symbol.graphics.len())
                .sum::<usize>(),
            symbol_graphic_text_effect_count,
            unit_scoped_item_count,
            body_style_scoped_item_count,
            pin_count,
            pin_display_setting_count,
            unit_name_count,
            pin_text_effect_count,
            pin_alternate_count,
            power_symbol_count,
            symbol_in_bom_setting_count,
            symbol_on_board_setting_count,
            symbol_in_pos_files_setting_count,
            duplicate_pin_numbers_are_jumpers_count,
            extended_symbol_count,
            described_symbol_count,
            keyword_symbol_count,
            footprint_filter_count,
            body_style_symbol_count,
            jumper_pin_group_count,
            embedded_font_symbol_count
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NspSymbolLibraryTable {
    pub source: String,
    pub version: Option<String>,
    pub libraries: Vec<NspSymbolLibraryTableRow>,
}

impl NspSymbolLibraryTable {
    /// enabled schema libraries。
    pub fn enabled_schema_libraries(&self) -> impl Iterator<Item = &NspSymbolLibraryTableRow> {
        self.libraries
            .iter()
            .filter(|row| !row.disabled && row.library_type.eq_ignore_ascii_case("NekoSpice"))
    }

    /// to summary json。
    pub fn to_summary_json(&self) -> String {
        format!(
            concat!(
                "{{\n",
                "  \"source\": \"{}\",\n",
                "  \"version\": {},\n",
                "  \"library_count\": {},\n",
                "  \"enabled_schema_library_count\": {},\n",
                "  \"disabled_library_count\": {},\n",
                "  \"hidden_library_count\": {}\n",
                "}}"
            ),
            json_escape(&self.source),
            json_option(self.version.as_deref()),
            self.libraries.len(),
            self.enabled_schema_libraries().count(),
            self.libraries.iter().filter(|row| row.disabled).count(),
            self.libraries.iter().filter(|row| row.hidden).count(),
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NspSymbolLibraryTableRow {
    pub name: String,
    pub library_type: String,
    pub uri: String,
    pub options: Option<String>,
    pub description: Option<String>,
    pub hidden: bool,
    pub disabled: bool,
}

/// parse schema symbol library。
pub fn parse_symbol_library(input: &str, source: &str) -> OslResult<NspSymbolLibrary> {
    let root = parse_sexpr(input)?;
    let root_list = expect_root_list_any(&root, &["nsp_symbol_lib", "kicad_symbol_lib"])?;

    Ok(NspSymbolLibrary {
        source: source.to_string(),
        version: child_value(root_list, "version"),
        generator: child_value(root_list, "generator"),
        generator_version: child_value(root_list, "generator_version"),
        symbols: direct_children(root_list, "symbol")
            .filter_map(parse_symbol_def)
            .collect(),
    })
}

/// parse schema symbol library table。
pub fn parse_symbol_library_table(input: &str, source: &str) -> OslResult<NspSymbolLibraryTable> {
    let root = parse_sexpr(input)?;
    let root_list = expect_root_list(&root, "sym_lib_table")?;

    Ok(NspSymbolLibraryTable {
        source: source.to_string(),
        version: child_value(root_list, "version"),
        libraries: direct_children(root_list, "lib")
            .filter_map(parse_symbol_library_table_row)
            .collect(),
    })
}

fn parse_symbol_library_table_row(node: &Sexp) -> Option<NspSymbolLibraryTableRow> {
    let items = list_items(node);
    Some(NspSymbolLibraryTableRow {
        name: child_value(items, "name")?,
        library_type: child_value(items, "type")?,
        uri: child_value(items, "uri")?,
        options: child_value(items, "options"),
        description: child_value(items, "descr"),
        hidden: child(items, "hidden").is_some(),
        disabled: child(items, "disabled").is_some(),
    })
}
