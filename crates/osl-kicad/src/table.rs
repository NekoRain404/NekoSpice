use crate::coordinates::{KicadAt, KicadSize, parse_at, parse_size};
use crate::geometry::{KicadBoundingBox, KicadBoundingBoxBuilder, kicad_rotated_rect_bounds};
use crate::sexpr::{
    Sexp, atom_text, child, child_value, direct_children, format_number, list_items, list_value,
    sexpr_string,
};
use crate::style::{
    KicadFill, KicadMargins, KicadStroke, KicadTextEffects, parse_fill, parse_margins,
    parse_stroke, parse_text_effects, write_inline_fill, write_inline_optional_bool_sexpr,
    write_inline_stroke, write_text_effects_line,
};
use crate::util::{parse_kicad_bool_value, parse_optional_bool_child};

#[derive(Debug, Clone, PartialEq)]
pub struct KicadTable {
    pub column_count: usize,
    pub border: Option<KicadTableBorder>,
    pub separators: Option<KicadTableSeparators>,
    pub column_widths: Vec<f64>,
    pub row_heights: Vec<f64>,
    pub cells: Vec<KicadTableCell>,
    pub uuid: Option<String>,
    pub locked: Option<bool>,
}

impl KicadTable {
    /// bounding box。
    pub fn bounding_box(&self) -> Option<KicadBoundingBox> {
        let mut bounds = KicadBoundingBoxBuilder::default();
        for cell in &self.cells {
            if let Some(cell_bounds) = cell.bounding_box() {
                bounds.include_box(cell_bounds);
            }
        }
        bounds.finish()
    }

    /// write table sexpr。
    pub(crate) fn write_table_sexpr(&self, output: &mut String, indent: usize) {
        let pad = " ".repeat(indent);
        output.push_str(&format!(
            "{}(table\n{}  (column_count {})\n",
            pad, pad, self.column_count
        ));
        write_table_border_sexpr(output, indent + 2, self.border.as_ref());
        write_table_separators_sexpr(output, indent + 2, self.separators.as_ref());
        output.push_str(&format!("{}  (column_widths", pad));
        for width in &self.column_widths {
            output.push_str(&format!(" {}", format_number(*width)));
        }
        output.push_str(")\n");
        output.push_str(&format!("{}  (row_heights", pad));
        for height in &self.row_heights {
            output.push_str(&format!(" {}", format_number(*height)));
        }
        output.push_str(")\n");
        if let Some(uuid) = &self.uuid {
            output.push_str(&format!("{}  (uuid {})\n", pad, sexpr_string(uuid)));
        }
        if self.locked == Some(true) {
            output.push_str(&format!("{}  (locked yes)\n", pad));
        }
        output.push_str(&format!("{}  (cells\n", pad));
        for cell in &self.cells {
            cell.write_table_cell_sexpr(output, indent + 4);
        }
        output.push_str(&format!("{}  )\n", pad));
        output.push_str(&format!("{})\n", pad));
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadTableBorder {
    pub external: Option<bool>,
    pub header: Option<bool>,
    pub stroke: Option<KicadStroke>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadTableSeparators {
    pub rows: Option<bool>,
    pub cols: Option<bool>,
    pub stroke: Option<KicadStroke>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadTableCell {
    pub text: String,
    pub at: Option<KicadAt>,
    pub size: Option<KicadSize>,
    pub margins: Option<KicadMargins>,
    pub column_span: usize,
    pub row_span: usize,
    pub fill: Option<KicadFill>,
    pub effects: Option<KicadTextEffects>,
    pub exclude_from_sim: Option<bool>,
    pub uuid: Option<String>,
    pub locked: Option<bool>,
}

impl KicadTableCell {
    /// bounding box。
    pub fn bounding_box(&self) -> Option<KicadBoundingBox> {
        kicad_rotated_rect_bounds(self.at?, self.size?)
    }

    fn write_table_cell_sexpr(&self, output: &mut String, indent: usize) {
        let pad = " ".repeat(indent);
        output.push_str(&format!(
            "{}(table_cell {}\n",
            pad,
            sexpr_string(&self.text)
        ));
        if let Some(exclude_from_sim) = self.exclude_from_sim {
            output.push_str(&format!(
                "{}  (exclude_from_sim {})\n",
                pad,
                if exclude_from_sim { "yes" } else { "no" }
            ));
        }
        if let Some(at) = self.at {
            output.push_str(&format!(
                "{}  (at {} {} {})\n",
                pad,
                format_number(at.x),
                format_number(at.y),
                format_number(at.rotation)
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
        if let Some(margins) = self.margins {
            output.push_str(&format!(
                "{}  (margins {} {} {} {})\n",
                pad,
                format_number(margins.left),
                format_number(margins.top),
                format_number(margins.right),
                format_number(margins.bottom)
            ));
        }
        output.push_str(&format!(
            "{}  (span {} {})\n",
            pad, self.column_span, self.row_span
        ));
        output.push_str(&format!("{} ", pad));
        write_inline_fill(output, self.fill.as_ref());
        output.push('\n');
        write_text_effects_line(output, indent + 2, self.effects.as_ref());
        if let Some(uuid) = &self.uuid {
            output.push_str(&format!("{}  (uuid {})\n", pad, sexpr_string(uuid)));
        }
        if self.locked == Some(true) {
            output.push_str(&format!("{}  (locked yes)\n", pad));
        }
        output.push_str(&format!("{})\n", pad));
    }
}

/// parse table。
pub(crate) fn parse_table(node: &Sexp) -> Option<KicadTable> {
    let items = list_items(node);
    Some(KicadTable {
        column_count: child_value(items, "column_count")
            .and_then(|value| value.parse().ok())
            .unwrap_or(0),
        border: child(items, "border").map(parse_table_border),
        separators: child(items, "separators").map(parse_table_separators),
        column_widths: child(items, "column_widths")
            .map(parse_number_list)
            .unwrap_or_default(),
        row_heights: child(items, "row_heights")
            .map(parse_number_list)
            .unwrap_or_default(),
        cells: child(items, "cells")
            .map(|cells| {
                direct_children(list_items(cells), "table_cell")
                    .filter_map(parse_table_cell)
                    .collect()
            })
            .unwrap_or_default(),
        uuid: child_value(items, "uuid"),
        locked: child_value(items, "locked").and_then(parse_kicad_bool_value),
    })
}

fn parse_table_cell(node: &Sexp) -> Option<KicadTableCell> {
    let items = list_items(node);
    let (column_span, row_span) = child(items, "span").map(parse_span).unwrap_or((1, 1));
    Some(KicadTableCell {
        text: list_value(node, 1)?,
        at: child(items, "at").and_then(parse_at),
        size: child(items, "size").and_then(parse_size),
        margins: child(items, "margins").and_then(parse_margins),
        column_span,
        row_span,
        fill: child(items, "fill").map(parse_fill),
        effects: child(items, "effects").map(parse_text_effects),
        exclude_from_sim: child_value(items, "exclude_from_sim").and_then(parse_kicad_bool_value),
        uuid: child_value(items, "uuid"),
        locked: parse_optional_bool_child(items, "locked"),
    })
}

fn parse_table_border(node: &Sexp) -> KicadTableBorder {
    let items = list_items(node);
    KicadTableBorder {
        external: child_value(items, "external").and_then(parse_kicad_bool_value),
        header: child_value(items, "header").and_then(parse_kicad_bool_value),
        stroke: child(items, "stroke").map(parse_stroke),
    }
}

fn parse_table_separators(node: &Sexp) -> KicadTableSeparators {
    let items = list_items(node);
    KicadTableSeparators {
        rows: child_value(items, "rows").and_then(parse_kicad_bool_value),
        cols: child_value(items, "cols").and_then(parse_kicad_bool_value),
        stroke: child(items, "stroke").map(parse_stroke),
    }
}

fn write_table_border_sexpr(output: &mut String, indent: usize, border: Option<&KicadTableBorder>) {
    let pad = " ".repeat(indent);
    output.push_str(&format!("{}(border", pad));
    match border {
        Some(border) => {
            write_inline_optional_bool_sexpr(output, "external", border.external);
            write_inline_optional_bool_sexpr(output, "header", border.header);
            write_inline_stroke(output, border.stroke.as_ref(), 0.0);
        }
        None => output.push_str(" (external yes) (header yes) (stroke (width 0) (type solid))"),
    }
    output.push_str(")\n");
}

fn write_table_separators_sexpr(
    output: &mut String,
    indent: usize,
    separators: Option<&KicadTableSeparators>,
) {
    let pad = " ".repeat(indent);
    output.push_str(&format!("{}(separators", pad));
    match separators {
        Some(separators) => {
            write_inline_optional_bool_sexpr(output, "rows", separators.rows);
            write_inline_optional_bool_sexpr(output, "cols", separators.cols);
            write_inline_stroke(output, separators.stroke.as_ref(), 0.0);
        }
        None => output.push_str(" (rows yes) (cols yes) (stroke (width 0) (type solid))"),
    }
    output.push_str(")\n");
}

fn parse_span(node: &Sexp) -> (usize, usize) {
    let items = list_items(node);
    let columns = items
        .get(1)
        .and_then(atom_text)
        .and_then(|value| value.parse().ok())
        .unwrap_or(1);
    let rows = items
        .get(2)
        .and_then(atom_text)
        .and_then(|value| value.parse().ok())
        .unwrap_or(1);
    (columns, rows)
}

fn parse_number_list(node: &Sexp) -> Vec<f64> {
    list_items(node)
        .iter()
        .skip(1)
        .filter_map(atom_text)
        .filter_map(|value| value.parse().ok())
        .collect()
}
