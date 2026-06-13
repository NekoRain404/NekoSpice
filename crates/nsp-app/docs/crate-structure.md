# NekoSpice App Crate Structure

## Overview

The `osl-app` crate provides the NekoSpice GUI shell using eframe/egui with wgpu hardware acceleration.
All GUI composition is separated from Schema data adapters and canvas rendering helpers.

## Module Layout

```
crates/osl-app/src/
├── main.rs                  # Entry point (delegates to run_native)
├── lib.rs                   # Crate root: re-exports, constants
├── document.rs              # KicadGuiDocument: load/save/edit Schema schematics
├── library.rs               # KicadGuiLibrary: symbol library browser adapter
├── placement_config.rs      # SymbolPlacementConfig: unit/body/alternate settings
├── viewport.rs              # CanvasViewport: zoom, pan, coordinate transforms
├── simulation.rs            # Background simulation task runner
├── simulation_run_loader.rs # Load existing simulation run directories
├── report_summary.rs        # GUI report summary state
├── waveform_summary.rs      # GUI waveform summary state
├── test_support.rs          # Test utilities and fixtures
│
├── canvas/                  # Canvas rendering pipeline
│   ├── colors.rs            # Schematic color palette (Schema standard)
│   └── primitives.rs        # Drawing primitives (grid, lines, arcs, bezier)
│
├── app/                     # Application UI modules
│   ├── app.rs               # NekoSpiceApp struct and core logic
│   ├── runtime.rs           # eframe window setup and CJK font loading
│   ├── theme.rs             # StudioTheme: palettes, styles, visual helpers
│   ├── localization.rs      # UiText enum + en/zh translations (split via include!)
│   ├── localization_en_impl.rs   # English translations
│   ├── localization_zh_impl.rs   # Simplified Chinese translations
│   ├── navigation.rs        # StudioWorkspace enum (9 workspaces)
│   ├── navigation_panel.rs  # Sidebar with icons and system info
│   ├── center_workspace.rs  # Workspace dispatcher
│   ├── status_strip.rs      # Top/bottom status bars
│   │
│   ├── schematic_workspace.rs           # Schematic center view + toolbar + dock
│   ├── schematic_workspace_widgets.rs   # Toolbar, tab, signal row widgets
│   ├── schematic_tools/                # Drawing tools
│   │   ├── mod.rs
│   │   ├── state.rs         # SchematicToolState
│   │   ├── controls.rs      # Tool UI controls
│   │   ├── editing.rs       # Wire/bus/label editing operations
│   │   └── preview.rs       # Tool preview rendering
│   ├── canvas_panel.rs      # Canvas interaction (click, drag, zoom, keyboard)
│   ├── placement.rs         # Symbol placement state machine
│   ├── symbol_placement_controls.rs  # Unit/body/alternate selection
│   ├── selection_properties.rs       # Selected item property editor
│   ├── schematic_inspector_panel.rs  # Inspector panel
│   ├── schematic_inspector_*.rs      # Inspector sub-modules
│   ├── schematic_review_panel.rs     # Design review panel
│   │
│   ├── home_dashboard.rs    # Home workspace dashboard
│   ├── home_*.rs            # Home sub-modules
│   ├── library_workspace.rs # Library browser workspace
│   ├── library_*.rs         # Library sub-modules
│   ├── simulation_workspace.rs        # Simulation workspace
│   ├── simulation_workspace_*.rs      # Simulation sub-modules
│   ├── simulation_panel.rs           # Simulation control panel
│   ├── simulation_profile_editor.rs  # Profile editor
│   ├── simulation_profile_editor_*.rs
│   ├── simulation_artifacts_panel.rs
│   ├── simulation_report_panel.rs
│   ├── simulation_waveform_panel.rs
│   ├── waveform_workspace.rs         # Waveform analysis workspace
│   ├── waveform_workspace_*.rs
│   ├── waveform_preview.rs           # Waveform preview widget
│   ├── waveform_preview_primitives.rs
│   ├── optimization_workspace.rs     # Optimization workspace
│   ├── optimization_workspace_*.rs
│   ├── review_workspace.rs           # Review workspace
│   ├── review_workspace_*.rs
│   ├── reports_workspace.rs          # Reports workspace
│   ├── reports_workspace_*.rs
│   ├── settings_workspace.rs         # Settings workspace
│   ├── settings_theme_preview.rs
│   ├── project_panel.rs              # Project panel
│   ├── panels.rs                     # Panel layout helpers
│   ├── workspace_panel.rs            # Workspace panel wrapper
│   ├── widgets.rs                    # Shared widget helpers
│   └── preferences.rs                # User preferences
│
└── docs/                    # Module documentation
    └── ui-improvements.md   # UI design documentation
```

## Design Principles

1. **Decoupling**: GUI composition never directly references Schema file internals.
   The `document.rs` and `library.rs` adapters provide clean interfaces.
2. **File Splitting**: Large files use `include!` macro to split implementation
   across files while maintaining shared scope. No file exceeds ~950 lines.
3. **Theming**: All colors flow through `StudioTheme::palette(mode)`.
   Widget code never hardcodes colors.
4. **Localization**: All user-facing strings go through `UiText` enum with
   `en()` and `zh_hans()` methods, supporting English and Simplified Chinese.
5. **Canvas Separation**: Canvas rendering (`canvas/`) is independent of the
   application state. It takes scene data + viewport and draws to a painter.
