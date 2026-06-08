# NekoSpice

NekoSpice is a Rust-first SPICE automation tool that uses ngspice for circuit solving and Rust for repeatable runs, measurements, reports, and batch verification.

Schematic authoring is KiCad-compatible and Rust-native: NekoSpice is growing its own schematic and symbol-library subsystem around `.kicad_sch`, `.kicad_sym`, and `.kicad_pro` assets while keeping simulation automation, import diagnostics, waveform data, model checks, and CI-ready reports as the core differentiators.

The current three-day build is a vertical slice:

- `osl run`: run one `.cir` file through ngspice.
- `osl verify`: run a small YAML verification plan.
- `osl bench`: run every `.cir` under a directory and collect timings.
- `osl model-check`: scan imported SPICE models for `.subckt`, `.model`, LTspice symbol pin mapping, dialect risks, and unsupported directives.
- `osl import`: inspect SPICE/KiCad-style netlists, Rust-native KiCad schematics, and LTspice schematics, then generate an import compatibility report and runnable NekoSpice project.
- `osl kicad-inspect`: parse KiCad `.kicad_sch` / `.kicad_sym` assets through the Rust-native KiCad IR and emit a JSON summary.
- `osl waveform`: query raw waveforms into viewport-sized min/max envelope JSON.
- HTML and JSON reports for runs and verification batches.
- Run artifacts include `waveform.raw`, `waveform.csv`, and `waveform-summary.json`.
- Failure drilldown with failed checks, waveform summaries, parameters, logs, netlists, and waveform artifacts.
- Measurement checks over ngspice binary or ASCII `waveform.raw`: `final_value`, `avg`, `min`, `max`, `pp`, `rms`.

## Requirements

- Rust stable
- ngspice 46 or compatible ngspice CLI

## Quick Start

```bash
cargo run -p osl-cli -- --version
cargo run -p osl-cli -- run examples/rc_filter/rc.cir --output runs/rc_001
cargo run -p osl-cli -- verify examples/basic_validation.osl.yaml --jobs 3 --output reports/basic_001
cargo run -p osl-cli -- verify examples/structured_validation.osl.yaml --jobs 3 --output reports/structured_001
cargo run -p osl-cli -- bench examples --output bench-results/basic_001
cargo run -p osl-cli -- model-check examples/diode_rectifier/rectifier.cir --output reports/modelcheck_001
cargo run -p osl-cli -- model-check examples/pin_mapping/good_opamp.lib --symbol examples/pin_mapping/good_opamp.asy --output reports/pinmap_001
cargo run -p osl-cli -- import examples/kicad_import/kicad_rc.cir --output reports/import_001
cargo run -p osl-cli -- import examples/kicad_schematic/rc.kicad_sch --output reports/import_kicad_schematic_001
cargo run -p osl-cli -- import examples/kicad_import/kicad_diode_include.cir --output reports/import_with_models_001
cargo run -p osl-cli -- kicad-inspect examples/kicad_schematic/rc.kicad_sch --output reports/kicad_schematic.json
cargo run -p osl-cli -- kicad-inspect examples/kicad_schematic/neko_spice.kicad_sym --output reports/kicad_symbol_library.json
cargo run -p osl-cli -- waveform runs/rc_001/waveform.raw --signal v(out) --from 8us --to 10us --points 200 --output reports/vout-envelope.json
```

## Verification Config

```yaml
project: basic_validation

runs:
  - name: rc_filter
    netlist: rc_filter/rc.cir
    sweep:
      rload: [500, 1000, 2000]
    checks:
      - name: average_output
        kind: avg
        signal: v(out)
        from: 8us
        to: 10us
        min: 0.45
        max: 0.50
```

Each sweep dimension expands into a Cartesian product of ngspice runs. `--jobs <n>` runs independent cases concurrently. Parameters are injected as `.param` overrides in the working netlist and recorded in `run.json` and `verify.json`; reports are sorted by the original expansion order. Checks can use optional `from` / `to` windows with SPICE-style suffixes such as `8us`, `3ms`, or `1k`. Verification reports include a compact summary for each evaluated signal window: sample count, first/last value, min/max, average, peak-to-peak, and RMS.

Verification files are parsed with `serde_yaml`, so normal YAML forms such as quoted strings, flow-style maps/lists, and numeric values with SPICE suffix strings are accepted.

The ngspice runner automatically injects a binary raw export into the working netlist:

```spice
set filetype=binary
write waveform.raw all
```

Checks can target any signal present in the raw variable table, such as `v(out)` or `i(v1)`. The waveform reader auto-detects ngspice `Binary:` and `Values:` raw files, so older ASCII artifacts remain readable. Every successful run exports `waveform.csv` for external tools and `waveform-summary.json` with per-signal sample count, first/last, min/max, average, peak-to-peak, and RMS.

For UI and plotting workflows, `osl waveform <waveform.raw> --signal <name>` returns min/max envelope buckets for a requested viewport. Use `--from`, `--to`, and `--points` to control the time window and target bucket count.

## Model Check

```bash
cargo run -p osl-cli -- model-check examples/vendor_model_issues --output /tmp/nekospice_modelcheck/bad
cargo run -p osl-cli -- model-check examples/pin_mapping/good_opamp.lib --symbol examples/pin_mapping/bad_opamp.asy --output /tmp/nekospice_modelcheck/pinmap_bad
```

`model-check` writes `model-check.json` and `report.html`. It extracts `.subckt` names and pin lists, indexes `.model` statements, flags unsupported or dialect-specific directives, and returns exit code `2` when error-level diagnostics are found. With `--symbol <ltspice.asy>`, it parses LTspice `PINATTR PinName` / `SpiceOrder` entries and verifies that symbol pin order matches the target `.subckt` pin list.

## Import Report

```bash
cargo run -p osl-cli -- import examples/kicad_import/kicad_rc.cir --output /tmp/nekospice_import/kicad_rc
cargo run -p osl-cli -- import examples/kicad_project --output /tmp/nekospice_import/kicad_project_dir
cargo run -p osl-cli -- import examples/kicad_project/kicad_project.kicad_pro --output /tmp/nekospice_import/kicad_project_file
cargo run -p osl-cli -- import examples/ltspice_import/ltspice_rc.asc --output /tmp/nekospice_import/ltspice_rc
cargo run -p osl-cli -- verify /tmp/nekospice_import/kicad_rc/project/project.osl.yaml --output /tmp/nekospice_import/kicad_rc_verify
```

`import` writes `import.json`, `report.html`, and a normalized `project/` directory. The project contains `input.cir`, `project.osl.yaml`, and `manifest.json`, so imported KiCad/LTspice/generic SPICE netlists can be handed directly to `osl verify`. KiCad/generic SPICE netlists are normalized directly. KiCad project directories and `.kicad_pro` files are accepted when they contain an exported SPICE netlist (`.cir`, `.spice`, or `.sp`); relative `.include` paths are resolved from the discovered netlist. LTspice `.asc` schematics have a first-pass importer for `WIRE`, `FLAG`, `TEXT ... !<directive>`, local and searched `.asy` pin mapping, subcircuit symbols with `Prefix X`, and common primitive fallback symbols (`res`, `cap`, `ind`, `voltage`, `current`, diode-family, BJT, MOSFET, JFET, controlled-source, and switch symbols). Symbol search checks the schematic directory, `sym/` below it, `NEKOSPICE_LTSPICE_SYM_PATH`, and common LTspice installation paths. Unsupported symbols are reported with line-level diagnostics instead of silently producing a broken netlist. Relative `.include`, `.inc`, and `.lib` dependencies are copied into `project/models/` and referenced from the normalized netlist. The generated validation file keeps `checks: []` for a smoke run, then adds commented check templates derived from observable node voltages and voltage-source currents. The manifest stores the same `suggested_signals` and `suggested_checks` as machine-readable JSON for future GUI/project tooling. The compatibility report counts components, symbols, directives, includes, and emits diagnostics before the netlist is handed to ngspice.

## KiCad Schematic And Library IR

```bash
cargo run -p osl-cli -- kicad-inspect examples/kicad_schematic/rc.kicad_sch
cargo run -p osl-cli -- kicad-inspect examples/kicad_schematic/neko_spice.kicad_sym
```

`osl-kicad` is the Rust-native KiCad-compatible foundation. It parses KiCad S-expression assets into schematic and symbol-library IR, covering schematic symbols, embedded library symbols, wires, labels, text/SPICE directives, junctions, symbol properties, pins, symbol graphics, symbol bounding boxes, and placement metadata. The local KiCad source mirror is treated only as reference material and is ignored by Git.

## Validation

```bash
cargo fmt --check
cargo test --workspace
cargo run -p osl-cli -- verify examples/basic_validation.osl.yaml --jobs 3 --output /tmp/nekospice_reports/basic
cargo run -p osl-cli -- verify examples/structured_validation.osl.yaml --jobs 3 --output /tmp/nekospice_reports/structured
cargo run -p osl-cli -- import examples/kicad_import/kicad_rc.cir --output /tmp/nekospice_import/kicad_rc
cargo run -p osl-cli -- verify /tmp/nekospice_import/kicad_rc/project/project.osl.yaml --output /tmp/nekospice_import/kicad_rc_verify
cargo run -p osl-cli -- import examples/kicad_project --output /tmp/nekospice_import/kicad_project_dir
cargo run -p osl-cli -- verify /tmp/nekospice_import/kicad_project_dir/project/project.osl.yaml --output /tmp/nekospice_import/kicad_project_dir_verify
cargo run -p osl-cli -- import examples/kicad_project/kicad_project.kicad_pro --output /tmp/nekospice_import/kicad_project_file
cargo run -p osl-cli -- verify /tmp/nekospice_import/kicad_project_file/project/project.osl.yaml --output /tmp/nekospice_import/kicad_project_file_verify
cargo run -p osl-cli -- import examples/kicad_schematic/rc.kicad_sch --output /tmp/nekospice_import/kicad_schematic
cargo run -p osl-cli -- verify /tmp/nekospice_import/kicad_schematic/project/project.osl.yaml --output /tmp/nekospice_import/kicad_schematic_verify
cargo run -p osl-cli -- kicad-inspect examples/kicad_schematic/rc.kicad_sch --output /tmp/nekospice_import/kicad_schematic.json
cargo run -p osl-cli -- kicad-inspect examples/kicad_schematic/neko_spice.kicad_sym --output /tmp/nekospice_import/kicad_symbol_library.json
cargo run -p osl-cli -- import examples/ltspice_import/ltspice_rc.asc --output /tmp/nekospice_import/ltspice_rc
cargo run -p osl-cli -- verify /tmp/nekospice_import/ltspice_rc/project/project.osl.yaml --output /tmp/nekospice_import/ltspice_rc_verify
cargo run -p osl-cli -- import examples/kicad_import/kicad_diode_include.cir --output /tmp/nekospice_import/kicad_with_models
cargo run -p osl-cli -- verify /tmp/nekospice_import/kicad_with_models/project/project.osl.yaml --output /tmp/nekospice_import/kicad_with_models_verify
cargo run -p osl-cli -- waveform /tmp/nekospice_reports/basic/runs/rc_filter/waveform.raw --signal 'v(out)' --points 100 --output /tmp/nekospice_reports/basic/vout-envelope.json
bash -lc 'cargo run -p osl-cli -- verify examples/failing_validation.osl.yaml --output /tmp/nekospice_reports/failing; test $? -eq 2'
bash -lc 'cargo run -p osl-cli -- model-check examples/vendor_model_issues --output /tmp/nekospice_modelcheck/bad; test $? -eq 2'
bash -lc 'cargo run -p osl-cli -- model-check examples/pin_mapping/good_opamp.lib --symbol examples/pin_mapping/bad_opamp.asy --output /tmp/nekospice_modelcheck/pinmap_bad; test $? -eq 2'
```
