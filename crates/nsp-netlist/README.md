# osl-netlist

`osl-netlist` owns import-time conversion from external schematic/netlist formats
into a runnable SPICE deck plus a structured compatibility report.

## Boundaries

- `lib.rs` exposes the public import/report API, generic SPICE netlist parsing,
  normalized project generation, and the dispatch path used by CLI and GUI
  workflows.
- `schema_import.rs` owns project/source discovery and schematic
  diagnostic mapping for import reports. It does not parse S-expressions;
  that remains in `nsp-schema`.
- `ltspice_import.rs` owns LTspice `.asc` parsing, `.asy` symbol lookup,
  primitive fallback symbols, pin-to-net mapping, and line-level migration
  diagnostics.

## Refactor Direction

Keep format-specific discovery and migration code out of the generic parser.
Future cleanup should move the generic SPICE parser/report formatter into its
own module while preserving `read_import_input` as the crate entry point used by
CLI and GUI workflows.
