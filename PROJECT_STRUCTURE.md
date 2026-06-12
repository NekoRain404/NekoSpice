# NekoSpice Project Structure

## Root

```
NekoSpice/
├── Cargo.toml              # Workspace root: 10 crates, Rust 2024 edition
├── Cargo.lock
├── README.md               # Project overview and CLI usage
├── PROJECT_STRUCTURE.md    # This file
├── .gitignore
├── crates/                 # All Rust source code
├── examples/               # Test fixtures and demo projects
├── docs/                   # Documentation and UI reference
├── runs/                   # Simulation run outputs (gitignored)
└── benchmarks/             # Benchmark configurations
```

## Crates

### Core

| Crate | Description |
|-------|-------------|
| `osl-core` | Shared types, error handling, and utility functions used across all crates |
| `osl-kicad` | KiCad `.kicad_sch` / `.kicad_sym` / `.kicad_pro` parser, IR, canvas scene builder, edit operations, SPICE export, and hit testing |

### Simulation

| Crate | Description |
|-------|-------------|
| `osl-sim` | ngspice and Xyce backend runners, simulation directives, and run artifact collection |
| `osl-netlist` | Netlist parsing (KiCad/SPICE/LTspice), import compatibility diagnostics, and signal suggestion engine |
| `osl-model` | SPICE model checking: `.subckt`/`.model` validation, LTspice symbol pin mapping, dialect risk detection |
| `osl-waveform` | Raw waveform data parser (`.raw` files) for post-simulation analysis |

### Rendering

| Crate | Description |
|-------|-------------|
| `osl-render` | SVG renderer for KiCad schematic canvas scenes (headless visual review) |

### Reporting

| Crate | Description |
|-------|-------------|
| `osl-report` | Verification report generation in HTML, JSON, JUnit XML, and Markdown formats |

### Interfaces

| Crate | Description |
|-------|-------------|
| `osl-cli` | CLI binary (`osl`): run, verify, bench, model-check, import, kicad-inspect/check/export/edit/render/select |
| `osl-app` | GUI binary (`nekospice`): hardware-accelerated egui/wgpu application with KiCad schematic editor |

## `osl-app` Internal Structure

```
osl-app/src/
├── lib.rs                    # Crate root, DEFAULT_SCHEMATIC constants
├── main.rs                   # Entry point
├── app.rs                    # NekoSpiceApp struct, Default impl, core actions
├── app/
│   ├── panels.rs             # Root layout: top/bottom bars, nav, context, workspace
│   ├── runtime.rs            # eframe::App impl, native window launch
│   ├── navigation.rs         # StudioWorkspace enum (Home/Schematic/Library/Simulation/...)
│   ├── navigation_panel.rs   # Left sidebar workspace switcher with icons
│   ├── theme.rs              # StudioTheme: Midnight/Graphite/Light palettes
│   ├── localization.rs       # UiText enum for i18n (en/zh_hans)
│   ├── localization_en_impl.rs
│   ├── localization_zh_impl.rs
│   │
│   ├── canvas_panel.rs       # Main canvas: viewport, mouse interaction, rendering
│   ├── canvas_shortcuts.rs   # Keyboard shortcuts (V/W/L/B/S/J/Q/R/F/Del/Esc)
│   ├── canvas_context_menu.rs # Right-click context menu with tool switching
│   ├── tool_palette.rs       # Vertical tool palette (left of canvas)
│   ├── shortcuts_overlay.rs  # Keyboard shortcut help overlay (? key)
│   │
│   ├── schematic_workspace.rs      # Schematic view: toolbar, tabs, canvas, inspector
│   ├── schematic_workspace_widgets.rs # Toolbar buttons, document tabs, signal rows
│   ├── schematic_bottom_dock.rs    # Bottom dock: Waveforms/FFT/Bode/Console/Netlist/ERC
│   ├── schematic_inspector_panel.rs # Right-side inspector: properties/libraries/simulator
│   ├── schematic_inspector_sections.rs
│   ├── schematic_inspector_simulator.rs
│   ├── schematic_inspector_widgets.rs
│   ├── schematic_review_panel.rs
│   ├── schematic_tools/            # Drawing tool state machine
│   │   ├── mod.rs
│   │   ├── state.rs                # SchematicTool enum and state
│   │   ├── controls.rs             # Tool activation and switching
│   │   ├── editing.rs              # Wire/bus/label/sheet creation
│   │   └── preview.rs              # Tool preview rendering
│   │
│   ├── home_dashboard.rs           # Home workspace dashboard
│   ├── home_command_center.rs
│   ├── home_insights_panel.rs
│   ├── home_project_context.rs
│   ├── home_sections.rs
│   ├── home_widgets.rs
│   │
│   ├── library_workspace.rs        # Symbol library browser
│   ├── library_model_browser.rs
│   ├── library_model_validation.rs
│   ├── library_preview.rs
│   ├── library_sections.rs
│   ├── library_widgets.rs
│   ├── library_data.rs
│   ├── library_inspector.rs
│   │
│   ├── simulation_workspace.rs     # Simulation configuration and run
│   ├── simulation_workspace_sections.rs
│   ├── simulation_workspace_widgets.rs
│   ├── simulation_panel.rs
│   ├── simulation_profile_editor.rs
│   ├── simulation_profile_editor_options.rs
│   ├── simulation_profile_editor_sections.rs
│   ├── simulation_profile_editor_widgets.rs
│   ├── simulation_report_panel.rs
│   ├── simulation_waveform_panel.rs
│   ├── simulation_artifacts_panel.rs
│   │
│   ├── waveform_workspace.rs       # Waveform viewer
│   ├── waveform_workspace_sections.rs
│   ├── waveform_workspace_widgets.rs
│   │
│   ├── optimization_workspace.rs   # Optimization workspace
│   ├── optimization_workspace_sections.rs
│   ├── optimization_workspace_state.rs
│   ├── optimization_workspace_widgets.rs
│   │
│   ├── review_workspace.rs         # Design review workspace
│   ├── review_workspace_state.rs
│   ├── review_workspace_widgets.rs
│   ├── review_checklist.rs
│   │
│   ├── reports_workspace.rs        # Reports workspace
│   ├── reports_workspace_measurements.rs
│   ├── reports_workspace_preview.rs
│   ├── reports_workspace_sections.rs
│   ├── reports_workspace_state.rs
│   ├── reports_workspace_widgets.rs
│   │
│   ├── settings_workspace.rs       # Settings workspace
│   ├── settings_theme_preview.rs
│   │
│   ├── placement.rs                # Symbol placement state machine
│   ├── symbol_placement_controls.rs
│   ├── selection_properties.rs     # Property editor for selected items
│   ├── project_panel.rs
│   ├── diagnostics_panel.rs
│   ├── studio_toolbar.rs           # Top status bar
│   ├── status_strip.rs             # Bottom status bar
│   ├── preferences.rs
│   ├── widgets.rs                  # Reusable UI widgets
│   ├── workspace_panel.rs
│   ├── history.rs                  # Undo/redo stack
│   ├── file_dialog.rs              # Native file open/save dialogs (rfd)
│   │
│   └── waveform_preview.rs         # Mini waveform charts in bottom dock
│       waveform_preview_primitives.rs
│
├── canvas/                   # Canvas rendering pipeline
│   ├── mod.rs                # draw_scene(), draw_hover_highlight()
│   ├── colors.rs             # SchematicColors: theme-aware color palettes
│   └── primitives/           # Low-level drawing functions
│       ├── mod.rs            # Barrel: re-exports + polyline/line/bounds/bezier
│       ├── grid.rs           # Background grid (minor/major lines)
│       ├── sheet.rs          # Hierarchical sheet boxes
│       ├── symbol.rs         # Symbol graphics, pin shapes, fills
│       └── text.rs           # Rotated text rendering
│
├── document.rs               # KicadGuiDocument: load/save/edit KiCad schematics
├── library.rs                # KicadGuiLibrary: symbol library browser data
├── viewport.rs               # CanvasViewport: zoom/pan/coordinate transforms
├── placement_config.rs       # SymbolPlacementConfig: unit/body style/alternates
├── simulation.rs             # Simulation runner integration
├── simulation_run_loader.rs  # Load existing simulation runs
├── report_summary.rs         # Report artifact loading for GUI
├── waveform_summary.rs       # Waveform raw file loading for GUI
└── test_support.rs           # Test helpers (temp files, workspace root)
```

## `osl-kicad` Internal Structure

```
osl-kicad/src/
├── lib.rs                    # Crate root, public API re-exports
├── sexpr.rs                  # S-expression parser (KiCad file format)
├── schematic_io.rs           # Read/write .kicad_sch files
├── symbols_parse_impl.rs     # Parse .kicad_sym symbol libraries
├── symbol_library.rs         # Symbol library index and lookup
├── library_index.rs          # sym-lib-table parsing
├── project.rs                # .kicad_pro project file parsing
│
├── canvas.rs                 # KicadCanvasScene: scene builder from schematic IR
├── canvas_items.rs           # Canvas item types (Symbol/Sheet/Graphic/Wire/...)
├── canvas_items_graphic_impl.rs  # Graphic element specifics
├── canvas_items_leaf_impl.rs     # Wire/label/junction/no-connect specifics
├── canvas_items_bounds_impl.rs   # Bounding box computation for all items
├── canvas_hit.rs             # Hit testing: point-in-item detection
│
├── edit.rs                   # KicadSchematicEdit: edit operations enum
├── schematic_edit_impl.rs    # Edit operation execution
├── schematic_edit_symbol_ops_impl.rs   # Symbol-specific edit ops
├── schematic_edit_wiring_ops_impl.rs   # Wire/bus/label edit ops
├── schematic_util_impl.rs    # Utility operations on schematic IR
├── schematic_check_impl.rs   # DRC/ERC diagnostics
├── schematic_library_impl.rs # Library operations on schematic
├── schematic_summary.rs      # Summary statistics for GUI display
│
├── geometry.rs               # Bounding boxes, hit testing, point-in-polygon
├── transform.rs              # Coordinate transforms (mirror/rotate)
├── coordinates.rs            # KiCad coordinate system helpers
│
├── graphics.rs               # Graphic element types (polyline/bezier/rect/circle/arc)
├── symbols.rs                # Symbol definition types
├── pins.rs                   # Pin definitions and shapes
├── labels.rs                 # Net labels (local/global/hierarchical)
├── text.rs                   # Text element types
├── sheet.rs                  # Hierarchical sheet types
├── wiring.rs                 # Wire and bus types
├── markers.rs                # Junction and no-connect markers
├── group.rs                  # Group types
├── table.rs                  # Table/spreadsheet types
├── image.rs                  # Image embedding types
├── property.rs               # Property types
├── instances.rs              # Symbol instance data
├── metadata.rs               # Title block and metadata
├── style.rs                  # Stroke and fill styles
│
├── connectivity.rs           # Net connectivity analysis
├── spice_export.rs           # Generate SPICE netlist from KiCad schematic
├── simulation.rs             # Simulation directive extraction
├── diagnostics.rs            # Diagnostic message types
├── json.rs                   # JSON serialization helpers
├── util.rs                   # Internal utility functions
└── tests.rs                  # Integration tests
```

## Examples

```
examples/
├── cm5_minima/               # CM5 Minima demo board (default GUI schematic)
│   ├── CM5.kicad_sch         # Main schematic
│   ├── CM5IO.kicad_sym       # Custom symbol library
│   └── sym-lib-table
├── kicad_schematic/          # RC filter test fixture (unit tests)
│   ├── rc.kicad_sch
│   ├── neko_spice.kicad_sym
│   └── sym-lib-table
├── kicad_hierarchical/       # Multi-sheet hierarchical design
├── kicad_project/            # Full KiCad project with .cir output
├── kicad_project_schematic/  # KiCad project with schematic
├── ltspice_import/           # LTspice schematic import examples
├── rc_filter/                # Simple RC filter .cir
├── rc_sweep/                 # RC sweep analysis .cir
├── rlc_resonance/            # RLC resonance .cir
├── diode_rectifier/          # Diode rectifier .cir
├── pin_mapping/              # Op-amp pin mapping test cases
├── kicad_import/             # KiCad netlist import examples
├── vendor_model_issues/      # SPICE model validation edge cases
├── basic_validation.osl.yaml # Verification plan example
├── failing_validation.osl.yaml
└── structured_validation.osl.yaml
```

## Docs

```
docs/
├── dev.md                    # Developer setup and build instructions
├── development-plan.md       # Architecture and roadmap
├── three-day-sprint.md       # Initial sprint planning notes
└── ui/                       # UI reference images (design targets)
```
