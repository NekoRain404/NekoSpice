# osl-schema

`osl-schema` is the Rust-native Schema-compatible schematic, project, symbol
library, canvas-scene, edit, and netlist foundation for NekoSpice.

## Boundaries

- `lib.rs` currently exposes the public Schema API and still owns top-level
  root API assembly.
- `diagnostics.rs` owns schematic check report DTOs, hierarchy netlist report
  DTOs, diagnostic severity formatting, and shared diagnostic constructors.
- `metadata.rs` owns schematic title block/comment IR plus title block
  parse/write helpers.
- `instances.rs` owns sheet/symbol/project instance path IR, variant DNP
  metadata, and embedded/top-level instance parse/write helpers.
- `schematic_summary.rs` owns schematic summary JSON generation and related
  metadata/style/count aggregation helpers.
- `schematic_io.rs` owns top-level `.schema_sch` read/write entry points, root
  S-expression parsing, and schematic-level writer orchestration.
- `simulation.rs` owns structured SPICE/simulation directive discovery,
  directive kind classification, directive text normalization, and the
  schematic edit helper for Schema-compatible simulation text items.
- `spice_export.rs` owns schematic-to-SPICE export, hierarchy expansion,
  SPICE include collection, symbol-to-device mapping, and hierarchy net alias
  scoping.
- `style.rs` owns reusable Schema stroke/fill/color/text-effects/margins IR,
  default text effects, style parse/write helpers, and style JSON projection
  helpers shared by schematic, symbol, canvas, and renderer paths.
- `coordinates.rs` owns reusable Schema point/size/at IR, coordinate parse
  helpers, point-list S-expression writer helpers, and coordinate JSON
  projection helpers used across parser, canvas, edit, and geometry paths.
- `graphics.rs` owns shared symbol/schematic drawing primitive IR, symbol
  graphic metadata, schematic graphic and rule-area IR, drawing bounds, canvas
  projection helpers, and graphic/rule-area parse/write helpers.
- `image.rs` owns schematic image IR, embedded base64 data parse/write, MIME
  sniffing, and PNG size metadata used by bounds and canvas projection.
- `wiring.rs` owns schematic wire, bus, bus-entry, bus-alias, and net-chain IR
  plus parse/write helpers for Schema line/net metadata roundtrip.
- `markers.rs` owns schematic junction and no-connect marker IR plus parse/write
  helpers for marker geometry metadata.
- `table.rs` owns schematic table/table-cell IR, border/separator metadata,
  rotated cell bounds, and table parse/write helpers.
- `property.rs` owns reusable Schema property IR and property parse/write helpers
  shared by symbols, sheets, labels, and library definitions.
- `pins.rs` owns symbol pin definitions, placed-symbol pin references, pin
  display metadata, pin alternates, pin parse/write helpers, pin JSON
  projection helpers, and pin ordering helpers used by library browser,
  canvas, and SPICE export paths.
- `symbols.rs` owns placed-symbol and symbol-definition IR, symbol inheritance,
  unit/body-style scoping, symbol parse/write helpers, library symbol
  qualification/lookup, symbol simulation metadata helpers, and placement
  property defaults.
- `labels.rs` owns schematic local/global/hierarchical labels and directive
  labels, label-kind formatting, and label parse/write helpers.
- `text.rs` owns schematic text/text-box IR, rotated text-box bounds, and
  text-item parse/write helpers.
- `group.rs` owns schematic group IR and group parse/write helpers.
- `sheet.rs` owns hierarchical sheet/sheet-pin IR, sheet bounds,
  Sheetname/Sheetfile default properties, and sheet parse/write helpers.
- `project.rs` owns `.schema_pro` project JSON parsing, project summary DTOs, and
  schematic stem candidate discovery used by Schema project import paths.
- `symbol_library.rs` owns `.schema_sym` library containers, `sym-lib-table`
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
- `sexpr.rs` owns the reusable Schema S-expression tree, parser, tree-navigation
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
