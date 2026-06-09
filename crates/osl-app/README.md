# osl-app

`osl-app` is the NekoSpice GUI alpha crate. It uses `eframe` with the `wgpu`
renderer selected explicitly, while KiCad parsing, editing, and symbol-library
search stay in `osl-kicad`.

## Module Boundaries

- `lib.rs`: public crate entry points and shared fixture defaults.
- `app.rs`: application state, event routing, canvas input handling, and native startup.
- `app/panels.rs`: toolbar, side panels, library browser, and `eframe::App` layout.
- `document.rs`: editable KiCad schematic adapter around `KicadSchematicEdit`.
- `library.rs`: GUI-facing symbol-library table adapter around `KicadSymbolLibraryIndex`.
- `viewport.rs`: world/screen transforms, zoom, pan, fit-to-scene, and culling bounds.
- `canvas.rs`: current egui painter prototype for schematic canvas drawing.

## Rules

- UI code may call document/library adapters, but should not parse KiCad files directly.
- KiCad geometry, hit testing, and edit semantics belong in `osl-kicad`; GUI code consumes those APIs.
- Canvas input handling belongs in `app.rs`; drawing primitives belong in `canvas.rs`.
- Future symbol placement should be added as a document/library adapter workflow first, then wired into UI state.
