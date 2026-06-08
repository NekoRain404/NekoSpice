# NekoSpice

NekoSpice is a Rust-first SPICE automation tool that uses ngspice for circuit solving and Rust for repeatable runs, measurements, reports, and batch verification.

The current three-day build is a vertical slice:

- `osl run`: run one `.cir` file through ngspice.
- `osl verify`: run a small YAML verification plan.
- `osl bench`: run every `.cir` under a directory and collect timings.
- `osl model-check`: scan imported SPICE models for `.subckt`, `.model`, LTspice symbol pin mapping, dialect risks, and unsupported directives.
- `osl import`: inspect SPICE/KiCad-style netlists and generate an import compatibility report.
- HTML and JSON reports for runs and verification batches.
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
cargo run -p osl-cli -- bench examples --output bench-results/basic_001
cargo run -p osl-cli -- model-check examples/diode_rectifier/rectifier.cir --output reports/modelcheck_001
cargo run -p osl-cli -- model-check examples/pin_mapping/good_opamp.lib --symbol examples/pin_mapping/good_opamp.asy --output reports/pinmap_001
cargo run -p osl-cli -- import examples/kicad_import/kicad_rc.cir --output reports/import_001
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

The ngspice runner automatically injects a binary raw export into the working netlist:

```spice
set filetype=binary
write waveform.raw all
```

Checks can target any signal present in the raw variable table, such as `v(out)` or `i(v1)`. The waveform reader auto-detects ngspice `Binary:` and `Values:` raw files, so older ASCII artifacts remain readable.

## Model Check

```bash
cargo run -p osl-cli -- model-check examples/vendor_model_issues --output /tmp/nekospice_modelcheck/bad
cargo run -p osl-cli -- model-check examples/pin_mapping/good_opamp.lib --symbol examples/pin_mapping/bad_opamp.asy --output /tmp/nekospice_modelcheck/pinmap_bad
```

`model-check` writes `model-check.json` and `report.html`. It extracts `.subckt` names and pin lists, indexes `.model` statements, flags unsupported or dialect-specific directives, and returns exit code `2` when error-level diagnostics are found. With `--symbol <ltspice.asy>`, it parses LTspice `PINATTR PinName` / `SpiceOrder` entries and verifies that symbol pin order matches the target `.subckt` pin list.

## Import Report

```bash
cargo run -p osl-cli -- import examples/kicad_import/kicad_rc.cir --output /tmp/nekospice_import/kicad_rc
cargo run -p osl-cli -- run examples/kicad_import/kicad_rc.cir --output /tmp/nekospice_import/kicad_rc_run
```

`import` writes `import.json` and `report.html`. It detects KiCad/LTspice/generic SPICE flavor, counts components, symbols, directives, includes, and emits compatibility diagnostics before the netlist is handed to ngspice.

## Validation

```bash
cargo fmt --check
cargo test --workspace
cargo run -p osl-cli -- verify examples/basic_validation.osl.yaml --jobs 3 --output /tmp/nekospice_reports/basic
cargo run -p osl-cli -- import examples/kicad_import/kicad_rc.cir --output /tmp/nekospice_import/kicad_rc
bash -lc 'cargo run -p osl-cli -- verify examples/failing_validation.osl.yaml --output /tmp/nekospice_reports/failing; test $? -eq 2'
bash -lc 'cargo run -p osl-cli -- model-check examples/vendor_model_issues --output /tmp/nekospice_modelcheck/bad; test $? -eq 2'
bash -lc 'cargo run -p osl-cli -- model-check examples/pin_mapping/good_opamp.lib --symbol examples/pin_mapping/bad_opamp.asy --output /tmp/nekospice_modelcheck/pinmap_bad; test $? -eq 2'
```
