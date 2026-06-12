# NekoSpice Project Structure

> Complete file tree with descriptions. Last updated: 2026-06-12.

NekoSpice is a Rust-native SPICE simulation platform with KiCad-compatible schematic editing,
ngspice/Xyce simulation backends, and a hardware-accelerated GUI (egui + wgpu).

---

## Root

```
NekoSpice/
├── Cargo.toml                # Workspace root: 10 crates, Rust 2024 edition
├── Cargo.lock                # Dependency lock file
├── README.md                 # Project overview, CLI usage, quick start
├── PROJECT_STRUCTURE.md      # This file
├── .gitignore                # Git ignore rules (target/, runs/, reports/, etc.)
│
├── crates/                   # All Rust source code (10 workspace crates)
├── docs/                     # Documentation and UI reference images
├── examples/                 # Test fixtures and demo projects
├── benchmarks/               # Benchmark configurations
├── runs/                     # Simulation run outputs (gitignored)
└── kicad-source-mirror-master/  # KiCad source reference (gitignored)
```

---

## crates/ — Workspace Crates

### Architecture Layers

```
┌─────────────────────────────────────────────────┐
│  osl-app (GUI)        osl-cli (CLI binary)      │  ← User-facing
├─────────────────────────────────────────────────┤
│  osl-render   osl-report   osl-waveform         │  ← Output & visualization
├─────────────────────────────────────────────────┤
│  osl-sim      osl-netlist   osl-model            │  ← Simulation & import
├─────────────────────────────────────────────────┤
│  osl-kicad                                         │  ← KiCad IR & operations
├─────────────────────────────────────────────────┤
│  osl-core                                         │  ← Shared foundation
└─────────────────────────────────────────────────┘
```

### osl-core — Shared Foundation

```
crates/osl-core/
├── Cargo.toml
├── README.md
└── src/
    └── lib.rs              # Common types, error handling, utilities
```

Zero external domain dependencies. Every other `osl-*` crate depends on it.

---

### osl-kicad — KiCad IR & Operations

```
crates/osl-kicad/
├── Cargo.toml
├── README.md
├── docs/
│   ├── file-split.md
│   └── schematic-impl-split.md
├── tests/
│   └── kicad_demo_smoke.rs     # External KiCad demo interoperability test
└── src/
    ├── lib.rs                   # Crate root, public API re-exports
    │
    ├── sexpr.rs                 # S-expression parser (KiCad file format)
    ├── schematic_io.rs          # Read/write .kicad_sch files
    ├── symbols_parse_impl.rs    # Parse .kicad_sym symbol libraries
    ├── symbol_library.rs        # Symbol library index and lookup
    ├── library_index.rs         # sym-lib-table parsing
    ├── project.rs               # .kicad_pro project file parsing
    │
    ├── canvas.rs                # KicadCanvasScene: scene builder from schematic IR
    ├── canvas_items.rs          # Canvas item types (Symbol/Sheet/Graphic/Wire/...)
    ├── canvas_items_graphic_impl.rs   # Graphic element specifics
    ├── canvas_items_leaf_impl.rs      # Wire/label/junction/no-connect specifics
    ├── canvas_items_bounds_impl.rs    # Bounding box computation
    ├── canvas_hit.rs            # Hit testing: point-in-item detection
    │
    ├── edit.rs                  # KicadSchematicEdit: edit operations enum
    ├── schematic_edit_impl.rs   # Edit operation execution
    ├── schematic_edit_symbol_ops_impl.rs   # Symbol placement/move/delete
    ├── schematic_edit_wiring_ops_impl.rs   # Wire/bus/label operations
    ├── schematic_util_impl.rs   # Utility operations on schematic IR
    ├── schematic_check_impl.rs  # DRC/ERC diagnostics
    ├── schematic_library_impl.rs # Library operations
    ├── schematic_summary.rs     # Summary statistics for GUI display
    │
    ├── geometry.rs              # Bounding boxes, hit testing, point-in-polygon
    ├── transform.rs             # Coordinate transforms (mirror/rotate)
    ├── coordinates.rs           # KiCad coordinate system helpers
    │
    ├── graphics.rs              # Graphic elements (polyline/bezier/rect/circle/arc)
    ├── symbols.rs               # Symbol definition types
    ├── pins.rs                  # Pin definitions and shapes
    ├── labels.rs                # Net labels (local/global/hierarchical)
    ├── text.rs                  # Text element types
    ├── sheet.rs                 # Hierarchical sheet types
    ├── wiring.rs                # Wire and bus types
    ├── markers.rs               # Junction and no-connect markers
    ├── group.rs                 # Group types
    ├── table.rs                 # Table/spreadsheet types
    ├── image.rs                 # Image embedding types
    ├── property.rs              # Property types
    ├── instances.rs             # Symbol instance data
    ├── metadata.rs              # Title block and metadata
    ├── style.rs                 # Stroke and fill styles
    │
    ├── connectivity.rs          # Net connectivity analysis
    ├── spice_export.rs          # Generate SPICE netlist from schematic
    ├── simulation.rs            # Simulation directive extraction
    ├── diagnostics.rs           # Diagnostic message types
    ├── json.rs                  # JSON serialization helpers
    ├── util.rs                  # Internal utility functions
    └── tests.rs                 # Integration tests
```

---

### osl-sim — Simulation Backends

```
crates/osl-sim/
├── Cargo.toml
├── README.md
└── src/
    ├── lib.rs              # NgspiceCliBackend, XyceCliBackend, SimulatorBackend trait
    └── artifacts.rs        # Run artifact collection (.raw, .csv, summaries)
```

---

### osl-netlist — Netlist Parsing & Import

```
crates/osl-netlist/
├── Cargo.toml
├── README.md
├── docs/
│   └── file-split.md
├── tests/
│   └── kicad_demo_import_smoke.rs   # External KiCad demo import test
└── src/
    ├── lib.rs                   # Crate root
    ├── kicad_import.rs          # KiCad netlist import
    ├── ltspice_import.rs        # LTspice schematic import
    ├── netlist_parse_impl.rs    # SPICE netlist parser
    ├── netlist_suggest_impl.rs  # Signal suggestion engine
    ├── ltspice_builtins_impl.rs # LTspice built-in component mapping
    └── ltspice_types_impl.rs    # LTspice type definitions
```

---

### osl-model — SPICE Model Checking

```
crates/osl-model/
├── Cargo.toml
├── README.md
└── src/
    ├── lib.rs                   # Crate root
    └── model_check_impl.rs      # .subckt/.model validation, pin mapping, dialect risks
```

---

### osl-waveform — Waveform Data Parser

```
crates/osl-waveform/
├── Cargo.toml
├── README.md
├── docs/
│   └── file-split.md
└── src/
    ├── lib.rs                   # Crate root
    └── raw_parser_impl.rs       # .raw file parser (binary + ASCII)
```

---

### osl-render — SVG Renderer

```
crates/osl-render/
├── Cargo.toml
├── README.md
├── docs/
│   └── file-split.md
└── src/
    ├── lib.rs                   # Crate root
    ├── svg_render_impl.rs       # Main SVG rendering pipeline
    └── svg_helpers_impl.rs      # SVG helper utilities
```

---

### osl-report — Report Generation

```
crates/osl-report/
├── Cargo.toml
├── README.md
└── src/
    ├── lib.rs                   # Crate root
    ├── format.rs                # Report format types
    ├── json.rs                  # JSON report writer
    ├── html.rs                  # HTML report writer
    ├── junit.rs                 # JUnit XML report writer
    ├── markdown.rs              # Markdown report writer
    ├── bundle.rs                # Multi-format bundle writer
    └── directory.rs             # Directory-based report output
```

---

### osl-cli — Command-Line Interface

```
crates/osl-cli/
├── Cargo.toml
├── README.md
├── docs/
│   └── file-split.md
└── src/
    ├── main.rs                  # Entry point, CLI argument parsing
    ├── kicad_edit.rs            # KiCad edit command implementation
    ├── cli_kicad.rs             # KiCad subcommand handlers
    └── cli_verify.rs            # Verify subcommand handler
```

---

### osl-app — GUI Application

```
crates/osl-app/
├── Cargo.toml
├── README.md
├── docs/
│   ├── crate-structure.md
│   ├── ui-improvements.md
│   ├── schematic-bottom-dock.md
│   ├── simulation-profile-editor.md
│   └── context-menu-tool-palette.md
└── src/
    ├── lib.rs                       # Crate root, DEFAULT_SCHEMATIC constants
    ├── main.rs                      # Entry point (native window launch)
    ├── test_support.rs              # Test helpers
    │
    ├── app.rs                       # NekoSpiceApp struct, Default impl, core actions
    ├── app/
    │   ├── README.md                # App module guide
    │   │
    │   │── [Layout & Panels]
    │   ├── panels.rs                # Root layout: top/bottom bars, nav, context, workspace
    │   ├── runtime.rs               # eframe::App impl, native window launch
    │   ├── workspace_panel.rs       # Workspace panel container
    │   ├── center_workspace.rs      # Center workspace area
    │   ├── status_strip.rs          # Status bar at bottom
    │   ├── studio_toolbar.rs        # Main toolbar
    │   ├── project_panel.rs         # Project file tree panel
    │   ├── diagnostics_panel.rs     # Diagnostics/errors panel
    │   │
    │   │── [Navigation & Theme]
    │   ├── navigation.rs            # StudioWorkspace enum
    │   ├── navigation_panel.rs      # Left sidebar workspace switcher
    │   ├── theme.rs                 # StudioTheme: Midnight/Graphite/Light palettes
    │   │
    │   │── [Localization]
    │   ├── localization.rs          # UiText enum for i18n (en/zh_hans)
    │   ├── localization_en_impl.rs  # English translations
    │   ├── localization_zh_impl.rs  # Chinese translations
    │   │
    │   │── [Schematic Canvas]
    │   ├── canvas_panel.rs          # Main canvas: viewport, mouse interaction, rendering
    │   ├── canvas_shortcuts.rs      # Keyboard shortcuts (V/W/L/B/S/J/Q/R/F/Del/Esc)
    │   ├── canvas_context_menu.rs   # Right-click context menu
    │   ├── tool_palette.rs          # Vertical tool palette (left of canvas)
    │   ├── shortcuts_overlay.rs     # Keyboard shortcut help overlay (? key)
    │   │
    │   │── [Schematic Workspace]
    │   ├── schematic_workspace.rs          # Schematic view: toolbar, tabs, canvas, inspector
    │   ├── schematic_workspace_widgets.rs  # Toolbar buttons, document tabs
    │   ├── schematic_bottom_dock.rs        # Bottom dock: Waveforms/FFT/Bode/Console/Netlist/ERC
    │   ├── schematic_inspector_panel.rs    # Right-side inspector panel
    │   ├── schematic_inspector_sections.rs # Inspector section layout
    │   ├── schematic_inspector_simulator.rs # Simulator config in inspector
    │   ├── schematic_inspector_widgets.rs  # Inspector widget helpers
    │   ├── schematic_review_panel.rs       # Schematic review/ERC panel
    │   │
    │   │── [Schematic Tools State Machine]
    │   ├── schematic_tools/
    │   │   ├── README.md            # Tools module guide
    │   │   ├── mod.rs               # Module root
    │   │   ├── state.rs             # SchematicTool enum and state
    │   │   ├── controls.rs          # Tool activation and switching
    │   │   ├── editing.rs           # Wire/bus/label/sheet creation
    │   │   └── preview.rs           # Tool preview rendering
    │   │
    │   │── [Selection & Placement]
    │   ├── selection_properties.rs  # Selected item property editor
    │   ├── symbol_placement_controls.rs # Symbol placement UI
    │   ├── placement.rs            # Placement state management
    │   ├── history.rs              # Undo/redo history
    │   ├── file_dialog.rs          # File open/save dialogs
    │   ├── preferences.rs          # User preferences
    │   ├── widgets.rs              # Shared UI widgets
    │   │
    │   │── [Home Workspace]
    │   ├── home_dashboard.rs       # Home workspace dashboard
    │   ├── home_command_center.rs  # Quick-action command center
    │   ├── home_insights_panel.rs  # Project insights panel
    │   ├── home_project_context.rs # Project context display
    │   ├── home_sections.rs        # Home section layout
    │   └── home_widgets.rs         # Home widget helpers
    │   │
    │   │── [Library Workspace]
    │   ├── library_workspace.rs    # Symbol library browser
    │   ├── library_model_browser.rs    # Model file browser
    │   ├── library_model_validation.rs # Model validation UI
    │   ├── library_preview.rs      # Symbol preview renderer
    │   ├── library_sections.rs     # Library section layout
    │   ├── library_widgets.rs      # Library widget helpers
    │   ├── library_data.rs         # Library data management
    │   └── library_inspector.rs    # Library inspector panel
    │   │
    │   │── [Simulation Workspace]
    │   ├── simulation_workspace.rs   # Simulation workspace view
    │   ├── simulation_workspace_sections.rs  # Section layout
    │   ├── simulation_workspace_widgets.rs   # Widget helpers
    │   ├── simulation_panel.rs       # Simulation control panel
    │   ├── simulation_profile_editor.rs      # Profile editor
    │   ├── simulation_profile_editor_options.rs   # Profile options
    │   ├── simulation_profile_editor_sections.rs  # Profile sections
    │   ├── simulation_profile_editor_widgets.rs   # Profile widgets
    │   ├── simulation_artifacts_panel.rs  # Artifacts display
    │   ├── simulation_waveform_panel.rs   # Waveform display
    │   └── simulation_report_panel.rs     # Report display
    │   │
    │   │── [Waveform Workspace]
    │   ├── waveform_workspace.rs   # Waveform viewer workspace
    │   ├── waveform_workspace_sections.rs  # Section layout
    │   ├── waveform_workspace_widgets.rs   # Widget helpers
    │   ├── waveform_preview.rs     # Waveform preview renderer
    │   └── waveform_preview_primitives.rs  # Preview drawing primitives
    │   │
    │   │── [Reports Workspace]
    │   ├── reports_workspace.rs    # Reports viewer workspace
    │   ├── reports_workspace_sections.rs   # Section layout
    │   ├── reports_workspace_widgets.rs    # Widget helpers
    │   ├── reports_workspace_state.rs      # State management
    │   ├── reports_workspace_measurements.rs # Measurement display
    │   └── reports_workspace_preview.rs    # Report preview
    │   │
    │   │── [Review Workspace]
    │   ├── review_workspace.rs     # Design review workspace
    │   ├── review_workspace_state.rs    # State management
    │   ├── review_workspace_widgets.rs  # Widget helpers
    │   └── review_checklist.rs     # Review checklist items
    │   │
    │   │── [Optimization Workspace]
    │   ├── optimization_workspace.rs         # Optimization workspace
    │   ├── optimization_workspace_state.rs   # State management
    │   ├── optimization_workspace_sections.rs # Section layout
    │   └── optimization_workspace_widgets.rs  # Widget helpers
    │   │
    │   └── [Settings Workspace]
    │       ├── settings_workspace.rs     # Settings/preferences workspace
    │       └── settings_theme_preview.rs # Theme preview
    │
    ├── canvas.rs                    # Canvas module root
    ├── canvas/
    │   ├── README.md                # Canvas module guide
    │   ├── colors.rs                # Theme-aware color definitions
    │   └── primitives/              # Canvas drawing primitives
    │       ├── mod.rs               # Primitive module root
    │       ├── grid.rs              # Grid rendering
    │       ├── sheet.rs             # Sheet boundary rendering
    │       ├── symbol.rs            # Symbol rendering
    │       └── text.rs              # Text rendering
    │
    ├── simulation.rs                # Simulation spawn helpers (ngspice + Xyce)
    ├── simulation_run_loader.rs     # Run output loader for GUI
    ├── waveform_summary.rs          # Waveform summary for GUI
    ├── report_summary.rs            # Report summary for GUI
    └── placement_config.rs          # Placement configuration
```

---

## docs/ — Documentation

```
docs/
├── README.md                   # Documentation index
├── dev.md                      # Developer setup and build instructions
├── development-plan.md         # Architecture overview and feature roadmap
├── three-day-sprint.md         # Initial sprint planning notes
└── ui/                         # UI design reference images (gitignored)
    ├── ui-ref-01.png ... ui-ref-10.png
```

---

## examples/ — Test Fixtures & Demos

```
examples/
├── cm5_minima/                 # CM5 Minima demo board (default GUI schematic)
│   ├── CM5.kicad_sch
│   ├── CM5IO.kicad_sym
│   └── sym-lib-table
├── kicad_schematic/            # RC filter test fixture (unit tests)
│   ├── rc.kicad_sch
│   ├── neko_spice.kicad_sym
│   └── sym-lib-table
├── kicad_hierarchical/         # Multi-sheet hierarchical design
│   ├── kicad_hierarchical.kicad_pro
│   ├── kicad_hierarchical.kicad_sch
│   ├── gain_stage.kicad_sch
│   └── sym-lib-table
├── kicad_project_schematic/    # KiCad project with schematic
│   ├── kicad_project_schematic.kicad_pro
│   ├── kicad_project_schematic.kicad_sch
│   └── sym-lib-table
├── kicad_project/              # Full KiCad project with .cir output
│   ├── kicad_project.kicad_pro
│   ├── kicad_project.cir
│   └── models/
├── kicad_import/               # KiCad netlist import examples
│   ├── kicad_rc.cir
│   ├── kicad_diode_include.cir
│   └── models/
├── ltspice_import/             # LTspice schematic import examples
│   ├── ltspice_rc.asc
│   ├── ltspice_bjt.asc
│   ├── ltspice_subckt.asc
│   ├── ltspice_sym_search.asc
│   ├── ltspice_vcvs.asc
│   ├── gain_block.asy
│   ├── gain_block.lib
│   └── sym/
├── rc_filter/                  # Simple RC filter .cir
│   └── rc.cir
├── rc_sweep/                   # RC sweep analysis .cir
│   └── rc_sweep.cir
├── rlc_resonance/              # RLC resonance .cir
│   └── rlc.cir
├── diode_rectifier/            # Diode rectifier .cir
│   └── rectifier.cir
├── pin_mapping/                # Op-amp pin mapping test cases
│   ├── good_opamp.lib
│   ├── good_opamp.asy
│   └── bad_opamp.asy
├── vendor_model_issues/        # SPICE model validation edge cases
│   └── bad_vendor_model.lib
├── basic_validation.osl.yaml   # Verification plan example
├── failing_validation.osl.yaml # Expected-failure verification plan
└── structured_validation.osl.yaml # Structured verification plan
```

---

## benchmarks/

```
benchmarks/
└── basic/
    └── basic.osl.yaml          # Basic benchmark configuration
```

---

## runs/ (gitignored)

```
runs/
└── gui/                        # GUI simulation run outputs (auto-generated)
```

---

## Dependency Graph

```
osl-app ──────┬── osl-kicad ──── osl-core
              ├── osl-sim ────── osl-core
              ├── osl-render ─── osl-kicad
              ├── osl-waveform
              └── osl-report

osl-cli ──────┬── osl-kicad
              ├── osl-sim
              ├── osl-netlist ── osl-core
              ├── osl-model
              ├── osl-render
              ├── osl-waveform
              └── osl-report
```
