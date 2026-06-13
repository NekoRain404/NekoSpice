# osl-cli

Command-line interface binary (`osl`). The primary entry point for headless NekoSpice operations.

## Commands

| Command | Description |
|---------|-------------|
| `run` | Run a `.cir` file through ngspice |
| `verify` | Execute a YAML verification plan |
| `bench` | Benchmark all `.cir` files under a directory |
| `model-check` | Scan SPICE models for `.subckt`/`.model` validation |
| `import` | Import SPICE/Schema/LTspice netlists into NekoSpice project |
| `waveform` | Query raw waveforms into min/max envelope JSON |
| `schema-inspect` | Parse Schema files and emit JSON summary |
| `schema-check` | Run schematic diagnostics (DRC/ERC) |
| `schema-edit` | Apply edit commands to Schema schematics |
| `schema-export` | Write Schema-compatible files from IR |
| `schema-render` | Render schematics to SVG |
| `schema-select` | Hit-test schematic canvas points |

## Build

```bash
cargo build -p osl-cli
```

## Usage

```bash
cargo run -p osl-cli -- --version
cargo run -p osl-cli -- run examples/rc_filter/rc.cir --output runs/rc_001
```
