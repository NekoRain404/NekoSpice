//! KiCad schematic I/O - reading, writing, parsing, and serialization.

use crate::graphics::{parse_rule_area, parse_schematic_graphic};
use crate::group::parse_group;
use crate::image::parse_image;
use crate::instances::{
    parse_sheet_instances, parse_symbol_path_instances, write_sheet_instances_sexpr,
    write_symbol_path_instances_sexpr,
};
use crate::labels::{parse_directive_label, parse_label};
use crate::markers::{parse_junction, parse_no_connect};
use crate::metadata::parse_title_block;
use crate::sexpr::{
    child, child_value, direct_children, expect_root_list, list_items, sexpr_atom_or_string,
    sexpr_string,
};
use crate::sheet::parse_sheet;
use crate::symbols::{parse_symbol_def, parse_symbol_instance};
use crate::table::parse_table;
use crate::text::{parse_text_box, parse_text_item};
use crate::util::parse_kicad_bool_value;
use crate::wiring::{parse_bus, parse_bus_alias, parse_bus_entry, parse_net_chain, parse_wire};
use crate::{KicadLabelKind, KicadSchematic, parse_sexpr};
use osl_core::{OslResult, read_text, write_text};
use std::path::Path;

/// read kicad schematic。
pub fn read_kicad_schematic(path: &Path) -> OslResult<KicadSchematic> {
    let content = read_text(path)?;
    parse_kicad_schematic(&content, &path.display().to_string())
}

/// read kicad schematic with libraries。
pub fn read_kicad_schematic_with_libraries(path: &Path) -> OslResult<KicadSchematic> {
    let mut schematic = read_kicad_schematic(path)?;
    if let Some(project_dir) = path.parent() {
        schematic.resolve_project_symbol_libraries(project_dir)?;
    }
    Ok(schematic)
}

/// write kicad schematic。
pub fn write_kicad_schematic(path: &Path, schematic: &KicadSchematic) -> OslResult<()> {
    write_text(path, &schematic.to_kicad_schematic_sexpr())
}

/// parse kicad schematic。
pub fn parse_kicad_schematic(input: &str, source: &str) -> OslResult<KicadSchematic> {
    let root = parse_sexpr(input)?;
    let root_list = expect_root_list(&root, "kicad_sch")?;
    let library_symbols = direct_children(root_list, "lib_symbols")
        .flat_map(|lib_symbols| direct_children(list_items(lib_symbols), "symbol"))
        .filter_map(parse_symbol_def)
        .collect::<Vec<_>>();

    Ok(KicadSchematic {
        source: source.to_string(),
        version: child_value(root_list, "version"),
        generator: child_value(root_list, "generator"),
        generator_version: child_value(root_list, "generator_version"),
        uuid: child_value(root_list, "uuid"),
        paper: child_value(root_list, "paper"),
        title_block: child(root_list, "title_block").map(parse_title_block),
        library_symbols,
        bus_aliases: direct_children(root_list, "bus_alias")
            .filter_map(parse_bus_alias)
            .collect(),
        symbols: direct_children(root_list, "symbol")
            .filter_map(parse_symbol_instance)
            .collect(),
        wires: direct_children(root_list, "wire")
            .map(parse_wire)
            .collect::<Vec<_>>(),
        buses: direct_children(root_list, "bus")
            .map(parse_bus)
            .collect::<Vec<_>>(),
        bus_entries: direct_children(root_list, "bus_entry")
            .filter_map(parse_bus_entry)
            .collect(),
        net_chains: direct_children(root_list, "net_chain")
            .filter_map(parse_net_chain)
            .collect(),
        graphics: root_list
            .iter()
            .filter_map(parse_schematic_graphic)
            .collect(),
        images: direct_children(root_list, "image")
            .filter_map(parse_image)
            .collect(),
        tables: direct_children(root_list, "table")
            .filter_map(parse_table)
            .collect(),
        rule_areas: direct_children(root_list, "rule_area")
            .filter_map(parse_rule_area)
            .collect(),
        groups: direct_children(root_list, "group")
            .filter_map(parse_group)
            .collect(),
        directive_labels: direct_children(root_list, "netclass_flag")
            .filter_map(parse_directive_label)
            .collect(),
        labels: direct_children(root_list, "label")
            .filter_map(|node| parse_label(node, KicadLabelKind::Local))
            .chain(
                direct_children(root_list, "global_label")
                    .filter_map(|node| parse_label(node, KicadLabelKind::Global)),
            )
            .chain(
                direct_children(root_list, "hierarchical_label")
                    .filter_map(|node| parse_label(node, KicadLabelKind::Hierarchical)),
            )
            .collect(),
        sheets: direct_children(root_list, "sheet")
            .filter_map(parse_sheet)
            .collect(),
        no_connects: direct_children(root_list, "no_connect")
            .filter_map(parse_no_connect)
            .collect(),
        text_items: direct_children(root_list, "text")
            .filter_map(parse_text_item)
            .collect(),
        text_boxes: direct_children(root_list, "text_box")
            .filter_map(parse_text_box)
            .collect(),
        junctions: direct_children(root_list, "junction")
            .filter_map(parse_junction)
            .collect(),
        sheet_instances: child(root_list, "sheet_instances")
            .map(parse_sheet_instances)
            .unwrap_or_default(),
        symbol_instances: child(root_list, "symbol_instances")
            .map(parse_symbol_path_instances)
            .unwrap_or_default(),
        embedded_fonts: child_value(root_list, "embedded_fonts").and_then(parse_kicad_bool_value),
    })
}

impl KicadSchematic {
    /// to kicad schematic sexpr。
    pub fn to_kicad_schematic_sexpr(&self) -> String {
        let mut output = String::new();
        output.push_str("(kicad_sch\n");
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
        if let Some(uuid) = &self.uuid {
            output.push_str(&format!("  (uuid {})\n", sexpr_string(uuid)));
        }
        output.push_str(&format!(
            "  (paper {})\n",
            sexpr_string(self.paper.as_deref().unwrap_or("A4"))
        ));
        if let Some(title_block) = &self.title_block {
            title_block.write_title_block_sexpr(&mut output, 2);
        }
        output.push_str("  (lib_symbols\n");
        for symbol in &self.library_symbols {
            symbol.write_symbol_sexpr(&mut output, 4);
        }
        output.push_str("  )\n");
        for alias in &self.bus_aliases {
            alias.write_bus_alias_sexpr(&mut output, 2);
        }
        for wire in &self.wires {
            wire.write_wire_sexpr(&mut output, 2);
        }
        for bus in &self.buses {
            bus.write_bus_sexpr(&mut output, 2);
        }
        for entry in &self.bus_entries {
            entry.write_bus_entry_sexpr(&mut output, 2);
        }
        for net_chain in &self.net_chains {
            net_chain.write_net_chain_sexpr(&mut output, 2);
        }
        for graphic in &self.graphics {
            graphic.write_schematic_graphic_sexpr(&mut output, 2);
        }
        for image in &self.images {
            image.write_image_sexpr(&mut output, 2);
        }
        for table in &self.tables {
            table.write_table_sexpr(&mut output, 2);
        }
        for rule_area in &self.rule_areas {
            rule_area.write_rule_area_sexpr(&mut output, 2);
        }
        for group in &self.groups {
            group.write_group_sexpr(&mut output, 2);
        }
        for junction in &self.junctions {
            junction.write_junction_sexpr(&mut output, 2);
        }
        for no_connect in &self.no_connects {
            no_connect.write_no_connect_sexpr(&mut output, 2);
        }
        for label in &self.labels {
            label.write_label_sexpr(&mut output, 2);
        }
        for label in &self.directive_labels {
            label.write_directive_label_sexpr(&mut output, 2);
        }
        for sheet in &self.sheets {
            sheet.write_sheet_sexpr(&mut output, 2);
        }
        for text in &self.text_items {
            text.write_text_sexpr(&mut output, 2);
        }
        for text_box in &self.text_boxes {
            text_box.write_text_box_sexpr(&mut output, 2);
        }
        for symbol in &self.symbols {
            symbol.write_instance_sexpr(&mut output, 2);
        }
        if !self.sheet_instances.is_empty() {
            write_sheet_instances_sexpr(&mut output, &self.sheet_instances, 2);
        }
        if !self.symbol_instances.is_empty() {
            write_symbol_path_instances_sexpr(&mut output, &self.symbol_instances, 2);
        }
        if let Some(embedded_fonts) = self.embedded_fonts {
            output.push_str(&format!(
                "  (embedded_fonts {})\n",
                if embedded_fonts { "yes" } else { "no" }
            ));
        }
        output.push_str(")\n");
        output
    }
}
