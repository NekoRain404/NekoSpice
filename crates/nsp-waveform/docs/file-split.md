# osl-waveform File Split

## Overview

The `osl-waveform` crate provides ngspice raw file parsing, waveform data types, viewport queries, and measurement functions. The original `lib.rs` (1047 lines) was split into two files for maintainability.

## File Structure

| File | Lines | Purpose |
|------|-------|---------|
| `src/lib.rs` | ~615 | Core types (`Waveform`, `WaveformViewportQuery`, `WaveformEnvelope`, `MeasurementKind`, `WaveformSummary`) and measurement functions |
| `src/raw_parser_impl.rs` | ~436 | ngspice raw file parsing: ASCII raw, binary raw, header parsing, value parsing, helpers |

## Inclusion Pattern

`raw_parser_impl.rs` is included via `include!("raw_parser_impl.rs");` in `lib.rs`. This is a textual include that shares the parent module's scope, so the raw parser can directly reference `Waveform`, `WaveformVariable`, and other types defined in `lib.rs`.

## Key Functions in raw_parser_impl.rs

- `read_ngspice_raw` / `parse_ngspice_raw` — Entry points for raw file parsing
- `parse_ngspice_ascii_raw` — ASCII format parser
- `parse_ngspice_binary_raw` — Binary format parser (little-endian f64)
- `parse_header` / `parse_variables` / `parse_values` — Internal parsing stages
- `find_section_payload_offset` — Binary section marker search

## Dependencies

- `osl_core::{OslError, OslResult, read_text}`
- `std::fs`, `std::path::Path`, `std::str`
