# NekoSpice Documentation

## Overview

| Document | Description |
|----------|-------------|
| [dev.md](dev.md) | Developer setup, build instructions, and toolchain requirements |
| [development-plan.md](development-plan.md) | Architecture overview and feature roadmap |
| [three-day-sprint.md](three-day-sprint.md) | Initial sprint planning notes |
| [../PROJECT_STRUCTURE.md](../PROJECT_STRUCTURE.md) | Complete project file tree with descriptions |

## UI Reference

`ui/` directory contains design reference images for the NekoSpice GUI. These are target designs for the schematic editor, simulation panel, waveform viewer, library browser, and other workspace views.

## Crate-level Documentation

Each crate may contain a `docs/` directory with internal architecture notes:

| Crate | Documents |
|-------|-----------|
| `osl-app` | [crate-structure](../crates/osl-app/docs/crate-structure.md), [ui-improvements](../crates/osl-app/docs/ui-improvements.md), [schematic-bottom-dock](../crates/osl-app/docs/schematic-bottom-dock.md), [simulation-profile-editor](../crates/osl-app/docs/simulation-profile-editor.md), [context-menu-tool-palette](../crates/osl-app/docs/context-menu-tool-palette.md) |
| `osl-kicad` | [file-split](../crates/osl-kicad/docs/file-split.md), [schematic-impl-split](../crates/osl-kicad/docs/schematic-impl-split.md) |
| `osl-cli` | [file-split](../crates/osl-cli/docs/file-split.md) |
| `osl-netlist` | [file-split](../crates/osl-netlist/docs/file-split.md) |
| `osl-render` | [file-split](../crates/osl-render/docs/file-split.md) |
| `osl-waveform` | [file-split](../crates/osl-waveform/docs/file-split.md) |
