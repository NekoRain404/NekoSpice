# osl-app

`osl-app` is the NekoSpice GUI alpha crate. It uses `eframe` with the `wgpu`
renderer selected explicitly, while KiCad parsing, editing, and symbol-library
search stay in `osl-kicad`.

## Module Boundaries

- `lib.rs`: public crate entry points and shared fixture defaults.
- `app.rs`: application state, document/library loading, and edit commands.
- `app/canvas_panel.rs`: canvas widget input, shortcuts, painter routing, and scene loading helper.
- `app/panels.rs`: toolbar, side panels, library browser, and `eframe::App` layout.
- `app/placement.rs`: symbol placement mode state, canvas placement routing, repeat placement, and post-edit selection refresh.
- `app/runtime.rs`: native window options, wgpu renderer selection, and initial egui style.
- `document.rs`: editable KiCad schematic adapter around `KicadSchematicEdit`.
- `library.rs`: GUI-facing symbol-library table adapter around `KicadSymbolLibraryIndex`.
- `viewport.rs`: world/screen transforms, zoom, pan, fit-to-scene, and culling bounds.
- `canvas.rs`: KiCad canvas scene traversal and draw-order routing.
- `canvas/primitives.rs`: current egui painter prototype for grid, shapes, graphics, and bounds.

## Rules

- UI code may call document/library adapters, but should not parse KiCad files directly.
- KiCad geometry, hit testing, and edit semantics belong in `osl-kicad`; GUI code consumes those APIs.
- Canvas input handling belongs in `app/canvas_panel.rs`; drawing primitives belong in `canvas/primitives.rs`.
- Symbol placement starts in the library/document adapters, then the UI wires selection and canvas clicks to those adapters; repeat placement is UI state only, not KiCad file logic.
