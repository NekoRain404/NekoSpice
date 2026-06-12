# osl-sim

Simulation backend runners for ngspice and Xyce.

## Backends

| Backend | Description |
|---------|-------------|
| `NgspiceCliBackend` | Runs simulations via ngspice CLI |
| `XyceCliBackend` | Runs simulations via Xyce CLI with automatic netlist preprocessing |

## Capabilities

- Automatic `.control`/`.endc` to Xyce `.print` directive conversion
- Simulation directive extraction from KiCad schematics
- Run artifact collection (`.raw`, `.csv`, waveform summaries)
- Multi-backend dispatch via `SimulatorBackend` trait

## Usage

```rust
use osl_sim::{NgspiceCliBackend, XyceCliBackend, SimulatorBackend};
let backend = NgspiceCliBackend::new();
let result = backend.run("circuit.cir", "output_dir")?;
```
