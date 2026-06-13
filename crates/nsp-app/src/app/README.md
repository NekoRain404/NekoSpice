# NekoSpice App Module

## Overview

The `app` module contains the main application logic for the NekoSpice GUI,
including workspace management, tool handling, and panel rendering.

## Module Structure

### Core
- `app.rs` — Main application struct `NekoSpiceApp` and state management
- `theme.rs` — Theme system (Midnight, Graphite, Light palettes)
- `localization.rs` — UI text localization (English, Simplified Chinese)
- `navigation.rs` — Workspace enum and navigation logic
- `navigation_panel.rs` — Left sidebar navigation panel
- `status_strip.rs` — Top/bottom status bars with project info

### Schematic Workspace
- `schematic_workspace.rs` — Main schematic center workspace layout
- `schematic_workspace_widgets.rs` — Toolbar buttons, tabs, signal rows
- `schematic_inspector_panel.rs` — Right-side inspector panel
- `schematic_inspector_sections.rs` — Properties, Inspector, Libraries, Simulator, Review tabs
- `schematic_inspector_widgets.rs` — Inspector widget helpers
- `schematic_tools/` — Tool state, editing, preview, controls
- `canvas_panel.rs` — Canvas interaction and rendering dispatch
- `context_menu.rs` — Right-click context menu
- `tool_palette.rs` — Vertical tool palette on canvas left side

### Other Workspaces
- `home_dashboard.rs` — Home workspace dashboard
- `home_sections.rs` — Dashboard panel sections
- `simulation_workspace.rs` — Simulation configuration workspace
- `waveform_workspace.rs` — Waveform analysis workspace
- `library_workspace.rs` — Symbol library browser workspace
- `optimization_workspace.rs` — Parameter optimization workspace
- `review_workspace.rs` — Design review workspace
- `reports_workspace.rs` — Reports and artifacts workspace
- `settings_workspace.rs` — Theme and language settings

## Architecture Patterns

### State Management
- `NekoSpiceApp` holds all application state
- Each workspace has its own state struct (e.g., `SimulationPanelState`)
- State is updated via `impl NekoSpiceApp` methods

### Rendering
- All rendering uses egui's retained mode API
- Canvas rendering uses `egui::Painter` for direct drawing
- Panels use `egui::Frame` for consistent styling

### Theming
- `StudioTheme::palette(mode)` returns the current color palette
- `StudioTheme::panel_frame_for(mode)` returns styled frames
- All widgets accept `StudioThemeMode` for theme-aware rendering
