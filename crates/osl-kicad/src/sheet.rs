//! KiCad hierarchical sheet parsing and pin management.

use crate::coordinates::{KicadAt, KicadPoint, KicadSize, parse_at, parse_size};
use crate::geometry::KicadBoundingBox;
use crate::instances::{
    KicadProjectInstance, parse_project_instances, write_project_instances_sexpr,
};
use crate::property::{KicadProperty, parse_property};
use crate::sexpr::{
    Sexp, child, child_value, direct_children, format_number, list_items, list_value,
    sexpr_atom_or_string, sexpr_string,
};
use crate::style::{
    KicadFill, KicadStroke, KicadTextEffects, parse_fill, parse_stroke, parse_text_effects,
    write_inline_fill, write_inline_stroke, write_inline_text_effects, write_optional_bool_sexpr,
};
use crate::util::{parse_kicad_bool_value, parse_optional_bool_child};

#[derive(Debug, Clone, PartialEq)]
pub struct KicadSheet {
    pub at: Option<KicadAt>,
    pub size: Option<KicadSize>,
    pub uuid: Option<String>,
    pub exclude_from_sim: Option<bool>,
    pub in_bom: Option<bool>,
    pub on_board: Option<bool>,
    pub dnp: Option<bool>,
    pub fields_autoplaced: Option<bool>,
    pub stroke: Option<KicadStroke>,
    pub fill: Option<KicadFill>,
    pub properties: Vec<KicadProperty>,
    pub pins: Vec<KicadSheetPin>,
    pub instances: Vec<KicadProjectInstance>,
}

impl KicadSheet {
    /// property。
    pub fn property(&self, name: &str) -> Option<&str> {
        self.properties
            .iter()
            .find(|property| property.name == name)
            .map(|property| property.value.as_str())
    }

    /// sheet name。
    pub fn sheet_name(&self) -> Option<&str> {
        self.property("Sheetname")
    }

    /// sheet file。
    pub fn sheet_file(&self) -> Option<&str> {
        self.property("Sheetfile")
    }

    /// bounding box。
    pub fn bounding_box(&self) -> Option<KicadBoundingBox> {
        let at = self.at?;
        let size = self.size?;
        Some(KicadBoundingBox {
            min: at.point(),
            max: KicadPoint {
                x: at.x + size.width,
                y: at.y + size.height,
            },
        })
    }

    /// write sheet sexpr。
    pub(crate) fn write_sheet_sexpr(&self, output: &mut String, indent: usize) {
        let pad = " ".repeat(indent);
        output.push_str(&format!("{}(sheet\n", pad));
        if let Some(at) = self.at {
            output.push_str(&format!(
                "{}  (at {} {})\n",
                pad,
                format_number(at.x),
                format_number(at.y)
            ));
        }
        if let Some(size) = self.size {
            output.push_str(&format!(
                "{}  (size {} {})\n",
                pad,
                format_number(size.width),
                format_number(size.height)
            ));
        }
        if let Some(exclude_from_sim) = self.exclude_from_sim {
            output.push_str(&format!(
                "{}  (exclude_from_sim {})\n",
                pad,
                if exclude_from_sim { "yes" } else { "no" }
            ));
        }
        write_optional_bool_sexpr(output, indent + 2, "in_bom", self.in_bom);
        write_optional_bool_sexpr(output, indent + 2, "on_board", self.on_board);
        write_optional_bool_sexpr(output, indent + 2, "dnp", self.dnp);
        write_optional_bool_sexpr(
            output,
            indent + 2,
            "fields_autoplaced",
            self.fields_autoplaced,
        );
        if self.stroke.is_some() {
            output.push_str(&format!("{} ", pad));
            write_inline_stroke(output, self.stroke.as_ref(), 0.0);
            output.push('\n');
        }
        if self.fill.is_some() {
            output.push_str(&format!("{} ", pad));
            write_inline_fill(output, self.fill.as_ref());
            output.push('\n');
        }
        if let Some(uuid) = &self.uuid {
            output.push_str(&format!("{}  (uuid {})\n", pad, sexpr_string(uuid)));
        }
        for property in &self.properties {
            property.write_property_sexpr(output, indent + 2);
        }
        for pin in &self.pins {
            pin.write_sheet_pin_sexpr(output, indent + 2);
        }
        write_project_instances_sexpr(output, &self.instances, indent + 2);
        output.push_str(&format!("{})\n", pad));
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadSheetPin {
    pub name: String,
    pub pin_type: String,
    pub at: Option<KicadAt>,
    pub uuid: Option<String>,
    pub effects: Option<KicadTextEffects>,
}

impl KicadSheetPin {
    fn write_sheet_pin_sexpr(&self, output: &mut String, indent: usize) {
        let pad = " ".repeat(indent);
        output.push_str(&format!(
            "{}(pin {} {}",
            pad,
            sexpr_string(&self.name),
            sexpr_atom_or_string(&self.pin_type)
        ));
        if let Some(at) = self.at {
            output.push_str(&format!(
                " (at {} {} {})",
                format_number(at.x),
                format_number(at.y),
                format_number(at.rotation)
            ));
        }
        if let Some(uuid) = &self.uuid {
            output.push_str(&format!(" (uuid {})", sexpr_string(uuid)));
        }
        write_inline_text_effects(output, self.effects.as_ref());
        output.push_str(")\n");
    }
}

/// parse sheet。
pub(crate) fn parse_sheet(node: &Sexp) -> Option<KicadSheet> {
    let items = list_items(node);
    Some(KicadSheet {
        at: child(items, "at").and_then(parse_at),
        size: child(items, "size").and_then(parse_size),
        uuid: child_value(items, "uuid"),
        exclude_from_sim: child_value(items, "exclude_from_sim").and_then(parse_kicad_bool_value),
        in_bom: child_value(items, "in_bom").and_then(parse_kicad_bool_value),
        on_board: child_value(items, "on_board").and_then(parse_kicad_bool_value),
        dnp: child_value(items, "dnp").and_then(parse_kicad_bool_value),
        fields_autoplaced: parse_optional_bool_child(items, "fields_autoplaced"),
        stroke: child(items, "stroke").map(parse_stroke),
        fill: child(items, "fill").map(parse_fill),
        properties: direct_children(items, "property")
            .filter_map(parse_property)
            .collect(),
        pins: direct_children(items, "pin")
            .filter_map(parse_sheet_pin)
            .collect(),
        instances: child(items, "instances")
            .map(parse_project_instances)
            .unwrap_or_default(),
    })
}

fn parse_sheet_pin(node: &Sexp) -> Option<KicadSheetPin> {
    let items = list_items(node);
    Some(KicadSheetPin {
        name: list_value(node, 1)?,
        pin_type: list_value(node, 2).unwrap_or_else(|| "unspecified".to_string()),
        at: child(items, "at").and_then(parse_at),
        uuid: child_value(items, "uuid"),
        effects: child(items, "effects").map(parse_text_effects),
    })
}

/// sheet properties。
pub(crate) fn sheet_properties(
    name: &str,
    file: &str,
    at: KicadAt,
    size: KicadSize,
) -> Vec<KicadProperty> {
    vec![
        KicadProperty {
            name: "Sheetname".to_string(),
            value: name.to_string(),
            id: None,
            at: Some(KicadAt {
                x: at.x,
                y: at.y - 1.27,
                rotation: 0.0,
            }),
            hide: None,
            show_name: None,
            do_not_autoplace: None,
            effects: None,
        },
        KicadProperty {
            name: "Sheetfile".to_string(),
            value: file.to_string(),
            id: None,
            at: Some(KicadAt {
                x: at.x,
                y: at.y + size.height + 1.27,
                rotation: 0.0,
            }),
            hide: None,
            show_name: None,
            do_not_autoplace: None,
            effects: None,
        },
    ]
}
