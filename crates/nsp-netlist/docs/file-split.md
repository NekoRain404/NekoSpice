# osl-netlist File Split

## Overview

The `osl-netlist` crate handles SPICE netlist parsing, normalization, import diagnostics, and suggested validation checks. It also contains the LTspice `.asc`/`.asy` import pipeline.

## File Structure

### Netlist parsing

| File | Lines | Purpose |
|------|-------|---------|
| `src/netlist_parse_impl.rs` | ~945 | Core netlist parsing (`parse_netlist`), directive/component parsers, flavor detection, signal normalization, include!d into lib.rs |
| `src/netlist_suggest_impl.rs` | ~218 | Suggested validation checks, signal priority ranking, YAML generation, include!d into netlist_parse_impl.rs |
| `src/lib.rs` | ~713 | Public types (`ImportReport`, `ComponentKind`, `AnalysisKind`, etc.), netlist normalization, YAML/JSON export, module declarations |

### LTspice import

| File | Lines | Purpose |
|------|-------|---------|
| `src/ltspice_import.rs` | ~701 | ASC/ASY parsing pipeline, netlist generation, graph building, include!d sub-files |
| `src/ltspice_types_impl.rs` | ~122 | Data types (`LtspiceSchematic`, `AscPoint`, `LtspiceWire`, `LtspiceSymbol`, etc.), include!d into ltspice_import.rs |
| `src/ltspice_builtins_impl.rs` | ~143 | Built-in LTspice symbol table (match arms), include!d into ltspice_import.rs |

## Inclusion Pattern

Both `netlist_parse_impl.rs` and `ltspice_import.rs` use `include!` to pull in their sub-files. The sub-files share the parent module's scope.

## Key APIs

- `parse_netlist(input, source)` — Parse SPICE netlist into `ImportReport`
- `import_ltspice_asc(path, base_dir)` — Import LTspice schematic into normalized netlist
- `normalize_netlist(...)` — Normalize imported netlist for NekoSpice compatibility
