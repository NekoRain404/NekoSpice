//! Factory for creating new empty KiCad schematics.
//!
//! Provides [`new_empty_schematic`] which creates a minimal valid
//! KiCad schematic with A4 paper and default title block.

use crate::{KicadSchematic, KicadTitleBlock, KicadTitleComment};
use crate::instances::KicadSheetInstance;

/// Create a new empty schematic with default A4 paper size.
///
/// The returned schematic is valid KiCad v6+ format and can be
/// serialized with `to_kicad_schematic_sexpr()` immediately.
pub fn new_empty_schematic() -> KicadSchematic {
    let now = current_date_string();
    KicadSchematic {
        source: "<new>".to_string(),
        version: Some("20230121".to_string()),
        generator: Some("nekospice".to_string()),
        generator_version: Some("1.0".to_string()),
        uuid: Some("00000000-0000-0000-0000-000000000001".to_string()),
        paper: Some("A4".to_string()),
        title_block: Some(KicadTitleBlock {
            title: Some("New Schematic".to_string()),
            date: Some(now),
            revision: Some("1.0".to_string()),
            company: None,
            comments: vec![
                KicadTitleComment { index: 1, text: "Created by NekoSpice".to_string() },
            ],
        }),
        library_symbols: Vec::new(),
        bus_aliases: Vec::new(),
        symbols: Vec::new(),
        wires: Vec::new(),
        buses: Vec::new(),
        bus_entries: Vec::new(),
        net_chains: Vec::new(),
        graphics: Vec::new(),
        images: Vec::new(),
        tables: Vec::new(),
        rule_areas: Vec::new(),
        groups: Vec::new(),
        directive_labels: Vec::new(),
        labels: Vec::new(),
        sheets: Vec::new(),
        no_connects: Vec::new(),
        text_items: Vec::new(),
        text_boxes: Vec::new(),
        junctions: Vec::new(),
        sheet_instances: vec![KicadSheetInstance {
            path: "/".to_string(),
            page: Some("1".to_string()),
        }],
        symbol_instances: Vec::new(),
        embedded_fonts: None,
    }
}

/// Get current date as "YYYY-MM-DD" string without external dependencies.
fn current_date_string() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let days = secs / 86400;
    let mut y = 1970u32;
    let mut remaining = days;
    loop {
        let days_in_year = if is_leap(y) { 366 } else { 365 };
        if remaining < days_in_year as u64 { break; }
        remaining -= days_in_year as u64;
        y += 1;
    }
    let mut m = 1u32;
    let dim = [31, if is_leap(y) { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    for &days_in_month in &dim {
        if remaining < days_in_month as u64 { break; }
        remaining -= days_in_month as u64;
        m += 1;
    }
    format!("{:04}-{:02}-{:02}", y, m, remaining + 1)
}

fn is_leap(y: u32) -> bool {
    (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0)
}
