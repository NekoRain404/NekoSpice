# osl-kicad

`osl-kicad` is the Rust-native KiCad-compatible schematic, project, symbol
library, canvas-scene, edit, and netlist foundation for NekoSpice.

## Boundaries

- `lib.rs` currently exposes the public KiCad API and still owns the main
  schematic/project/symbol parsers, writer, schematic IR, edit commands,
  diagnostics, and schematic-to-SPICE export.
- `library_index.rs` owns the GUI-facing symbol library index, library browser
  query filters, indexed unit/body-style/pin metadata, and library load
  diagnostics built from `sym-lib-table`.
- `canvas.rs` owns the canvas scene DTOs, schematic/symbol-to-scene projection,
  canvas JSON export, and canvas item bounding metadata used by frontends.
- `canvas_hit.rs` owns canvas hit-test reports, point selection, and UUID-based
  selection refresh helpers used by GUI/editor state.
- `sexpr.rs` owns the reusable KiCad S-expression tree, parser, tree-navigation
  helpers, atom/string escaping, inline writer, and numeric formatting used by
  the parser/writer layers.
- `geometry.rs` owns reusable schematic/canvas geometry: bounding boxes,
  point/segment distances, rotated rectangles, sheet pin and no-connect marker
  geometry, text bounds estimation, polygon/polyline/Bezier/arc hit-testing, and
  circular arc sampling used by both canvas hit-tests and render/export helpers.

## Refactor Direction

Keep file-format parsing, IR mutation, symbol-library resolution, canvas scene
projection, hit-testing, and geometry math in separate modules. Future cleanup
should peel off schematic edit commands and parser families while preserving
the public API consumed by `osl-app`, `osl-render`, `osl-netlist`, and
`osl-cli`.
