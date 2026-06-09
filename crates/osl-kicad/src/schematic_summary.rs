use crate::json::{json_bool_option, json_option};
use crate::{KicadProperty, KicadSchematic};
use osl_core::json_escape;

impl KicadSchematic {
    pub fn to_summary_json(&self) -> String {
        format!(
            concat!(
                "{{\n",
                "  \"source\": \"{}\",\n",
                "  \"version\": {},\n",
                "  \"generator\": {},\n",
                "  \"generator_version\": {},\n",
                "  \"has_title_block\": {},\n",
                "  \"title_comment_count\": {},\n",
                "  \"symbol_count\": {},\n",
                "  \"library_symbol_count\": {},\n",
                "  \"bus_alias_count\": {},\n",
                "  \"wire_count\": {},\n",
                "  \"styled_wire_count\": {},\n",
                "  \"bus_count\": {},\n",
                "  \"styled_bus_count\": {},\n",
                "  \"bus_entry_count\": {},\n",
                "  \"styled_bus_entry_count\": {},\n",
                "  \"net_chain_count\": {},\n",
                "  \"net_chain_member_net_count\": {},\n",
                "  \"schematic_graphic_count\": {},\n",
                "  \"styled_schematic_graphic_count\": {},\n",
                "  \"locked_schematic_graphic_count\": {},\n",
                "  \"image_count\": {},\n",
                "  \"table_count\": {},\n",
                "  \"styled_table_count\": {},\n",
                "  \"table_cell_count\": {},\n",
                "  \"styled_table_cell_count\": {},\n",
                "  \"locked_table_cell_count\": {},\n",
                "  \"rule_area_count\": {},\n",
                "  \"styled_rule_area_count\": {},\n",
                "  \"locked_rule_area_count\": {},\n",
                "  \"group_count\": {},\n",
                "  \"group_member_count\": {},\n",
                "  \"label_count\": {},\n",
                "  \"directive_label_count\": {},\n",
                "  \"directive_label_property_count\": {},\n",
                "  \"junction_count\": {},\n",
                "  \"styled_junction_count\": {},\n",
                "  \"no_connect_count\": {},\n",
                "  \"sheet_count\": {},\n",
                "  \"styled_sheet_count\": {},\n",
                "  \"sheet_pin_count\": {},\n",
                "  \"text_count\": {},\n",
                "  \"text_box_count\": {},\n",
                "  \"styled_text_box_count\": {},\n",
                "  \"locked_text_box_count\": {},\n",
                "  \"spice_directive_count\": {},\n",
                "  \"sheet_instance_count\": {},\n",
                "  \"symbol_instance_count\": {},\n",
                "  \"symbol_pin_alternate_count\": {},\n",
                "  \"embedded_project_instance_count\": {},\n",
                "  \"embedded_instance_path_count\": {},\n",
                "  \"variant_instance_count\": {},\n",
                "  \"dnp_item_count\": {},\n",
                "  \"bom_excluded_count\": {},\n",
                "  \"board_excluded_count\": {},\n",
                "  \"mirrored_symbol_count\": {},\n",
                "  \"symbol_body_style_count\": {},\n",
                "  \"fields_autoplaced_count\": {},\n",
                "  \"shaped_label_count\": {},\n",
                "  \"label_property_count\": {},\n",
                "  \"hidden_property_count\": {},\n",
                "  \"property_effect_count\": {},\n",
                "  \"embedded_fonts\": {},\n",
                "  \"library_unit_name_count\": {},\n",
                "  \"library_graphic_count\": {}\n",
                "}}"
            ),
            json_escape(&self.source),
            json_option(self.version.as_deref()),
            json_option(self.generator.as_deref()),
            json_option(self.generator_version.as_deref()),
            self.title_block.is_some(),
            self.title_block
                .as_ref()
                .map(|title_block| title_block.comments.len())
                .unwrap_or(0),
            self.symbols.len(),
            self.library_symbols.len(),
            self.bus_aliases.len(),
            self.wires.len(),
            self.styled_wire_count(),
            self.buses.len(),
            self.styled_bus_count(),
            self.bus_entries.len(),
            self.styled_bus_entry_count(),
            self.net_chains.len(),
            self.net_chain_member_net_count(),
            self.graphics.len(),
            self.styled_schematic_graphic_count(),
            self.locked_schematic_graphic_count(),
            self.images.len(),
            self.tables.len(),
            self.styled_table_count(),
            self.tables
                .iter()
                .map(|table| table.cells.len())
                .sum::<usize>(),
            self.styled_table_cell_count(),
            self.locked_table_cell_count(),
            self.rule_areas.len(),
            self.styled_rule_area_count(),
            self.locked_rule_area_count(),
            self.groups.len(),
            self.groups
                .iter()
                .map(|group| group.members.len())
                .sum::<usize>(),
            self.labels.len(),
            self.directive_labels.len(),
            self.directive_label_property_count(),
            self.junctions.len(),
            self.styled_junction_count(),
            self.no_connects.len(),
            self.sheets.len(),
            self.styled_sheet_count(),
            self.sheets
                .iter()
                .map(|sheet| sheet.pins.len())
                .sum::<usize>(),
            self.text_items.len(),
            self.text_boxes.len(),
            self.styled_text_box_count(),
            self.locked_text_box_count(),
            self.spice_directives().len(),
            self.sheet_instances.len(),
            self.symbol_instances.len(),
            self.symbol_pin_alternate_count(),
            self.embedded_project_instance_count(),
            self.embedded_instance_path_count(),
            self.variant_instance_count(),
            self.dnp_item_count(),
            self.bom_excluded_count(),
            self.board_excluded_count(),
            self.mirrored_symbol_count(),
            self.symbol_body_style_count(),
            self.fields_autoplaced_count(),
            self.shaped_label_count(),
            self.label_property_count(),
            self.hidden_property_count(),
            self.property_effect_count(),
            json_bool_option(self.embedded_fonts),
            self.library_unit_name_count(),
            self.library_symbols
                .iter()
                .map(|symbol| symbol.graphics.len())
                .sum::<usize>()
        )
    }

    fn library_unit_name_count(&self) -> usize {
        self.library_symbols
            .iter()
            .map(|symbol| symbol.unit_names.len())
            .sum()
    }

    fn embedded_project_instance_count(&self) -> usize {
        self.symbols
            .iter()
            .map(|symbol| symbol.instances.len())
            .sum::<usize>()
            + self
                .sheets
                .iter()
                .map(|sheet| sheet.instances.len())
                .sum::<usize>()
    }

    fn symbol_pin_alternate_count(&self) -> usize {
        self.symbols
            .iter()
            .flat_map(|symbol| &symbol.pins)
            .filter(|pin| pin.alternate.is_some())
            .count()
    }

    fn embedded_instance_path_count(&self) -> usize {
        self.symbols
            .iter()
            .flat_map(|symbol| &symbol.instances)
            .map(|instance| instance.paths.len())
            .sum::<usize>()
            + self
                .sheets
                .iter()
                .flat_map(|sheet| &sheet.instances)
                .map(|instance| instance.paths.len())
                .sum::<usize>()
    }

    fn variant_instance_count(&self) -> usize {
        let embedded_variants = self
            .symbols
            .iter()
            .flat_map(|symbol| &symbol.instances)
            .flat_map(|instance| &instance.paths)
            .map(|path| path.variants.len())
            .sum::<usize>()
            + self
                .sheets
                .iter()
                .flat_map(|sheet| &sheet.instances)
                .flat_map(|instance| &instance.paths)
                .map(|path| path.variants.len())
                .sum::<usize>();
        let top_level_variants = self
            .symbol_instances
            .iter()
            .map(|instance| instance.variants.len())
            .sum::<usize>();
        embedded_variants + top_level_variants
    }

    fn styled_schematic_graphic_count(&self) -> usize {
        self.graphics
            .iter()
            .filter(|graphic| graphic.stroke.is_some() || graphic.fill.is_some())
            .count()
    }

    fn styled_wire_count(&self) -> usize {
        self.wires
            .iter()
            .filter(|wire| wire.stroke.is_some())
            .count()
    }

    fn styled_bus_count(&self) -> usize {
        self.buses.iter().filter(|bus| bus.stroke.is_some()).count()
    }

    fn styled_bus_entry_count(&self) -> usize {
        self.bus_entries
            .iter()
            .filter(|entry| entry.stroke.is_some())
            .count()
    }

    fn net_chain_member_net_count(&self) -> usize {
        self.net_chains
            .iter()
            .map(|net_chain| net_chain.member_nets.len())
            .sum()
    }

    fn locked_schematic_graphic_count(&self) -> usize {
        self.graphics
            .iter()
            .filter(|graphic| graphic.locked == Some(true))
            .count()
    }

    fn styled_text_box_count(&self) -> usize {
        self.text_boxes
            .iter()
            .filter(|text_box| text_box.stroke.is_some() || text_box.fill.is_some())
            .count()
    }

    fn locked_text_box_count(&self) -> usize {
        self.text_boxes
            .iter()
            .filter(|text_box| text_box.locked == Some(true))
            .count()
    }

    fn styled_table_count(&self) -> usize {
        self.tables
            .iter()
            .filter(|table| table.border.is_some() || table.separators.is_some())
            .count()
    }

    fn styled_table_cell_count(&self) -> usize {
        self.tables
            .iter()
            .flat_map(|table| &table.cells)
            .filter(|cell| cell.fill.is_some() || cell.effects.is_some())
            .count()
    }

    fn locked_table_cell_count(&self) -> usize {
        self.tables
            .iter()
            .flat_map(|table| &table.cells)
            .filter(|cell| cell.locked == Some(true))
            .count()
    }

    fn styled_rule_area_count(&self) -> usize {
        self.rule_areas
            .iter()
            .filter(|rule_area| rule_area.stroke.is_some() || rule_area.fill.is_some())
            .count()
    }

    fn locked_rule_area_count(&self) -> usize {
        self.rule_areas
            .iter()
            .filter(|rule_area| rule_area.locked == Some(true))
            .count()
    }

    fn styled_junction_count(&self) -> usize {
        self.junctions
            .iter()
            .filter(|junction| junction.diameter.is_some() || junction.color.is_some())
            .count()
    }

    fn styled_sheet_count(&self) -> usize {
        self.sheets
            .iter()
            .filter(|sheet| sheet.stroke.is_some() || sheet.fill.is_some())
            .count()
    }

    fn dnp_item_count(&self) -> usize {
        self.symbols
            .iter()
            .filter(|symbol| symbol.dnp == Some(true))
            .count()
            + self
                .sheets
                .iter()
                .filter(|sheet| sheet.dnp == Some(true))
                .count()
            + self
                .rule_areas
                .iter()
                .filter(|rule_area| rule_area.dnp == Some(true))
                .count()
    }

    fn bom_excluded_count(&self) -> usize {
        self.symbols
            .iter()
            .filter(|symbol| symbol.in_bom == Some(false))
            .count()
            + self
                .sheets
                .iter()
                .filter(|sheet| sheet.in_bom == Some(false))
                .count()
            + self
                .rule_areas
                .iter()
                .filter(|rule_area| rule_area.in_bom == Some(false))
                .count()
    }

    fn board_excluded_count(&self) -> usize {
        self.symbols
            .iter()
            .filter(|symbol| symbol.on_board == Some(false))
            .count()
            + self
                .sheets
                .iter()
                .filter(|sheet| sheet.on_board == Some(false))
                .count()
            + self
                .rule_areas
                .iter()
                .filter(|rule_area| rule_area.on_board == Some(false))
                .count()
    }

    fn mirrored_symbol_count(&self) -> usize {
        self.symbols
            .iter()
            .filter(|symbol| symbol.mirror.is_some())
            .count()
    }

    fn symbol_body_style_count(&self) -> usize {
        self.symbols
            .iter()
            .filter(|symbol| symbol.body_style.is_some())
            .count()
    }

    fn fields_autoplaced_count(&self) -> usize {
        self.symbols
            .iter()
            .filter(|symbol| symbol.fields_autoplaced == Some(true))
            .count()
            + self
                .sheets
                .iter()
                .filter(|sheet| sheet.fields_autoplaced == Some(true))
                .count()
            + self
                .labels
                .iter()
                .filter(|label| label.fields_autoplaced == Some(true))
                .count()
            + self
                .directive_labels
                .iter()
                .filter(|label| label.fields_autoplaced == Some(true))
                .count()
    }

    fn shaped_label_count(&self) -> usize {
        self.labels
            .iter()
            .filter(|label| label.shape.is_some())
            .count()
    }

    fn label_property_count(&self) -> usize {
        self.labels.iter().map(|label| label.properties.len()).sum()
    }

    fn directive_label_property_count(&self) -> usize {
        self.directive_labels
            .iter()
            .map(|label| label.properties.len())
            .sum()
    }

    fn hidden_property_count(&self) -> usize {
        self.symbols
            .iter()
            .flat_map(|symbol| &symbol.properties)
            .filter(|property| property_is_hidden(property))
            .count()
            + self
                .sheets
                .iter()
                .flat_map(|sheet| &sheet.properties)
                .filter(|property| property_is_hidden(property))
                .count()
            + self
                .labels
                .iter()
                .flat_map(|label| &label.properties)
                .filter(|property| property_is_hidden(property))
                .count()
            + self
                .directive_labels
                .iter()
                .flat_map(|label| &label.properties)
                .filter(|property| property_is_hidden(property))
                .count()
    }

    fn property_effect_count(&self) -> usize {
        self.symbols
            .iter()
            .flat_map(|symbol| &symbol.properties)
            .filter(|property| property.effects.is_some())
            .count()
            + self
                .sheets
                .iter()
                .flat_map(|sheet| &sheet.properties)
                .filter(|property| property.effects.is_some())
                .count()
            + self
                .labels
                .iter()
                .flat_map(|label| &label.properties)
                .filter(|property| property.effects.is_some())
                .count()
            + self
                .directive_labels
                .iter()
                .flat_map(|label| &label.properties)
                .filter(|property| property.effects.is_some())
                .count()
    }
}

fn property_is_hidden(property: &KicadProperty) -> bool {
    property.hide == Some(true)
        || property
            .effects
            .as_ref()
            .is_some_and(|effects| effects.hide)
}
