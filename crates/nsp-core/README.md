# osl-core

Shared types, error handling, and utility functions used across all NekoSpice crates.

## Role

This is the foundation crate. Every other `osl-*` crate depends on it for:

- Common error types (`OslError`, `OslResult`)
- Shared enums and data structures
- Utility helpers (path normalization, unit conversion, etc.)

## Dependencies

Zero external domain dependencies. Only standard library and lightweight utility crates.

## Usage

```rust
use osl_core::{OslError, OslResult};
```
