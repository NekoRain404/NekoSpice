# NekoSpice

NekoSpice is a Rust-first SPICE automation tool that uses ngspice for circuit solving and Rust for repeatable runs, measurements, reports, and batch verification.

The current three-day build is a vertical slice:

- `osl run`: run one `.cir` file through ngspice.
- `osl verify`: run a small YAML verification plan.
- `osl bench`: run every `.cir` under a directory and collect timings.
- HTML and JSON reports for runs and verification batches.
- Measurement checks over ngspice ASCII `waveform.raw`: `final_value`, `avg`, `min`, `max`, `pp`, `rms`.

## Requirements

- Rust stable
- ngspice 46 or compatible ngspice CLI

## Quick Start

```bash
cargo run -p osl-cli -- --version
cargo run -p osl-cli -- run examples/rc_filter/rc.cir --output runs/rc_001
cargo run -p osl-cli -- verify examples/basic_validation.osl.yaml --jobs 3 --output reports/basic_001
cargo run -p osl-cli -- bench examples --output bench-results/basic_001
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
        min: 0.45
        max: 0.50
```

Each sweep dimension expands into a Cartesian product of ngspice runs. `--jobs <n>` runs independent cases concurrently. Parameters are injected as `.param` overrides in the working netlist and recorded in `run.json` and `verify.json`; reports are sorted by the original expansion order.

The ngspice runner automatically injects an ASCII raw export into the working netlist:

```spice
set filetype=ascii
write waveform.raw all
```

Checks can target any signal present in the raw variable table, such as `v(out)` or `i(v1)`.

## Validation

```bash
cargo fmt --check
cargo test --workspace
cargo run -p osl-cli -- verify examples/basic_validation.osl.yaml --jobs 3 --output /tmp/nekospice_reports/basic
```
