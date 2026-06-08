# NekoSpice

NekoSpice is a Rust-first SPICE automation tool that uses ngspice for circuit solving and Rust for repeatable runs, measurements, reports, and batch verification.

The current three-day build is a vertical slice:

- `osl run`: run one `.cir` file through ngspice.
- `osl verify`: run a small YAML verification plan.
- `osl bench`: run every `.cir` under a directory and collect timings.
- HTML and JSON reports for runs and verification batches.
- Measurement checks over `waveform.csv`: `final_value`, `avg`, `min`, `max`, `pp`, `rms`.

## Requirements

- Rust stable
- ngspice 46 or compatible ngspice CLI

## Quick Start

```bash
cargo run -p osl-cli -- --version
cargo run -p osl-cli -- run examples/rc_filter/rc.cir --output runs/rc_001
cargo run -p osl-cli -- verify examples/basic_validation.osl.yaml --output reports/basic_001
cargo run -p osl-cli -- bench examples --output bench-results/basic_001
```

## Verification Config

```yaml
project: basic_validation

runs:
  - name: rc_filter
    netlist: rc_filter/rc.cir
    checks:
      - name: average_output
        kind: avg
        signal: v(out)
        min: 0.45
        max: 0.50
```

The current waveform reader assumes each example writes:

```spice
wrdata waveform.csv time v(in) v(out)
```

This keeps the first build fast and deterministic. The next implementation step is a real ngspice raw parser so checks no longer depend on that `wrdata` convention.

## Validation

```bash
cargo fmt --check
cargo test --workspace
cargo run -p osl-cli -- verify examples/basic_validation.osl.yaml --output /tmp/nekospice_reports/basic
```

