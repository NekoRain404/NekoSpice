# Schematic Bottom Dock

## Overview

The schematic workspace bottom dock provides a tabbed panel at the bottom of the schematic canvas. It allows users to switch between different views without leaving the schematic context.

## Tabs

| Tab | Content |
|-----|---------|
| Waveforms | Signal list from loaded schematic labels + simulation status |
| FFT | Placeholder for future FFT analysis |
| Bode | Placeholder for future Bode plot |
| Console | Simulation status, errors, log output |
| Netlist | Live SPICE netlist preview from loaded schematic |
| ERC | Electrical Rules Check results with severity levels |
| Inspector | Selected item kind, UUID, and bounding box |

## State

- `SchematicBottomTab` enum in `app.rs` tracks the active tab
- Tab state persists across redraws until user clicks a different tab
- Default tab: `Waveforms`

## Implementation

- Tabs are rendered inline in `schematic_workspace.rs` (no separate widget)
- Each tab has its own draw method: `draw_bottom_*_tab()`
- ERC tab uses `KicadSchematicCheckReport` with `diagnostics` vector
- Netlist tab calls `document.spice_netlist_preview()` for live preview
