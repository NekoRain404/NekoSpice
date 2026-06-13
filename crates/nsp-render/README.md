# osl-render

SVG renderer for schematic canvas scenes. Used for headless visual review and CI artifact generation.

## Capabilities

- Renders `NspCanvasScene` to SVG format
- Symbol, wire, bus, label, graphic, and text rendering
- Sheet boundary visualization
- Junction and no-connect marker rendering

## Usage

```rust
use osl_render::render_scene_to_svg;
let svg = render_scene_to_svg(&scene, &options)?;
```
