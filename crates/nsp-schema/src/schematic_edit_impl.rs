// Schematic edit and placement operations.
// Symbol operations: apply_edit, move/delete/configure/place symbols.
include!("schematic_edit_symbol_ops_impl.rs");

// Wiring and annotation: add_wire, add_bus, add_junction, add_label, add_text, add_sheet.
include!("schematic_edit_wiring_ops_impl.rs");
