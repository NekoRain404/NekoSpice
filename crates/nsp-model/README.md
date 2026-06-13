# osl-model

SPICE model checking and validation library.

## Capabilities

- `.subckt` / `.model` directive parsing and validation
- LTspice symbol pin mapping analysis
- Dialect risk detection (ngspice vs LTspice incompatibilities)
- Unsupported directive warnings
- Vendor model issue surfacing

## Usage

```rust
use osl_model::check_models;
let report = check_models("path/to/models/")?;
```
