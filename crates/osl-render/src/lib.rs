use osl_core::html_escape;
use osl_kicad::{
    KicadAt, KicadBoundingBox, KicadCanvasGraphic, KicadCanvasScene, KicadCanvasSheet,
    KicadCanvasSymbol, KicadLabelKind, KicadPoint,
};

const DEFAULT_PADDING_MM: f64 = 6.0;
const DEFAULT_SCALE: f64 = 18.0;

#[derive(Debug, Clone, Copy)]
pub struct SvgRenderOptions {
    pub padding_mm: f64,
    pub scale: f64,
    pub show_grid: bool,
}

impl Default for SvgRenderOptions {
    fn default() -> Self {
        Self {
            padding_mm: DEFAULT_PADDING_MM,
            scale: DEFAULT_SCALE,
            show_grid: true,
        }
    }
}

pub fn render_kicad_scene_svg(scene: &KicadCanvasScene) -> String {
    render_kicad_scene_svg_with_options(scene, SvgRenderOptions::default())
}

pub fn render_kicad_scene_svg_with_options(
    scene: &KicadCanvasScene,
    options: SvgRenderOptions,
) -> String {
    let bounds = scene.bounds.unwrap_or(KicadBoundingBox {
        min: KicadPoint { x: 0.0, y: 0.0 },
        max: KicadPoint { x: 20.0, y: 20.0 },
    });
    let viewport = SvgViewport::new(bounds, options);
    let mut output = String::new();

    output.push_str(&format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{:.0}" height="{:.0}" viewBox="0 0 {:.3} {:.3}" role="img" aria-label="{}">"#,
        viewport.width_px,
        viewport.height_px,
        viewport.width_px,
        viewport.height_px,
        html_escape(&format!("KiCad schematic {}", scene.source))
    ));
    output.push('\n');
    output.push_str("  <rect width=\"100%\" height=\"100%\" fill=\"#f8fafc\"/>\n");
    if options.show_grid {
        render_grid(&mut output, &viewport);
    }
    output.push_str("  <g fill=\"none\" stroke-linecap=\"round\" stroke-linejoin=\"round\">\n");
    for wire in &scene.wires {
        render_polyline(&mut output, &viewport, &wire.points, "#0f172a", 2.0);
    }
    for sheet in &scene.sheets {
        render_sheet(&mut output, &viewport, sheet);
    }
    for symbol in &scene.symbols {
        render_symbol(&mut output, &viewport, symbol);
    }
    for junction in &scene.junctions {
        let point = viewport.project(junction.at);
        output.push_str(&format!(
            "    <circle cx=\"{}\" cy=\"{}\" r=\"3\" fill=\"#0f172a\" stroke=\"none\"/>\n",
            fmt(point.x),
            fmt(point.y)
        ));
    }
    output.push_str("  </g>\n");
    output.push_str("  <g font-family=\"ui-monospace, SFMono-Regular, Menlo, Consolas, monospace\" font-size=\"11\" fill=\"#334155\">\n");
    for label in &scene.labels {
        if let Some(at) = label.at {
            let point = viewport.project(at_point(at));
            let fill = match label.kind {
                KicadLabelKind::Local => "#0369a1",
                KicadLabelKind::Global => "#7c3aed",
                KicadLabelKind::Hierarchical => "#b45309",
            };
            output.push_str(&format!(
                "    <text x=\"{}\" y=\"{}\" fill=\"{}\">{}</text>\n",
                fmt(point.x + 4.0),
                fmt(point.y - 4.0),
                fill,
                html_escape(&label.text)
            ));
        }
    }
    for symbol in &scene.symbols {
        if !symbol.reference.is_empty() {
            let point = viewport.project(at_point(symbol.at));
            output.push_str(&format!(
                "    <text x=\"{}\" y=\"{}\" fill=\"#0f172a\">{}</text>\n",
                fmt(point.x + 5.0),
                fmt(point.y - 10.0),
                html_escape(&symbol.reference)
            ));
        }
        if !symbol.value.is_empty() {
            let point = viewport.project(at_point(symbol.at));
            output.push_str(&format!(
                "    <text x=\"{}\" y=\"{}\" fill=\"#64748b\">{}</text>\n",
                fmt(point.x + 5.0),
                fmt(point.y + 22.0),
                html_escape(&symbol.value)
            ));
        }
    }
    output.push_str("  </g>\n");
    output.push_str("</svg>\n");
    output
}

fn render_sheet(output: &mut String, viewport: &SvgViewport, sheet: &KicadCanvasSheet) {
    let Some(at) = sheet.at else {
        return;
    };
    let Some(size) = sheet.size else {
        return;
    };
    let origin = viewport.project(at_point(at));
    output.push_str(&format!(
        "    <g data-sheet-name=\"{}\" data-sheet-file=\"{}\">\n",
        html_escape(&sheet.name),
        html_escape(&sheet.file)
    ));
    output.push_str(&format!(
        "      <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" stroke=\"#b45309\" stroke-width=\"1.8\" fill=\"#fef3c7\" fill-opacity=\"0.18\"/>\n",
        fmt(origin.x),
        fmt(origin.y),
        fmt(size.width * viewport.scale),
        fmt(size.height * viewport.scale)
    ));
    if !sheet.name.is_empty() {
        output.push_str(&format!(
            "      <text x=\"{}\" y=\"{}\" fill=\"#92400e\" stroke=\"none\">{}</text>\n",
            fmt(origin.x + 4.0),
            fmt(origin.y - 6.0),
            html_escape(&sheet.name)
        ));
    }
    if !sheet.file.is_empty() {
        output.push_str(&format!(
            "      <text x=\"{}\" y=\"{}\" fill=\"#a16207\" stroke=\"none\">{}</text>\n",
            fmt(origin.x + 4.0),
            fmt(origin.y + size.height * viewport.scale + 14.0),
            html_escape(&sheet.file)
        ));
    }
    for pin in &sheet.pins {
        if let Some(at) = pin.at {
            let start = viewport.project(at_point(at));
            let end = viewport.project(pin_body_end(at, 2.54));
            output.push_str(&format!(
                "      <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#b45309\" stroke-width=\"1.4\"/>\n",
                fmt(start.x),
                fmt(start.y),
                fmt(end.x),
                fmt(end.y)
            ));
            output.push_str(&format!(
                "      <text x=\"{}\" y=\"{}\" fill=\"#92400e\" stroke=\"none\">{}</text>\n",
                fmt(start.x + 4.0),
                fmt(start.y - 4.0),
                html_escape(&pin.name)
            ));
        }
    }
    output.push_str("    </g>\n");
}

fn render_symbol(output: &mut String, viewport: &SvgViewport, symbol: &KicadCanvasSymbol) {
    output.push_str(&format!(
        "    <g data-lib-id=\"{}\" data-reference=\"{}\">\n",
        html_escape(&symbol.lib_id),
        html_escape(&symbol.reference)
    ));
    for graphic in &symbol.graphics {
        match graphic {
            KicadCanvasGraphic::Polyline { points } => {
                render_polyline(output, viewport, points, "#1d4ed8", 1.8);
            }
            KicadCanvasGraphic::Rectangle { start, end } => {
                let left_top = viewport.project(KicadPoint {
                    x: start.x.min(end.x),
                    y: start.y.min(end.y),
                });
                let right_bottom = viewport.project(KicadPoint {
                    x: start.x.max(end.x),
                    y: start.y.max(end.y),
                });
                output.push_str(&format!(
                    "      <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" stroke=\"#1d4ed8\" stroke-width=\"1.8\" fill=\"#dbeafe\" fill-opacity=\"0.25\"/>\n",
                    fmt(left_top.x),
                    fmt(left_top.y),
                    fmt((right_bottom.x - left_top.x).abs()),
                    fmt((right_bottom.y - left_top.y).abs())
                ));
            }
            KicadCanvasGraphic::Circle { center, radius } => {
                let center = viewport.project(*center);
                output.push_str(&format!(
                    "      <circle cx=\"{}\" cy=\"{}\" r=\"{}\" stroke=\"#1d4ed8\" stroke-width=\"1.8\" fill=\"none\"/>\n",
                    fmt(center.x),
                    fmt(center.y),
                    fmt(radius * viewport.scale)
                ));
            }
            KicadCanvasGraphic::Arc { start, mid, end } => {
                let mut points = vec![*start];
                if let Some(mid) = mid {
                    points.push(*mid);
                }
                points.push(*end);
                render_polyline(output, viewport, &points, "#1d4ed8", 1.8);
            }
            KicadCanvasGraphic::Text { text, at } => {
                if let Some(at) = at {
                    let point = viewport.project(at_point(*at));
                    output.push_str(&format!(
                        "      <text x=\"{}\" y=\"{}\" fill=\"#1d4ed8\" stroke=\"none\">{}</text>\n",
                        fmt(point.x),
                        fmt(point.y),
                        html_escape(text)
                    ));
                }
            }
        }
    }
    for pin in &symbol.pins {
        let start = viewport.project(pin.start);
        let end = viewport.project(pin.end);
        output.push_str(&format!(
            "      <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#475569\" stroke-width=\"1.4\"/>\n",
            fmt(start.x),
            fmt(start.y),
            fmt(end.x),
            fmt(end.y)
        ));
    }
    output.push_str("    </g>\n");
}

fn render_polyline(
    output: &mut String,
    viewport: &SvgViewport,
    points: &[KicadPoint],
    color: &str,
    stroke_width: f64,
) {
    if points.len() < 2 {
        return;
    }
    let points = points
        .iter()
        .map(|point| {
            let point = viewport.project(*point);
            format!("{},{}", fmt(point.x), fmt(point.y))
        })
        .collect::<Vec<_>>()
        .join(" ");
    output.push_str(&format!(
        "      <polyline points=\"{}\" stroke=\"{}\" stroke-width=\"{}\"/>\n",
        points,
        color,
        fmt(stroke_width)
    ));
}

fn render_grid(output: &mut String, viewport: &SvgViewport) {
    output.push_str("  <g stroke=\"#e2e8f0\" stroke-width=\"1\">\n");
    let step = 2.54 * viewport.scale;
    let mut x = viewport.padding_px % step;
    while x <= viewport.width_px {
        output.push_str(&format!(
            "    <line x1=\"{}\" y1=\"0\" x2=\"{}\" y2=\"{}\"/>\n",
            fmt(x),
            fmt(x),
            fmt(viewport.height_px)
        ));
        x += step;
    }
    let mut y = viewport.padding_px % step;
    while y <= viewport.height_px {
        output.push_str(&format!(
            "    <line x1=\"0\" y1=\"{}\" x2=\"{}\" y2=\"{}\"/>\n",
            fmt(y),
            fmt(viewport.width_px),
            fmt(y)
        ));
        y += step;
    }
    output.push_str("  </g>\n");
}

#[derive(Debug, Clone, Copy)]
struct SvgViewport {
    min: KicadPoint,
    width_px: f64,
    height_px: f64,
    padding_px: f64,
    scale: f64,
}

impl SvgViewport {
    fn new(bounds: KicadBoundingBox, options: SvgRenderOptions) -> Self {
        let width_mm = bounds.width().max(1.0) + 2.0 * options.padding_mm;
        let height_mm = bounds.height().max(1.0) + 2.0 * options.padding_mm;
        Self {
            min: bounds.min,
            width_px: width_mm * options.scale,
            height_px: height_mm * options.scale,
            padding_px: options.padding_mm * options.scale,
            scale: options.scale,
        }
    }

    fn project(self, point: KicadPoint) -> SvgPoint {
        SvgPoint {
            x: (point.x - self.min.x) * self.scale + self.padding_px,
            y: (point.y - self.min.y) * self.scale + self.padding_px,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct SvgPoint {
    x: f64,
    y: f64,
}

fn fmt(value: f64) -> String {
    let normalized = if value == -0.0 { 0.0 } else { value };
    let mut formatted = format!("{normalized:.3}");
    while formatted.contains('.') && formatted.ends_with('0') {
        formatted.pop();
    }
    if formatted.ends_with('.') {
        formatted.pop();
    }
    formatted
}

fn at_point(at: KicadAt) -> KicadPoint {
    KicadPoint { x: at.x, y: at.y }
}

fn pin_body_end(at: KicadAt, length: f64) -> KicadPoint {
    let radians = at.rotation.to_radians();
    KicadPoint {
        x: at.x + length * radians.cos(),
        y: at.y + length * radians.sin(),
    }
}

#[cfg(test)]
mod tests {
    use super::render_kicad_scene_svg;
    use osl_kicad::{parse_kicad_schematic, read_kicad_schematic};
    use std::path::Path;

    #[test]
    fn renders_kicad_canvas_scene_to_svg() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let schematic =
            read_kicad_schematic(&workspace_root.join("examples/kicad_schematic/rc.kicad_sch"))
                .unwrap();
        let svg = render_kicad_scene_svg(&schematic.canvas_scene());

        assert!(svg.starts_with("<svg "));
        assert!(svg.contains("data-reference=\"R1\""));
        assert!(svg.contains(">1k</text>"));
        assert!(svg.contains(">in</text>"));
        assert!(svg.contains("<polyline"));
        assert!(svg.ends_with("</svg>\n"));
    }

    #[test]
    fn renders_kicad_hierarchical_sheet_to_svg() {
        let schematic = parse_kicad_schematic(
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (sheet
    (at 20 10)
    (size 15 10)
    (property "Sheetname" "gain_stage" (at 20 9 0))
    (property "Sheetfile" "gain_stage.kicad_sch" (at 20 21 0))
    (pin "in" input (at 20 15 180))
    (pin "out" output (at 35 15 0))
  )
)"#,
            "hierarchical.kicad_sch",
        )
        .unwrap();

        let svg = render_kicad_scene_svg(&schematic.canvas_scene());

        assert!(svg.contains("data-sheet-name=\"gain_stage\""));
        assert!(svg.contains("gain_stage.kicad_sch"));
        assert!(svg.contains(">in</text>"));
        assert!(svg.contains(">out</text>"));
    }
}
