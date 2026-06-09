# osl-report

`osl-report` owns report data transfer objects and HTML/JSON/JUnit rendering for
NekoSpice workflows. It does not run simulators, parse command-line arguments,
or evaluate measurements; callers pass already computed run metadata and check
results into this crate.

## Boundaries

- CLI and GUI code may build report DTOs and write rendered output.
- Simulator execution and artifact finalization stay in `osl-sim`.
- Waveform parsing and numeric measurement stay in `osl-waveform` and the
  verification runner.
- Report templates, shared CSS, artifact links, JSON projection, and JUnit XML
  stay here so CLI, GUI, and CI exporters do not duplicate presentation logic.
