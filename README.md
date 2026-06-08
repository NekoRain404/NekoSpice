# NekoSpice

NekoSpice is a Rust-first SPICE automation tool that uses ngspice for circuit solving and Rust for repeatable runs, measurements, reports, and batch verification.

The current three-day build is a vertical slice:

- `osl run`: run one `.cir` file through ngspice.
- `osl verify`: run a small YAML verification plan.
- `osl bench`: run every `.cir` under a directory and collect timings.
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

## Validation

```bash
cargo fmt --check
cargo test --workspace
cargo run -p osl-cli -- verify examples/basic_validation.osl.yaml --jobs 3 --output /tmp/nekospice_reports/basic
bash -lc 'cargo run -p osl-cli -- verify examples/failing_validation.osl.yaml --output /tmp/nekospice_reports/failing; test $? -eq 2'
```
