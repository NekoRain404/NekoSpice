# Canvas Rendering Pipeline

## Overview

The canvas module handles all schematic rendering in the NekoSpice GUI.
It converts KiCad canvas scene data into visual output using egui's painter API.

## Architecture

```text
KicadSchematic → KicadCanvasScene → draw_scene(colors) → egui Painter
```

### Theme-Aware Rendering

All element colors are provided by `SchematicColors`, constructed from the
current `StudioThemeMode`:

```rust
let colors = SchematicColors::for_mode(self.theme_mode());
canvas::draw_grid(&painter, rect, viewport, colors);
canvas::draw_scene(&painter, rect, scene, viewport, bounds, colors);
```

- **Light theme**: KiCad-standard colors on light-gray background
- **Dark theme (Midnight/Graphite)**: Bright element colors on dark background

### Rendering Layers (draw_scene)

Items are drawn in order from back to front:

1. **Hierarchical sheets** — background fill + border + name label
2. **Rule areas** — translucent fill outlines
3. **Top-level graphics** — polylines, arcs, rectangles, circles
4. **Symbols** — graphic bodies, pins, pin names/numbers, reference/value labels
5. **Wires** — green polylines with KiCad-default stroke width
6. **Buses** — blue polylines
7. **Bus entries** — short diagonal lines
8. **Directive labels** — netclass flags with bounds box
9. **Net labels** — local (blue), global (dark blue), hierarchical (purple)
10. **Free text** — user-placed text items
11. **Text boxes** — bordered text regions
12. **Junctions** — filled green dots
13. **No-connect markers** — X marks

### File Structure

- `canvas.rs` — `draw_scene()` main entry point and label/text rendering
- `canvas/primitives.rs` — Drawing primitives (grid, sheet, graphics, wires, text rotation, pins)
- `canvas/colors.rs` — `SchematicColors` struct and legacy color constants

### Key Features

- **Theme-aware colors**: All element colors adapt to light/dark themes via `SchematicColors`
- **Stroke width resolution**: Wire/bus/graphic stroke widths are read from KiCad data.
  When KiCad specifies `(stroke (width 0))`, the renderer defaults to 0.1524mm (6mil).
- **Rotated text**: Text rotation is supported via character-by-character rendering
  for arbitrary angles, with optimized native rendering for cardinal angles (0/90/180/270).
- **Pin shape rendering**: Supports line, inverted, clock, inverted_clock, input_low,
  clock_low, falling_edge_clock, and non_logic pin shapes.
- **Fill rendering**: Solid fills are rendered with translucent background for polygons,
  rectangles, and circles.
- **Hover highlight**: Semi-transparent blue glow appears when hovering over elements
- **Selection highlight**: Solid blue outline for selected elements

### Coordinate System

- World coordinates: KiCad mm units (origin top-left, Y increases downward)
- Screen coordinates: egui pixel coordinates (origin at canvas rect top-left)
- `CanvasViewport` handles the mapping between world and screen space
