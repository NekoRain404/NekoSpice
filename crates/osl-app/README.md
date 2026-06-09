# osl-app

`osl-app` is the NekoSpice GUI alpha crate. It uses `eframe` with the `wgpu`
renderer selected explicitly, while KiCad parsing, editing, and symbol-library
search stay in `osl-kicad`.

## Module Boundaries

- `lib.rs`: public crate entry points and shared fixture defaults.
- `app.rs`: application state, document/library loading, and edit commands.
- `app/canvas_panel.rs`: canvas widget input, shortcuts, painter routing, and scene loading helper.
- `app/center_workspace.rs`: center workspace router. Home owns the dashboard;
  schematic-oriented workspaces keep using the framed canvas until their own
  center surfaces are split out.
- `app/home_dashboard.rs`: Home center workspace composition and high-level
  dashboard ordering.
- `app/home_insights_panel.rs`: Home right-side assistant, insights, and
  shortcut surfaces.
- `app/home_project_context.rs`: Home left-side project, schematic health, and
  library scope context cards.
- `app/home_sections.rs`: Home dashboard sections such as recent projects, quick
  actions, templates, queue, solver health, measurements, and recommendations.
- `app/home_widgets.rs`: Home-only responsive layout helpers and small display
  rows/cards. Shared widgets still belong in `app/widgets.rs`.
- `app/library_workspace.rs`: Library center workspace layout, header, search,
  and category tabs for KiCad-compatible symbols.
- `app/library_sections.rs`: Library symbol list, selected-symbol preview card,
  and generated SPICE model preview sections.
- `app/library_inspector.rs`: Library right-side status, selected-symbol detail,
  and validation summary cards.
- `app/library_preview.rs`: Library-only preview drawing for KiCad symbol canvas
  snapshots and compact SPICE stub listings.
- `app/library_data.rs`: GUI-facing library filter, search result, selected
  symbol snapshot, and SPICE preview helper data.
- `app/library_widgets.rs`: Library-only tabs, symbol rows, metadata rows, and
  code-line widgets.
- `app/navigation.rs`: Studio workspace tabs and labels for home, schematic,
  library, simulation, report, and settings contexts.
- `app/panels.rs`: Studio shell layout compositor only. It mounts the chrome
  regions and delegates all panel content to focused modules.
- `app/navigation_panel.rs`: left Studio workspace navigation and renderer /
  solver system summary.
- `app/project_panel.rs`: active schematic path, project health, selection, and
  edit command sidebar.
- `app/workspace_panel.rs`: left/right workspace context routers for home,
  schematic tools, library, simulation, reports, and settings contexts.
- `app/diagnostics_panel.rs`: reusable document diagnostic summary and scroll
  list for schematic-focused surfaces.
- `app/studio_toolbar.rs`: top action buttons and the framed canvas mounting
  helper used by the shell.
- `app/localization.rs`: Studio UI text keys and locale tables. New user-facing
  shell text should be added here instead of scattering string literals.
- `app/preferences.rs`: GUI preferences such as theme mode and locale, plus the
  app-facing helpers that expose text and palette data to panels.
- `app/placement.rs`: symbol placement mode state, repeat placement, and post-edit selection refresh.
- `app/runtime.rs`: native window options, wgpu renderer selection, and initial egui style.
- `app/selection_properties.rs`: selected symbol property editor state sync and `KicadSchematicEdit::{SetSymbolProperty, ConfigureSymbol}` routing.
- `app/simulation_artifacts_panel.rs`: run artifact drilldown for GUI
  simulation results, including artifact kind, file name, and size.
- `app/simulation_report_panel.rs`: GUI report summary panel for the generated
  `report.html` and its backing run/verify/bench/model/import JSON source.
- `app/simulation_panel.rs`: simulation directive editor, schematic diagnostics,
  SPICE netlist preview, and ngspice run controls routed through the document
  and simulation adapters.
- `app/simulation_waveform_panel.rs`: GUI-only waveform result panel for signal
  selection, compact measurement display, and preview-envelope drawing.
- `app/status_strip.rs`: Studio project, solver, diagnostics, selection, and
  waveform status summaries used by the shell chrome.
- `app/schematic_tools/mod.rs`: schematic tool module entry point and canvas
  preview delegation.
- `app/schematic_tools/controls.rs`: schematic tool selection and tool-local
  input widgets.
- `app/schematic_tools/editing.rs`: canvas click routing and GUI calls into the
  document adapter.
- `app/schematic_tools/state.rs`: active tool state, pending wire/bus starts, sheet options, and other tool-local inputs.
- `app/schematic_tools/preview.rs`: transient canvas previews for active schematic drawing tools.
- `app/schematic_inspector_panel.rs`: Schematic right workspace tab state and
  routing for properties, KiCad inspection, libraries, and simulator context.
- `app/schematic_inspector_sections.rs`: Schematic right workspace properties,
  cross-probe actions, KiCad document structure, diagnostics, and library scope
  sections.
- `app/schematic_inspector_simulator.rs`: compact right-side simulator status,
  run shortcuts, live-measurement summary, and handoff to the full Simulation
  workspace.
- `app/schematic_inspector_widgets.rs`: Schematic inspector-only tabs, property
  rows, status pills, action buttons, and captions.
- `app/schematic_workspace.rs`: Schematic center workspace chrome: document
  tabs, canvas toolbar, framed canvas mounting, and bottom waveform/console dock.
- `app/schematic_workspace_widgets.rs`: Schematic-only display widgets for tabs,
  toolbar buttons, signal rows, and console lines.
- `app/symbol_placement_controls.rs`: unit, body-style, and pin-alternate controls for KiCad-compatible symbol placement.
- `app/theme.rs`: Studio theme modes, palette tokens, style application, and
  frame/text helpers. Shared UI color and spacing decisions live here instead of
  being duplicated across panels.
- `app/widgets.rs`: small shared egui widgets that are visual only and carry no
  document or library behavior.
- `document.rs`: editable KiCad schematic adapter around `KicadSchematicEdit`,
  structured simulation directive updates, check reports, and netlist previews.
- `simulation.rs`: GUI-facing simulation run adapter that writes the current
  schematic netlist to a run directory and invokes `osl-sim` backends on a
  background worker, then finalizes shared run artifacts through `osl-sim`.
- `report_summary.rs`: GUI-facing report summary adapter around `osl-report`,
  keeping report directory scanning and fallback `report.html` generation out of
  panel drawing code.
- `waveform_summary.rs`: GUI-facing waveform summary adapter around
  `osl-waveform`, keeping raw parsing and preview-envelope generation out of
  panel drawing code.
- `library.rs`: GUI-facing symbol-library table, definition, dependency, and preview adapter around `KicadSymbolLibraryIndex`.
- `placement_config.rs`: GUI-neutral symbol placement scope (`unit` / `body_style` / pin alternates) shared by preview and document edits.
- `viewport.rs`: world/screen transforms, zoom, pan, fit-to-scene, and culling bounds.
- `canvas.rs`: KiCad canvas scene traversal and draw-order routing.
- `canvas/primitives.rs`: current egui painter prototype for grid, shapes, graphics, and bounds.

## Rules

- UI code may call document/library adapters, but should not parse KiCad files directly.
- `app/panels.rs` should stay a layout compositor; new business behavior belongs
  in focused panels/adapters, shared visual tokens belong in `app/theme.rs`,
  user-facing shell text belongs in `app/localization.rs`, and small reusable
  display widgets belong in `app/widgets.rs`.
- Theme changes should flow through `StudioThemeMode` / `StudioPalette`; panels
  should not duplicate color constants for Studio chrome.
- New shell text should use `NekoSpiceApp::text(UiText::...)`; deeper feature
  panels can migrate incrementally, but new Studio chrome should not add
  hard-coded English labels.
- KiCad geometry, hit testing, and edit semantics belong in `osl-kicad`; GUI code consumes those APIs.
- Canvas input handling belongs in `app/canvas_panel.rs`; drawing primitives belong in `canvas/primitives.rs`.
- Selection property editing reads selected canvas metadata from `KicadCanvasScene` and writes only through `document.rs`.
- Symbol placement starts in the library/document adapters, then the UI wires selection, scope controls, preview, and canvas clicks to those adapters; repeat placement is UI state only, not KiCad file logic.
- Schematic drawing tools keep transient UI state under `app/schematic_tools/` and route all file mutations through `document.rs`, which in turn calls `KicadSchematicEdit`.
- Simulation setup UI should stay in GUI state and route KiCad-compatible directive changes through `document.rs`, not by editing text items directly.
- Simulation execution should route through `simulation.rs` and `osl-sim`; UI
  widgets should not invoke simulator processes directly.
- Run artifact export, artifact classification, and refreshed `run.json`
  metadata should stay in `osl-sim` so CLI and GUI runs remain consistent.
  Single-run `report.html` generation belongs to the same shared artifact
  finalizer.
- Report panels should consume precomputed GUI DTOs from `report_summary.rs`;
  directory scanning and fallback report generation should stay in `osl-report`.
- Waveform panels should consume precomputed GUI DTOs from `waveform_summary.rs`
  so drawing code does not parse raw files or scan full waveform arrays.

## GUI Verification

- GUI changes should run `cargo fmt --check`, `cargo check --workspace`,
  `cargo clippy --workspace --all-targets -- -D warnings`, and
  `cargo test --workspace`.
- For visual changes, capture a smoke screenshot when the local desktop/session
  allows it. Store temporary screenshots under `target/ui-smoke/` so they stay
  outside source control.
- `NEKOSPICE_INITIAL_WORKSPACE=schematic` can be used by GUI smoke scripts to
  open a non-default workspace deterministically before capturing screenshots.
