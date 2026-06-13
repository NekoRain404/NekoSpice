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
| `nsp-app` | [crate-structure](../crates/nsp-app/docs/crate-structure.md), [ui-improvements](../crates/nsp-app/docs/ui-improvements.md), [schematic-bottom-dock](../crates/nsp-app/docs/schematic-bottom-dock.md), [simulation-profile-editor](../crates/nsp-app/docs/simulation-profile-editor.md), [context-menu-tool-palette](../crates/nsp-app/docs/context-menu-tool-palette.md) |
| `nsp-schema` | [file-split](../crates/nsp-schema/docs/file-split.md), [schematic-impl-split](../crates/nsp-schema/docs/schematic-impl-split.md) |
| `nsp-cli` | [file-split](../crates/nsp-cli/docs/file-split.md) |
| `nsp-netlist` | [file-split](../crates/nsp-netlist/docs/file-split.md) |
| `nsp-render` | [file-split](../crates/nsp-render/docs/file-split.md) |
| `nsp-waveform` | [file-split](../crates/nsp-waveform/docs/file-split.md) |
