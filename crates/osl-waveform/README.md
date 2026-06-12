# osl-waveform

Raw waveform data parser for post-simulation analysis.

## Capabilities

- Parse ngspice `.raw` binary and ASCII waveform files
- Extract signal names, values, and sweep data
- Generate viewport-sized min/max envelopes for efficient plotting
- Support for transient, AC, DC sweep, and other analysis types

## Usage

```rust
use osl_waveform::parse_raw_file;
let data = parse_raw_file("waveform.raw")?;
let envelope = data.signal_envelope("v(out)", 100)?;
```
