# Schematic Tools Module

## Overview

Manages the drawing tools available in the schematic workspace.
Each tool represents a different editing mode (Select, Wire, Bus, Label, etc.).

## Module Structure

- `mod.rs` — Module root and tool preview dispatch
- `state.rs` — `SchematicTool` enum and `SchematicToolState`
- `editing.rs` — Tool-specific editing actions (click handling)
- `preview.rs` — Tool preview rendering (ghost wire, label preview, etc.)
- `controls.rs` — Tool-specific UI controls

## Tool Enum

```rust
pub enum SchematicTool {
    Select,        // V - Selection tool
    Wire,          // W - Wire drawing
    Bus,           // B - Bus drawing
    BusEntry,      // E - Bus entry
    Label,         // L - Local net label
    GlobalLabel,   // G - Global net label
    HierarchicalLabel, // H - Hierarchical label
    Sheet,         // S - Hierarchical sheet
    Text,          // T - Free text
    Junction,      // J - Junction dot
    NoConnect,     // Q - No-connect marker
}
```

## Keyboard Shortcuts

| Key | Tool |
|-----|------|
| V | Select |
| W | Wire |
| L | Label |
| B | Bus |
| S | Sheet |
| J | Junction |
| Q | No-Connect |
| Esc | Cancel & Switch to Select |
| Del | Delete selected |
| F | Fit to screen |
| Arrow keys | Nudge selected item |

## Tool Preview

Each tool shows a preview at the mouse cursor position:
- Wire tool: shows a ghost wire from last point to cursor
- Label tool: shows a preview label at cursor
- Junction tool: shows a junction dot at cursor
- No-Connect tool: shows an X marker at cursor
