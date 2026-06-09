# NekoSpice

NekoSpice is a Rust-first SPICE automation tool that uses ngspice for circuit solving and Rust for repeatable runs, measurements, reports, and batch verification.

Schematic authoring is KiCad-compatible and Rust-native: NekoSpice is growing its own schematic and symbol-library subsystem around `.kicad_sch`, `.kicad_sym`, and `.kicad_pro` assets while keeping simulation automation, import diagnostics, waveform data, model checks, and CI-ready reports as the core differentiators.

The current three-day build is a vertical slice:

- `osl run`: run one `.cir` file through ngspice.
- `osl verify`: run a small YAML verification plan.
- `osl bench`: run every `.cir` under a directory and collect timings.
- `osl model-check`: scan imported SPICE models for `.subckt`, `.model`, LTspice symbol pin mapping, dialect risks, and unsupported directives.
- `osl import`: inspect SPICE/KiCad-style netlists, Rust-native KiCad schematics, and LTspice schematics, then generate an import compatibility report and runnable NekoSpice project.
- `osl kicad-inspect`: parse KiCad `.kicad_pro`, `.kicad_sch`, `.kicad_sym`, and `sym-lib-table` assets through the Rust-native KiCad IR and emit a JSON summary or symbol-library index.
- `osl kicad-check`: run Rust-native schematic diagnostics for KiCad symbol/net/SPICE readiness.
- `osl kicad-export`: write KiCad-compatible `.kicad_sch` and `.kicad_sym` files back from the Rust-native KiCad IR.
- `osl kicad-edit`: apply Rust-native schematic edit commands and write a KiCad-compatible `.kicad_sch`.
- `osl kicad-render`: render Rust-native KiCad schematic canvas scenes to SVG for headless visual review.
- `osl kicad-select`: hit-test a KiCad schematic canvas point through the Rust-native scene bounds and emit selectable item metadata.
- `osl waveform`: query raw waveforms into viewport-sized min/max envelope JSON.
- HTML and JSON reports for runs and verification batches.
- Run artifacts include `waveform.raw`, `waveform.csv`, and `waveform-summary.json`.
- Failure drilldown with failed checks, waveform summaries, parameters, logs, netlists, and waveform artifacts.
- Measurement checks over ngspice binary or ASCII `waveform.raw`: `final_value`, `avg`, `min`, `max`, `pp`, `rms`.

## Requirements

- Rust stable
- ngspice 46 or compatible ngspice CLI

## Quick Start

```bash
cargo run -p osl-cli -- --version
cargo run -p osl-cli -- run examples/rc_filter/rc.cir --output runs/rc_001
cargo run -p osl-cli -- verify examples/basic_validation.osl.yaml --jobs 3 --output reports/basic_001
cargo run -p osl-cli -- verify examples/structured_validation.osl.yaml --jobs 3 --output reports/structured_001
cargo run -p osl-cli -- bench examples --output bench-results/basic_001
cargo run -p osl-cli -- model-check examples/diode_rectifier/rectifier.cir --output reports/modelcheck_001
cargo run -p osl-cli -- model-check examples/pin_mapping/good_opamp.lib --symbol examples/pin_mapping/good_opamp.asy --output reports/pinmap_001
cargo run -p osl-cli -- import examples/kicad_import/kicad_rc.cir --output reports/import_001
cargo run -p osl-cli -- import examples/kicad_schematic/rc.kicad_sch --output reports/import_kicad_schematic_001
cargo run -p osl-cli -- import examples/kicad_hierarchical --output reports/import_kicad_hierarchical_001
cargo run -p osl-cli -- import examples/kicad_import/kicad_diode_include.cir --output reports/import_with_models_001
cargo run -p osl-cli -- kicad-inspect examples/kicad_schematic/rc.kicad_sch --output reports/kicad_schematic.json
cargo run -p osl-cli -- kicad-inspect examples/kicad_schematic/rc.kicad_sch --canvas --output reports/kicad_canvas_scene.json
cargo run -p osl-cli -- kicad-select examples/kicad_schematic/rc.kicad_sch 88.9,50.8 --output reports/kicad_canvas_hits.json
cargo run -p osl-cli -- kicad-check examples/kicad_schematic/rc.kicad_sch --output reports/kicad_check.json
cargo run -p osl-cli -- kicad-check examples/kicad_hierarchical/kicad_hierarchical.kicad_sch --output reports/kicad_hierarchical_check.json
cargo run -p osl-cli -- kicad-edit examples/kicad_schematic/rc.kicad_sch --library examples/kicad_schematic/neko_spice.kicad_sym --output reports/rc_edited.kicad_sch place-symbol:NekoSpice:C:C2:47n:101.6,53.34 'add-wire:101.6,50.8;88.9,50.8' 'add-wire:101.6,55.88;88.9,55.88' 'add-junction:88.9,50.8' 'add-text:.save v(out):45.72,35.56'
cargo run -p osl-cli -- kicad-edit examples/kicad_schematic/rc.kicad_sch --output reports/rc_with_bus.kicad_sch 'add-bus:88.9,38.1;101.6,38.1' 'add-bus-entry:101.6,38.1:2.54,-2.54'
cargo run -p osl-cli -- kicad-edit examples/kicad_schematic/rc.kicad_sch --library examples/kicad_schematic/neko_spice.kicad_sym --output reports/rc_with_no_connect.kicad_sch place-symbol:NekoSpice:R:R2:10k:101.6,50.8 'add-wire:88.9,50.8;99.06,50.8' 'add-no-connect:104.14,50.8'
cargo run -p osl-cli -- kicad-edit examples/kicad_schematic/rc.kicad_sch --output reports/rc_with_sheet.kicad_sch 'add-sheet:gain_stage:gain_stage.kicad_sch:101.6,43.18:25.4,12.7:in@101.6,48.26,180,input;out@127,48.26,0,output'
cargo run -p osl-cli -- kicad-render examples/kicad_schematic/rc.kicad_sch --output reports/kicad_canvas_scene.svg
cargo run -p osl-cli -- kicad-render examples/kicad_schematic/neko_spice.kicad_sym --symbol R --output reports/kicad_symbol_preview.svg
cargo run -p osl-cli -- kicad-inspect examples/kicad_schematic/neko_spice.kicad_sym --output reports/kicad_symbol_library.json
cargo run -p osl-cli -- kicad-inspect examples/kicad_schematic/sym-lib-table --index --output reports/kicad_symbol_index.json
cargo run -p osl-cli -- kicad-inspect examples/kicad_schematic/sym-lib-table --index --query R --output reports/kicad_symbol_search.json
cargo run -p osl-cli -- kicad-export examples/kicad_schematic/rc.kicad_sch --output reports/rc_roundtrip.kicad_sch
cargo run -p osl-cli -- kicad-export examples/kicad_schematic/neko_spice.kicad_sym --output reports/neko_spice_roundtrip.kicad_sym
cargo run -p osl-cli -- waveform runs/rc_001/waveform.raw --signal v(out) --from 8us --to 10us --points 200 --output reports/vout-envelope.json
```

## Verification Config

```yaml
project: basic_validation

runs:
  - name: rc_filter
    netlist: rc_filter/rc.cir
    sweep:
      rload: [500, 1000, 2000]
    checks:
      - name: average_output
        kind: avg
        signal: v(out)
        from: 8us
        to: 10us
        min: 0.45
        max: 0.50
```

Each sweep dimension expands into a Cartesian product of ngspice runs. `--jobs <n>` runs independent cases concurrently. Parameters are injected as `.param` overrides in the working netlist and recorded in `run.json` and `verify.json`; reports are sorted by the original expansion order. Checks can use optional `from` / `to` windows with SPICE-style suffixes such as `8us`, `3ms`, or `1k`. Verification reports include a compact summary for each evaluated signal window: sample count, first/last value, min/max, average, peak-to-peak, and RMS.

Verification files are parsed with `serde_yaml`, so normal YAML forms such as quoted strings, flow-style maps/lists, and numeric values with SPICE suffix strings are accepted.

The ngspice runner automatically injects a binary raw export into the working netlist:

```spice
set filetype=binary
write waveform.raw all
```

Checks can target any signal present in the raw variable table, such as `v(out)` or `i(v1)`. The waveform reader auto-detects ngspice `Binary:` and `Values:` raw files, so older ASCII artifacts remain readable. Every successful run exports `waveform.csv` for external tools and `waveform-summary.json` with per-signal sample count, first/last, min/max, average, peak-to-peak, and RMS.

For UI and plotting workflows, `osl waveform <waveform.raw> --signal <name>` returns min/max envelope buckets for a requested viewport. Use `--from`, `--to`, and `--points` to control the time window and target bucket count.

## Model Check

```bash
cargo run -p osl-cli -- model-check examples/vendor_model_issues --output /tmp/nekospice_modelcheck/bad
cargo run -p osl-cli -- model-check examples/pin_mapping/good_opamp.lib --symbol examples/pin_mapping/bad_opamp.asy --output /tmp/nekospice_modelcheck/pinmap_bad
```

`model-check` writes `model-check.json` and `report.html`. It extracts `.subckt` names and pin lists, indexes `.model` statements, flags unsupported or dialect-specific directives, and returns exit code `2` when error-level diagnostics are found. With `--symbol <ltspice.asy>`, it parses LTspice `PINATTR PinName` / `SpiceOrder` entries and verifies that symbol pin order matches the target `.subckt` pin list.

## Import Report

```bash
cargo run -p osl-cli -- import examples/kicad_import/kicad_rc.cir --output /tmp/nekospice_import/kicad_rc
cargo run -p osl-cli -- import examples/kicad_project --output /tmp/nekospice_import/kicad_project_dir
cargo run -p osl-cli -- import examples/kicad_project/kicad_project.kicad_pro --output /tmp/nekospice_import/kicad_project_file
cargo run -p osl-cli -- import examples/kicad_hierarchical --output /tmp/nekospice_import/kicad_hierarchical
cargo run -p osl-cli -- import examples/ltspice_import/ltspice_rc.asc --output /tmp/nekospice_import/ltspice_rc
cargo run -p osl-cli -- verify /tmp/nekospice_import/kicad_rc/project/project.osl.yaml --output /tmp/nekospice_import/kicad_rc_verify
```

`import` writes `import.json`, `report.html`, and a normalized `project/` directory. The project contains `input.cir`, `project.osl.yaml`, and `manifest.json`, so imported KiCad/LTspice/generic SPICE netlists can be handed directly to `osl verify`. KiCad/generic SPICE netlists are normalized directly. KiCad project directories and `.kicad_pro` files first resolve a native `.kicad_sch`, using `.kicad_pro` metadata and file stems to prefer the root schematic when multiple sheets exist, load local `sym-lib-table` / `.kicad_sym` symbol definitions when needed, and generate SPICE from the Rust KiCad IR; if no schematic is present, import falls back to a discovered exported SPICE netlist (`.cir`, `.spice`, or `.sp`). KiCad schematic diagnostics are carried into the import report. Hierarchical sheets are expanded by loading child `.kicad_sch` files and mapping parent sheet pins to child `hierarchical_label` nets; missing child sheets, cycles, or unmapped sheet pins produce explicit diagnostics instead of silent netlist loss. LTspice `.asc` schematics have a first-pass importer for `WIRE`, `FLAG`, `TEXT ... !<directive>`, local and searched `.asy` pin mapping, subcircuit symbols with `Prefix X`, and common primitive fallback symbols (`res`, `cap`, `ind`, `voltage`, `current`, diode-family, BJT, MOSFET, JFET, controlled-source, and switch symbols). Symbol search checks the schematic directory, `sym/` below it, `NEKOSPICE_LTSPICE_SYM_PATH`, and common LTspice installation paths. Unsupported symbols are reported with line-level diagnostics instead of silently producing a broken netlist. Relative `.include`, `.inc`, and `.lib` dependencies are copied into `project/models/` and referenced from the normalized netlist. The generated validation file keeps `checks: []` for a smoke run, then adds commented check templates derived from observable node voltages and voltage-source currents. The manifest stores the same `suggested_signals` and `suggested_checks` as machine-readable JSON for future GUI/project tooling. The compatibility report counts components, symbols, directives, includes, and emits diagnostics before the netlist is handed to ngspice.

## KiCad Schematic And Library IR

```bash
cargo run -p osl-cli -- kicad-inspect examples/kicad_schematic/rc.kicad_sch
cargo run -p osl-cli -- kicad-inspect examples/kicad_schematic/rc.kicad_sch --canvas
cargo run -p osl-cli -- kicad-inspect examples/kicad_project_schematic/kicad_project_schematic.kicad_pro
cargo run -p osl-cli -- kicad-check examples/kicad_schematic/rc.kicad_sch --output /tmp/nekospice_import/kicad_check.json
cargo run -p osl-cli -- kicad-select examples/kicad_schematic/rc.kicad_sch 88.9,50.8 --output /tmp/nekospice_import/kicad_canvas_hits.json
cargo run -p osl-cli -- kicad-edit examples/kicad_schematic/rc.kicad_sch --library examples/kicad_schematic/neko_spice.kicad_sym --output /tmp/nekospice_import/rc_edited.kicad_sch place-symbol:NekoSpice:C:C2:47n:101.6,53.34:unit=1 'add-wire:101.6,50.8;88.9,50.8' 'add-wire:101.6,55.88;88.9,55.88' 'add-junction:88.9,50.8' 'add-global-label:sense:101.6,50.8' 'add-text:.save v(out):45.72,35.56'
cargo run -p osl-cli -- kicad-edit examples/kicad_schematic/rc.kicad_sch --library examples/kicad_schematic/neko_spice.kicad_sym --output /tmp/nekospice_import/rc_configured.kicad_sch 'configure-symbol:R1:unit=1:mirror=xy'
cargo run -p osl-cli -- kicad-edit examples/kicad_schematic/rc.kicad_sch --output /tmp/nekospice_import/rc_deleted_wire.kicad_sch 'delete-item:22222222-2222-2222-2222-222222222222'
cargo run -p osl-cli -- kicad-edit examples/kicad_schematic/rc.kicad_sch --output /tmp/nekospice_import/rc_moved_wire.kicad_sch 'move-item:22222222-2222-2222-2222-222222222222:2.54,0'
cargo run -p osl-cli -- kicad-edit examples/kicad_schematic/rc.kicad_sch --output /tmp/nekospice_import/rc_with_bus.kicad_sch 'add-bus:88.9,38.1;101.6,38.1' 'add-bus-entry:101.6,38.1:2.54,-2.54'
cargo run -p osl-cli -- kicad-edit examples/kicad_schematic/rc.kicad_sch --library examples/kicad_schematic/neko_spice.kicad_sym --output /tmp/nekospice_import/rc_with_no_connect.kicad_sch place-symbol:NekoSpice:R:R2:10k:101.6,50.8 'add-wire:88.9,50.8;99.06,50.8' 'add-no-connect:104.14,50.8'
cargo run -p osl-cli -- kicad-edit examples/kicad_schematic/rc.kicad_sch --output /tmp/nekospice_import/rc_with_sheet.kicad_sch 'add-sheet:gain_stage:gain_stage.kicad_sch:101.6,43.18:25.4,12.7:in@101.6,48.26,180,input;out@127,48.26,0,output'
cargo run -p osl-cli -- kicad-render examples/kicad_schematic/rc.kicad_sch --output /tmp/nekospice_import/kicad_canvas_scene.svg
cargo run -p osl-cli -- kicad-inspect examples/kicad_schematic/neko_spice.kicad_sym
cargo run -p osl-cli -- kicad-inspect examples/kicad_schematic/sym-lib-table --index
cargo run -p osl-cli -- kicad-export examples/kicad_schematic/rc.kicad_sch --output /tmp/nekospice_import/rc_roundtrip.kicad_sch
cargo run -p osl-cli -- kicad-export examples/kicad_schematic/neko_spice.kicad_sym --output /tmp/nekospice_import/neko_spice_roundtrip.kicad_sym
```

`osl-kicad` is the Rust-native KiCad-compatible foundation. It parses KiCad S-expression and project JSON assets into schematic, project, and symbol-library IR, covering `.kicad_pro` metadata, sheet summaries, schematic file metadata (`generator_version`, `title_block`, sheet/symbol instance tables, embedded project instances, variant instance metadata, and `embedded_fonts`), schematic symbols, embedded and external project library symbols, hierarchical sheet items and sheet pins with sheet box stroke/fill metadata, one-level/recursive child sheet netlist expansion, wires, buses, and bus entries with stroke metadata, bus aliases, net-chain metadata (`from`/`to`, `net_class`, `color`, member nets, and legacy/unknown child preservation), labels, netclass/directive labels with length/shape/properties, rule areas with polyline/stroke/fill/assembly flags, text/SPICE directives, text boxes with stroke/fill/locked metadata, embedded schematic images, schematic tables and table cells with border/separator/fill/effects metadata, schematic groups and member references, junctions with diameter/color metadata, no-connect markers, schematic-level drawing primitives (`polyline`, `bezier`, `rectangle`, `circle`, `arc`) with stroke/fill/locked metadata, symbol, sheet, and label assembly/display metadata (`in_bom`, `on_board`, `dnp`, `fields_autoplaced`, label `shape`, symbol `mirror`), symbol properties, label properties, and canvas text items with display flags and text effects, pins, symbol library file metadata (`generator_version`), symbol definition flags (`power`, `in_bom`, `on_board`, `in_pos_files`, `duplicate_pin_numbers_are_jumpers`, `embedded_fonts`), symbol inheritance/body-style/jumper metadata (`extends`, `body_styles`, `jumper_pin_groups`), symbol pin display settings plus pin name/number text effects and alternate pin definitions, symbol graphics including Bezier curves plus stroke/fill/private/UUID/locked metadata, symbol bounding boxes, symbol library tables, a symbol library index for later GUI library browsing and schematic symbol resolution, a schematic canvas scene with transformed symbol graphics and schematic drawing primitives carrying their KiCad stroke/fill metadata, styled sheet boxes and sheet pins, netclass/directive labels, rule areas, schematic-level drawing primitives including Bezier curves, embedded images, tables/table cells, wires, buses, bus entries, labels, text annotations/SPICE directive text, text boxes, junctions, no-connect markers, and scene bounds, schematic diagnostics for duplicate references, missing symbol definitions, missing values, unconnected/generated nets, malformed bus geometry, floating labels, floating no-connect markers, missing ground, invalid simulation pin mappings, missing child sheets, sheet hierarchy cycles, unsupported simulation devices, and missing analysis directives, KiCad simulation fields (`Sim.Device`, `Sim.Pins`, `Sim.Params`, `Sim.Library`, `Sim.Name`) plus legacy `Spice_*` field handling for schematic-to-SPICE export, `exclude_from_sim` preservation, schematic edit commands for moving symbols, moving top-level canvas items by UUID, setting symbol properties, placing symbols from `.kicad_sym` libraries, adding wires, adding buses and bus entries, adding junctions, adding no-connect markers, adding labels, adding hierarchical sheet boxes and sheet pins, and adding text/SPICE directives, and `.kicad_sch` / `.kicad_sym` writer support for asset roundtrips. `osl-render` consumes the same canvas scene and produces SVG as a deterministic headless rendering baseline before the later wgpu/GUI renderer, including multiline schematic text, highlighted SPICE directives, KiCad styled and rotated text boxes, styled sheet boxes, netclass/directive labels, rule areas, Bezier drawing paths, embedded KiCad images as data URIs, styled schematic/symbol graphics, styled wires/buses/junctions, and KiCad styled and rotated tables/table cells. Schematic roundtrips preserve paper size, schematic UUID, title block fields/comments, generator version, bus aliases, net-chain metadata and legacy child records, wire/bus/bus-entry/label/text/text-box/image/table/table-cell/group/junction/no-connect/netclass-flag/rule-area/schematic-graphic UUIDs, wire/bus/bus-entry stroke settings, hierarchical sheet stroke/fill settings, schematic graphic stroke/fill/locked settings, rule area polyline/stroke/fill/assembly flags, text box stroke/fill/locked settings, table border/separator settings, table cell fill/effects/locked settings, junction diameter/color settings, group member UUID references, hierarchical sheet UUIDs, sheet pin UUIDs, sheet/symbol instance tables, embedded symbol/sheet project path instances, variant DNP metadata, symbol/sheet BOM/board/DNP/autoplace flags, label shape/autoplace flags, netclass/directive label length/shape/autoplace flags and label/directive properties, symbol library generator version, symbol definition power/BOM/board/position/jumper/embedded-font flags, symbol inheritance/body-style/jumper groups, symbol graphics stroke/fill/private/UUID/locked metadata, symbol pin display/name/number text metadata and alternate pin definitions, symbol mirror settings, property IDs, property hide/show-name/autoplace flags, property and canvas text font size/thickness/bold/italic/color, justification, hide flags, and hrefs, symbol instance UUIDs, symbol pin number/UUID pairs and selected alternate names, embedded font settings, simulation exclusion flags, and supported symbol simulation fields. The local KiCad source mirror is treated only as reference material and is ignored by Git.

`osl kicad-inspect <file.kicad_sch> --canvas` emits full canvas scene JSON for downstream GUI work, including KiCad UUIDs and object-level bounds for selectable top-level objects, symbols, transformed symbol graphics, pins with alternate/text-effect metadata, sheet boxes and sheet pins, schematic graphics, embedded images, tables/table cells, rule areas, wires, buses, bus entries, directive labels with properties, labels, text items, text boxes, junctions, no-connect markers, group metadata/member UUID references, KiCad stroke/fill/text effects, margins, counts, and scene bounds. These per-item bounds are selection/hit-test metadata for the later Rust GUI and command routing. `KicadCanvasScene::to_summary_json()` remains available for compact counts.

`osl kicad-select <file.kicad_sch> <x,y>` runs the same Rust-native canvas scene through hit-testing and returns stable JSON for GUI selection. Hits include kind, KiCad UUID, label, bounds, and area, sorted from smaller bounds to larger bounds with deterministic tie-breakers. Wire, bus, bus-entry, rule areas, text boxes, tables/table cells, and schematic drawing primitives use geometry-aware hit-testing after a bounds prefilter: line-like objects use segment distance, Bezier curves use sampled cubic segments, KiCad three-point arcs use sampled circular arcs, hollow rectangles/circles/polygons hit their outlines, filled rectangles/circles/polygons hit their interior, rotated text boxes and table cells hit their rotated rectangles, and labels/text/directive labels use estimated text boxes from text effects and justification. Other selectable objects currently use object-level bounds. This is the command-routing bridge for future select/move/delete workflows on `.kicad_sch` assets while exact glyph metrics are added later.

Symbol graphic text items now preserve KiCad text effects through Rust IR, `.kicad_sym` / `.kicad_sch` roundtrips, canvas scene generation, and the SVG baseline renderer, including font size, thickness, bold/italic, color, justification, hide flags, and href metadata.

Multi-unit and body-style symbol data is tracked from KiCad nested symbol names and instance `(unit ...)` / `(body_style ...)` metadata. Canvas scenes, schematic-to-SPICE pin selection, no-connect/connection diagnostics, placement pin UUID generation, and KiCad writers use that scope so a placed unit does not render, connect, or satisfy checks using pins from other units. `osl kicad-edit ... place-symbol:<lib_id>:<reference>:<value>:<x,y[,rotation]>[:unit=<n>][:body-style=<n>][:alt=<pin>=<alternate>[,<pin>=<alternate>...]]` preserves the same scope and selected pin alternates chosen by the symbol-library index and preview path. Existing placed symbols can be reconfigured with `configure-symbol:<reference>[:unit=<n>][:body-style=<n|none>][:mirror=<x|y|xy|none>][:alt=<pin>=<alternate>[,<pin>=<alternate>...]]`; the Rust edit path rebuilds instance pin refs for the selected scope, validates alternate names, preserves reusable pin UUIDs, writes KiCad-compatible `(mirror x y)` metadata, and applies symbol mirroring in the canvas/connectivity transform.

`osl kicad-edit ... delete-item:<uuid>` removes schematic canvas items by KiCad UUID, including symbols, wires, buses, bus entries, junctions, no-connect markers, labels, directive labels, text, sheets, sheet pins, drawing graphics, rule areas, images, tables, table cells, and groups, giving the future GUI a stable Rust-native deletion path without touching shared library symbol definitions.

`osl kicad-edit ... move-item:<uuid>:<dx,dy>` translates schematic geometry by KiCad UUID, including placed symbols with their properties, wires, buses, bus entries, junctions, no-connect markers, labels, directive labels and their properties, text, sheets with sheet properties and pins, individual sheet pins, drawing graphics, rule areas, images, tables, and individual table cells returned by `kicad-select`. Group records remain metadata-only; move the referenced member items for a geometry change.

KiCad symbol unit display names from nested `(unit_name ...)` records are preserved in the Rust IR, schematic and symbol-library summaries, canvas symbol metadata, and `.kicad_sch` / `.kicad_sym` writers so multi-unit parts keep their user-facing unit labels for later library browser and placement UI work.

The symbol library index now carries browser-oriented metadata for each symbol, including KiCad `Description` / legacy `ki_description`, `ki_keywords`, decoded `ki_fp_filters`, unit count, unit display names, body-style names, pin electrical/shape metadata, pin alternate choices, inheritance parent, inherited browser metadata for derived symbols, resolved bounding boxes, and power-symbol kind, so later placement UI can search, filter by footprint, and choose the correct unit/body-style/pin alternate without reparsing the library file.

`osl kicad-inspect <sym-lib-table> --index` emits the full Rust-native symbol index JSON, including loaded libraries, searchable symbols, unit records, footprint filters, resolved bounding boxes, and diagnostics, while retaining top-level count fields for quick CI checks. Add `--query <text>`, `--library <nickname>`, and `--footprint <footprint>` to get the same index shape filtered for library-browser search and footprint-compatible placement.

`osl kicad-render <file.kicad_sym> --symbol <name>` renders a single library symbol through the same Rust canvas scene and SVG renderer used by schematics. Optional `--unit <n>` and `--body-style <n>` select the preview scope for multi-unit/body-style symbols, giving the future library browser a deterministic headless preview path.

KiCad symbol inheritance via `.kicad_sym` `(extends ...)` is resolved in the Rust IR at use time. Canvas scene generation, schematic-to-SPICE pin selection, simulation-field lookup, and symbol placement use inherited parent graphics, pins, pin display settings, and default simulation properties while writers keep the KiCad-derived symbol shape instead of flattening parent items back into the child symbol.

## Validation

```bash
cargo fmt --check
cargo test --workspace
cargo run -p osl-cli -- verify examples/basic_validation.osl.yaml --jobs 3 --output /tmp/nekospice_reports/basic
cargo run -p osl-cli -- verify examples/structured_validation.osl.yaml --jobs 3 --output /tmp/nekospice_reports/structured
cargo run -p osl-cli -- import examples/kicad_import/kicad_rc.cir --output /tmp/nekospice_import/kicad_rc
cargo run -p osl-cli -- verify /tmp/nekospice_import/kicad_rc/project/project.osl.yaml --output /tmp/nekospice_import/kicad_rc_verify
cargo run -p osl-cli -- import examples/kicad_project --output /tmp/nekospice_import/kicad_project_dir
cargo run -p osl-cli -- verify /tmp/nekospice_import/kicad_project_dir/project/project.osl.yaml --output /tmp/nekospice_import/kicad_project_dir_verify
cargo run -p osl-cli -- import examples/kicad_project/kicad_project.kicad_pro --output /tmp/nekospice_import/kicad_project_file
cargo run -p osl-cli -- verify /tmp/nekospice_import/kicad_project_file/project/project.osl.yaml --output /tmp/nekospice_import/kicad_project_file_verify
cargo run -p osl-cli -- import examples/kicad_schematic/rc.kicad_sch --output /tmp/nekospice_import/kicad_schematic
cargo run -p osl-cli -- verify /tmp/nekospice_import/kicad_schematic/project/project.osl.yaml --output /tmp/nekospice_import/kicad_schematic_verify
cargo run -p osl-cli -- import examples/kicad_hierarchical --output /tmp/nekospice_import/kicad_hierarchical
cargo run -p osl-cli -- verify /tmp/nekospice_import/kicad_hierarchical/project/project.osl.yaml --output /tmp/nekospice_import/kicad_hierarchical_verify
cargo run -p osl-cli -- kicad-inspect examples/kicad_schematic/rc.kicad_sch --output /tmp/nekospice_import/kicad_schematic.json
cargo run -p osl-cli -- kicad-inspect examples/kicad_schematic/rc.kicad_sch --canvas --output /tmp/nekospice_import/kicad_canvas_scene.json
cargo run -p osl-cli -- kicad-select examples/kicad_schematic/rc.kicad_sch 88.9,50.8 --output /tmp/nekospice_import/kicad_canvas_hits.json
cargo run -p osl-cli -- kicad-check examples/kicad_schematic/rc.kicad_sch --output /tmp/nekospice_import/kicad_check.json
cargo run -p osl-cli -- kicad-check examples/kicad_hierarchical/kicad_hierarchical.kicad_sch --output /tmp/nekospice_import/kicad_hierarchical_check.json
cargo run -p osl-cli -- kicad-edit examples/kicad_schematic/rc.kicad_sch --library examples/kicad_schematic/neko_spice.kicad_sym --output /tmp/nekospice_import/rc_edited.kicad_sch place-symbol:NekoSpice:C:C2:47n:101.6,53.34 'add-wire:101.6,50.8;88.9,50.8' 'add-wire:101.6,55.88;88.9,55.88' 'add-junction:88.9,50.8' 'add-global-label:sense:101.6,50.8' 'add-text:.save v(out):45.72,35.56'
cargo run -p osl-cli -- kicad-edit examples/kicad_schematic/rc.kicad_sch --output /tmp/nekospice_import/rc_with_bus.kicad_sch 'add-bus:88.9,38.1;101.6,38.1' 'add-bus-entry:101.6,38.1:2.54,-2.54'
cargo run -p osl-cli -- kicad-edit examples/kicad_schematic/rc.kicad_sch --library examples/kicad_schematic/neko_spice.kicad_sym --output /tmp/nekospice_import/rc_with_no_connect.kicad_sch place-symbol:NekoSpice:R:R2:10k:101.6,50.8 'add-wire:88.9,50.8;99.06,50.8' 'add-no-connect:104.14,50.8'
cargo run -p osl-cli -- kicad-edit examples/kicad_schematic/rc.kicad_sch --output /tmp/nekospice_import/rc_with_sheet.kicad_sch 'add-sheet:gain_stage:gain_stage.kicad_sch:101.6,43.18:25.4,12.7:in@101.6,48.26,180,input;out@127,48.26,0,output'
cargo run -p osl-cli -- import /tmp/nekospice_import/rc_edited.kicad_sch --output /tmp/nekospice_import/rc_edited_import
cargo run -p osl-cli -- verify /tmp/nekospice_import/rc_edited_import/project/project.osl.yaml --output /tmp/nekospice_import/rc_edited_verify
cargo run -p osl-cli -- kicad-render examples/kicad_schematic/rc.kicad_sch --output /tmp/nekospice_import/kicad_canvas_scene.svg
cargo run -p osl-cli -- kicad-inspect examples/kicad_schematic/neko_spice.kicad_sym --output /tmp/nekospice_import/kicad_symbol_library.json
cargo run -p osl-cli -- kicad-inspect examples/kicad_schematic/sym-lib-table --index --output /tmp/nekospice_import/kicad_symbol_index.json
cargo run -p osl-cli -- kicad-export examples/kicad_schematic/rc.kicad_sch --output /tmp/nekospice_import/rc_roundtrip.kicad_sch
cargo run -p osl-cli -- kicad-inspect /tmp/nekospice_import/rc_roundtrip.kicad_sch --output /tmp/nekospice_import/rc_roundtrip.json
cargo run -p osl-cli -- import /tmp/nekospice_import/rc_roundtrip.kicad_sch --output /tmp/nekospice_import/rc_roundtrip_import
cargo run -p osl-cli -- verify /tmp/nekospice_import/rc_roundtrip_import/project/project.osl.yaml --output /tmp/nekospice_import/rc_roundtrip_verify
cargo run -p osl-cli -- kicad-export examples/kicad_schematic/neko_spice.kicad_sym --output /tmp/nekospice_import/neko_spice_roundtrip.kicad_sym
cargo run -p osl-cli -- kicad-inspect /tmp/nekospice_import/neko_spice_roundtrip.kicad_sym --output /tmp/nekospice_import/neko_spice_roundtrip.json
cargo run -p osl-cli -- import examples/ltspice_import/ltspice_rc.asc --output /tmp/nekospice_import/ltspice_rc
cargo run -p osl-cli -- verify /tmp/nekospice_import/ltspice_rc/project/project.osl.yaml --output /tmp/nekospice_import/ltspice_rc_verify
cargo run -p osl-cli -- import examples/kicad_import/kicad_diode_include.cir --output /tmp/nekospice_import/kicad_with_models
cargo run -p osl-cli -- verify /tmp/nekospice_import/kicad_with_models/project/project.osl.yaml --output /tmp/nekospice_import/kicad_with_models_verify
cargo run -p osl-cli -- waveform /tmp/nekospice_reports/basic/runs/rc_filter/waveform.raw --signal 'v(out)' --points 100 --output /tmp/nekospice_reports/basic/vout-envelope.json
bash -lc 'cargo run -p osl-cli -- verify examples/failing_validation.osl.yaml --output /tmp/nekospice_reports/failing; test $? -eq 2'
bash -lc 'cargo run -p osl-cli -- model-check examples/vendor_model_issues --output /tmp/nekospice_modelcheck/bad; test $? -eq 2'
bash -lc 'cargo run -p osl-cli -- model-check examples/pin_mapping/good_opamp.lib --symbol examples/pin_mapping/bad_opamp.asy --output /tmp/nekospice_modelcheck/pinmap_bad; test $? -eq 2'
```
