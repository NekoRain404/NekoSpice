# osl-app

`osl-app` is the NekoSpice GUI alpha crate. It uses `eframe` with the `wgpu`
renderer selected explicitly, while KiCad parsing, editing, and symbol-library
search stay in `osl-kicad`.

## Module Boundaries

- `lib.rs`: public crate entry points and shared fixture defaults.
- `app.rs`: application state, document/library loading, and edit commands.
- `app/canvas_panel.rs`: canvas widget input, shortcuts, painter routing, and scene loading helper.
- `app/panels.rs`: toolbar, project/selection side panel, and `eframe::App` layout.
- `app/placement.rs`: symbol placement mode state, repeat placement, and post-edit selection refresh.
- `app/runtime.rs`: native window options, wgpu renderer selection, and initial egui style.
- `app/selection_properties.rs`: selected symbol property editor state sync and `KicadSchematicEdit::{SetSymbolProperty, ConfigureSymbol}` routing.
- `app/schematic_tools/mod.rs`: schematic tool UI, canvas click routing, and GUI calls into the document adapter.
- `app/schematic_tools/state.rs`: active tool state, pending wire/bus starts, sheet options, and other tool-local inputs.
- `app/schematic_tools/preview.rs`: transient canvas previews for active schematic drawing tools.
- `app/symbol_browser.rs`: symbol library browser, metadata details, and preview canvas.
- `app/symbol_placement_controls.rs`: unit, body-style, and pin-alternate controls for KiCad-compatible symbol placement.
- `document.rs`: editable KiCad schematic adapter around `KicadSchematicEdit`,
  including structured simulation directive updates for future GUI panels.
- `library.rs`: GUI-facing symbol-library table, definition, dependency, and preview adapter around `KicadSymbolLibraryIndex`.
- `placement_config.rs`: GUI-neutral symbol placement scope (`unit` / `body_style` / pin alternates) shared by preview and document edits.
- `viewport.rs`: world/screen transforms, zoom, pan, fit-to-scene, and culling bounds.
- `canvas.rs`: KiCad canvas scene traversal and draw-order routing.
- `canvas/primitives.rs`: current egui painter prototype for grid, shapes, graphics, and bounds.

## Rules

- UI code may call document/library adapters, but should not parse KiCad files directly.
- KiCad geometry, hit testing, and edit semantics belong in `osl-kicad`; GUI code consumes those APIs.
- Canvas input handling belongs in `app/canvas_panel.rs`; drawing primitives belong in `canvas/primitives.rs`.
- Selection property editing reads selected canvas metadata from `KicadCanvasScene` and writes only through `document.rs`.
- Symbol placement starts in the library/document adapters, then the UI wires selection, scope controls, preview, and canvas clicks to those adapters; repeat placement is UI state only, not KiCad file logic.
- Schematic drawing tools keep transient UI state under `app/schematic_tools/` and route all file mutations through `document.rs`, which in turn calls `KicadSchematicEdit`.
- Simulation setup UI should stay in GUI state and route KiCad-compatible directive changes through `document.rs`, not by editing text items directly.
