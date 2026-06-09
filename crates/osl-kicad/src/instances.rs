use crate::parse_kicad_bool_value;
use crate::sexpr::{Sexp, child_value, direct_children, list_items, list_value, sexpr_string};

#[derive(Debug, Clone, PartialEq)]
pub struct KicadSheetInstance {
    pub path: String,
    pub page: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadSymbolPathInstance {
    pub path: String,
    pub reference: Option<String>,
    pub unit: Option<u32>,
    pub value: Option<String>,
    pub footprint: Option<String>,
    pub variants: Vec<KicadVariantInstance>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadProjectInstance {
    pub name: String,
    pub paths: Vec<KicadInstancePath>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadInstancePath {
    pub path: String,
    pub page: Option<String>,
    pub reference: Option<String>,
    pub unit: Option<u32>,
    pub value: Option<String>,
    pub footprint: Option<String>,
    pub variants: Vec<KicadVariantInstance>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadVariantInstance {
    pub name: Option<String>,
    pub dnp: Option<bool>,
}

pub(crate) fn parse_sheet_instances(node: &Sexp) -> Vec<KicadSheetInstance> {
    direct_children(list_items(node), "path")
        .filter_map(parse_sheet_instance)
        .collect()
}

fn parse_sheet_instance(node: &Sexp) -> Option<KicadSheetInstance> {
    let items = list_items(node);
    Some(KicadSheetInstance {
        path: list_value(node, 1)?,
        page: child_value(items, "page"),
    })
}

pub(crate) fn parse_symbol_path_instances(node: &Sexp) -> Vec<KicadSymbolPathInstance> {
    direct_children(list_items(node), "path")
        .filter_map(parse_symbol_path_instance)
        .collect()
}

fn parse_symbol_path_instance(node: &Sexp) -> Option<KicadSymbolPathInstance> {
    let path = parse_instance_path(node)?;
    Some(KicadSymbolPathInstance {
        path: path.path,
        reference: path.reference,
        unit: path.unit,
        value: path.value,
        footprint: path.footprint,
        variants: path.variants,
    })
}

pub(crate) fn parse_project_instances(node: &Sexp) -> Vec<KicadProjectInstance> {
    direct_children(list_items(node), "project")
        .filter_map(parse_project_instance)
        .collect()
}

fn parse_project_instance(node: &Sexp) -> Option<KicadProjectInstance> {
    let items = list_items(node);
    Some(KicadProjectInstance {
        name: list_value(node, 1)?,
        paths: direct_children(items, "path")
            .filter_map(parse_instance_path)
            .collect(),
    })
}

fn parse_instance_path(node: &Sexp) -> Option<KicadInstancePath> {
    let items = list_items(node);
    Some(KicadInstancePath {
        path: list_value(node, 1)?,
        page: child_value(items, "page"),
        reference: child_value(items, "reference"),
        unit: child_value(items, "unit").and_then(|value| value.parse().ok()),
        value: child_value(items, "value"),
        footprint: child_value(items, "footprint"),
        variants: direct_children(items, "variant")
            .filter_map(parse_variant_instance)
            .collect(),
    })
}

fn parse_variant_instance(node: &Sexp) -> Option<KicadVariantInstance> {
    let items = list_items(node);
    Some(KicadVariantInstance {
        name: child_value(items, "name"),
        dnp: child_value(items, "dnp").and_then(parse_kicad_bool_value),
    })
}

pub(crate) fn write_sheet_instances_sexpr(
    output: &mut String,
    instances: &[KicadSheetInstance],
    indent: usize,
) {
    let pad = " ".repeat(indent);
    output.push_str(&format!("{}(sheet_instances\n", pad));
    for instance in instances {
        output.push_str(&format!("{}  (path {}", pad, sexpr_string(&instance.path)));
        if let Some(page) = &instance.page {
            output.push_str(&format!(" (page {})", sexpr_string(page)));
        }
        output.push_str(")\n");
    }
    output.push_str(&format!("{})\n", pad));
}

pub(crate) fn write_symbol_path_instances_sexpr(
    output: &mut String,
    instances: &[KicadSymbolPathInstance],
    indent: usize,
) {
    let pad = " ".repeat(indent);
    output.push_str(&format!("{}(symbol_instances\n", pad));
    for instance in instances {
        output.push_str(&format!(
            "{}  (path {}\n",
            pad,
            sexpr_string(&instance.path)
        ));
        if let Some(reference) = &instance.reference {
            output.push_str(&format!(
                "{}    (reference {})\n",
                pad,
                sexpr_string(reference)
            ));
        }
        if let Some(unit) = instance.unit {
            output.push_str(&format!("{}    (unit {})\n", pad, unit));
        }
        if let Some(value) = &instance.value {
            output.push_str(&format!("{}    (value {})\n", pad, sexpr_string(value)));
        }
        if let Some(footprint) = &instance.footprint {
            output.push_str(&format!(
                "{}    (footprint {})\n",
                pad,
                sexpr_string(footprint)
            ));
        }
        for variant in &instance.variants {
            write_variant_instance_sexpr(output, variant, indent + 4);
        }
        output.push_str(&format!("{}  )\n", pad));
    }
    output.push_str(&format!("{})\n", pad));
}

pub(crate) fn write_project_instances_sexpr(
    output: &mut String,
    instances: &[KicadProjectInstance],
    indent: usize,
) {
    if instances.is_empty() {
        return;
    }
    let pad = " ".repeat(indent);
    output.push_str(&format!("{}(instances\n", pad));
    for instance in instances {
        output.push_str(&format!(
            "{}  (project {}\n",
            pad,
            sexpr_string(&instance.name)
        ));
        for path in &instance.paths {
            write_instance_path_sexpr(output, path, indent + 4);
        }
        output.push_str(&format!("{}  )\n", pad));
    }
    output.push_str(&format!("{})\n", pad));
}

fn write_instance_path_sexpr(output: &mut String, path: &KicadInstancePath, indent: usize) {
    let pad = " ".repeat(indent);
    output.push_str(&format!("{}(path {}\n", pad, sexpr_string(&path.path)));
    if let Some(page) = &path.page {
        output.push_str(&format!("{}  (page {})\n", pad, sexpr_string(page)));
    }
    if let Some(reference) = &path.reference {
        output.push_str(&format!(
            "{}  (reference {})\n",
            pad,
            sexpr_string(reference)
        ));
    }
    if let Some(unit) = path.unit {
        output.push_str(&format!("{}  (unit {})\n", pad, unit));
    }
    if let Some(value) = &path.value {
        output.push_str(&format!("{}  (value {})\n", pad, sexpr_string(value)));
    }
    if let Some(footprint) = &path.footprint {
        output.push_str(&format!(
            "{}  (footprint {})\n",
            pad,
            sexpr_string(footprint)
        ));
    }
    for variant in &path.variants {
        write_variant_instance_sexpr(output, variant, indent + 2);
    }
    output.push_str(&format!("{})\n", pad));
}

fn write_variant_instance_sexpr(
    output: &mut String,
    variant: &KicadVariantInstance,
    indent: usize,
) {
    let pad = " ".repeat(indent);
    output.push_str(&format!("{}(variant\n", pad));
    if let Some(name) = &variant.name {
        output.push_str(&format!("{}  (name {})\n", pad, sexpr_string(name)));
    }
    if let Some(dnp) = variant.dnp {
        output.push_str(&format!(
            "{}  (dnp {})\n",
            pad,
            if dnp { "yes" } else { "no" }
        ));
    }
    output.push_str(&format!("{})\n", pad));
}
