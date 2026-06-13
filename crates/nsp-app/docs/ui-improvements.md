# NekoSpice UI Improvements

## Overview

This document describes the UI improvements made to NekoSpice's schematic workspace,
navigation panel, and canvas rendering.

## Navigation Panel (`navigation_panel.rs`)

### Features
- Unicode icon-based workspace switcher (⌂ ◎ ☰ ▶ ⚙ ✔ ∿ ≡ ⚒)
- Active workspace indicator with left accent bar
- Branded header with ◎ logo and project subtitle
- System info footer (renderer, solver, document state, theme, language)

### Workspace Icons
| Workspace | Icon | Description |
|-----------|------|-------------|
| Home | ⌂ | Project dashboard |
| Schematic | ◎ | format-compatible editing |
| Library | ☰ | Symbol browser |
| Simulation | ▶ | ngspice runner |
| Optimization | ⚙ | Sweep/Monte Carlo |
| Review | ✔ | Design review |
| Waveforms | ∿ | Signal analysis |
| Reports | ≡ | Artifact viewer |
| Settings | ⚒ | Theme/language |

## Toolbar (`schematic_workspace.rs`)

### Features
- Grouped sections: File ops | Drawing tools | Zoom | DRC
- Drawing tools: Wire (┌), Net Label (←), Bus (═), Sheet Symbol (▣)
- Live DRC status with color-coded dot indicator
- Zoom percentage display

## Bottom Dock Tabs (`schematic_workspace.rs`)

### Tabs
- **Waveforms**: Signal list with colored indicators
- **FFT**: Placeholder for FFT analysis
- **Bode**: Placeholder for Bode plot
- **Console**: Simulation output and status messages
- **Netlist**: SPICE netlist preview from loaded schematic
- **ERC**: Electrical Rules Check results with severity colors
- **Inspector**: Selected item properties and UUID

## Theme (`theme.rs`)

### Palette Modes
1. **Midnight**: GitHub-inspired dark theme (best contrast for schematics)
2. **Graphite**: Neutral dark theme (reduced eye strain)
3. **Light**: Clean light theme for daytime use

### Color Philosophy
- Canvas background is always light (standard convention: `rgb(236, 240, 244)`)
- UI panels use dark backgrounds regardless of theme mode
- Accent color: GitHub blue (`rgb(56, 139, 253)`) in Midnight
- Schematic elements use standard colors (green wires, blue buses)

## Canvas Rendering (`canvas/`)

### Grid
- Minor lines: 2.54mm (100mil) step — aligns with standard schematic grid
- Major lines: 12.7mm (500mil) step — thicker, darker lines
- Dynamic grid density based on zoom level

### Pin Rendering
- Direction-aware text placement (perpendicular to pin direction)
- Pin name positioned at body end
- Pin number positioned at external end
- Font size from text effects (fallback: 8pt name, 7pt number)
- Monospace for numbers, proportional for names

### Layer Order (bottom to top)
1. Sheets
2. Rule areas
3. Graphics
4. Symbols (body, pins, labels)
5. Wires
6. Buses
7. Bus entries
8. Directive labels
9. Net labels
10. Free text
11. Text boxes
12. Junctions
13. No-connects

## Localization (`localization.rs`)

### File Split
- `localization.rs` — StudioLocale enum, UiText enum, tests (323 lines)
- `localization_en_impl.rs` — English translations (282 lines)
- `localization_zh_impl.rs` — Simplified Chinese translations (268 lines)

Uses `include!` macro at module level, consistent with the project pattern.
