# osl-kicad

`osl-kicad` is the Rust-native KiCad-compatible schematic, project, symbol
library, canvas-scene, edit, and netlist foundation for NekoSpice.

## Boundaries

- `lib.rs` currently exposes the public KiCad API and still owns the main
  schematic/symbol parsers, writer, schematic IR, and schematic-to-SPICE
  export.
- `diagnostics.rs` owns schematic check report DTOs, hierarchy netlist report
  DTOs, diagnostic severity formatting, and shared diagnostic constructors.
- `metadata.rs` owns schematic title block/comment IR plus title block
  parse/write helpers.
- `instances.rs` owns sheet/symbol/project instance path IR, variant DNP
  metadata, and embedded/top-level instance parse/write helpers.
- `schematic_summary.rs` owns schematic summary JSON generation and related
  metadata/style/count aggregation helpers.
- `project.rs` owns `.kicad_pro` project JSON parsing, project summary DTOs, and
  schematic stem candidate discovery used by KiCad project import paths.
- `symbol_library.rs` owns `.kicad_sym` library containers, `sym-lib-table`
  containers, library/table summary JSON, library-level writer entry points, and
  library/table root parsers.
- `edit.rs` owns the public schematic edit command DTOs, edit summaries, symbol
  placement payloads, move/delete helpers, edit validation, and stable edit UUID
  generation used by GUI and CLI mutation paths.
- `library_index.rs` owns the GUI-facing symbol library index, library browser
  query filters, indexed unit/body-style/pin metadata, and library load
  diagnostics built from `sym-lib-table`.
- `canvas.rs` owns the canvas scene DTOs, schematic/symbol-to-scene projection,
  canvas JSON export, and canvas item bounding metadata used by frontends.
- `canvas_hit.rs` owns canvas hit-test reports, point selection, and UUID-based
  selection refresh helpers used by GUI/editor state.
- `connectivity.rs` owns schematic connectivity graph construction, quantized
  point keys, wire segment membership, net label normalization, and generated
  net naming used by diagnostics and SPICE export.
- `sexpr.rs` owns the reusable KiCad S-expression tree, parser, tree-navigation
  helpers, atom/string escaping, inline writer, and numeric formatting used by
  the parser/writer layers.
- `json.rs` owns small shared JSON summary formatting helpers used by project,
  schematic, and library reports.
- `transform.rs` owns symbol mirror normalization plus local-to-symbol point/at
  transforms and rotation normalization shared by canvas, netlist, and editing
  paths.
- `geometry.rs` owns reusable schematic/canvas geometry: bounding boxes,
  point/segment distances, rotated rectangles, sheet pin and no-connect marker
  geometry, text bounds estimation, polygon/polyline/Bezier/arc hit-testing, and
  circular arc sampling used by both canvas hit-tests and render/export helpers.

## Refactor Direction

Keep file-format parsing, IR mutation, symbol-library resolution, connectivity,
canvas scene projection, hit-testing, and geometry math in separate modules.
Future cleanup should peel off the remaining mutation method bodies and parser
families while preserving the public API consumed by `osl-app`, `osl-render`,
`osl-netlist`, and `osl-cli`.
