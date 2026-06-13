# osl-schema File Split

## Overview

The `osl-schema` crate provides Schema `.nsp_sch`/`.nsp_sym`/`.nsp_pro` parsing, canvas scene generation, schematic editing, and geometry utilities.

## Canvas Items Split

| File | Lines | Purpose |
|------|-------|---------|
| `src/canvas_items.rs` | ~158 | Header container types + include! directives |
| `src/canvas_items_graphic_impl.rs` | ~343 | `NspCanvasGraphic` enum (Polyline, Bezier, Rectangle, Circle, Arc, Text) and impl |
| `src/canvas_items_leaf_impl.rs` | ~328 | Simple leaf types: Image, Table, TableCell, Pin, Wire, Bus, BusEntry, Label, Text, TextBox, Junction, NoConnect, Group |
| `src/canvas_items_bounds_impl.rs` | ~116 | Bounds calculation helpers |

### Inclusion chain
```
canvas.rs → include!("canvas_items.rs")
  canvas_items.rs → include!("canvas_items_graphic_impl.rs")
  canvas_items.rs → include!("canvas_items_leaf_impl.rs")
  canvas_items.rs → include!("canvas_items_bounds_impl.rs")
```

## Schematic Edit Split

| File | Lines | Purpose |
|------|-------|---------|
| `src/schematic_edit_impl.rs` | ~6 | Hub file with include! directives |
| `src/schematic_edit_symbol_ops_impl.rs` | ~601 | Symbol operations: apply_edit, move/delete/configure/place |
| `src/schematic_edit_wiring_ops_impl.rs` | ~322 | Wiring: add_wire, add_bus, add_bus_entry, add_junction, add_no_connect, add_label, add_text, add_sheet |

### Inclusion chain
```
lib.rs → include!("schematic_edit_impl.rs")
  schematic_edit_impl.rs → include!("schematic_edit_symbol_ops_impl.rs")
  schematic_edit_impl.rs → include!("schematic_edit_wiring_ops_impl.rs")
```

## Rendering Improvements

- **Arc rendering**: Uses `sample_schema_arc_points()` for proper arc interpolation instead of 3-point V-shape
- **Bezier rendering**: Quadratic bezier approximation with 24-segment sampling
- **Text rendering**: Respects Schema `font_size` from text effects, uses proportional font
- **Symbol labels**: Shows both reference (e.g. R1) and value (e.g. 10k) labels
