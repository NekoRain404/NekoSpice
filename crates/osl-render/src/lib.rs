use osl_core::html_escape;
use osl_kicad::{
    KicadAt, KicadBoundingBox, KicadCanvasGraphic, KicadCanvasImage, KicadCanvasScene,
    KicadCanvasSheet, KicadCanvasSymbol, KicadCanvasTable, KicadCanvasTextBox, KicadColor,
    KicadFill, KicadLabelKind, KicadPoint, KicadStroke,
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
    for graphic in &scene.graphics {
        render_graphic(&mut output, &viewport, graphic, "#64748b", "#e2e8f0");
    }
    for image in &scene.images {
        render_image(&mut output, &viewport, image);
    }
    for table in &scene.tables {
        render_table(&mut output, &viewport, table);
    }
    for bus in &scene.buses {
        render_polyline(&mut output, &viewport, &bus.points, "#2563eb", 3.2);
    }
    for wire in &scene.wires {
        render_polyline(&mut output, &viewport, &wire.points, "#0f172a", 2.0);
    }
    for entry in &scene.bus_entries {
        let start = viewport.project(entry.at);
        let end = viewport.project(entry.end());
        output.push_str(&format!(
            "    <line data-bus-entry=\"true\" x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#2563eb\" stroke-width=\"2\"/>\n",
            fmt(start.x),
            fmt(start.y),
            fmt(end.x),
            fmt(end.y)
        ));
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
    for marker in &scene.no_connects {
        let point = viewport.project(marker.at);
        output.push_str(&format!(
            "    <g data-no-connect=\"true\" stroke=\"#dc2626\" stroke-width=\"1.8\">\n      <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\"/>\n      <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\"/>\n    </g>\n",
            fmt(point.x - 4.0),
            fmt(point.y - 4.0),
            fmt(point.x + 4.0),
            fmt(point.y + 4.0),
            fmt(point.x - 4.0),
            fmt(point.y + 4.0),
            fmt(point.x + 4.0),
            fmt(point.y - 4.0)
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
    for item in &scene.text_items {
        if let Some(at) = item.at {
            let point = viewport.project(at_point(at));
            let fill = if item.is_spice_directive {
                "#b91c1c"
            } else {
                "#475569"
            };
            output.push_str(&format!(
                "    <text data-schematic-text=\"true\" x=\"{}\" y=\"{}\" fill=\"{}\">",
                fmt(point.x),
                fmt(point.y),
                fill
            ));
            for (index, line) in item.text.lines().enumerate() {
                if index == 0 {
                    output.push_str(&html_escape(line));
                } else {
                    output.push_str(&format!(
                        "<tspan x=\"{}\" dy=\"13\">{}</tspan>",
                        fmt(point.x),
                        html_escape(line)
                    ));
                }
            }
            if item.text.ends_with('\n') {
                output.push_str(&format!("<tspan x=\"{}\" dy=\"13\"></tspan>", fmt(point.x)));
            }
            output.push_str("</text>\n");
        }
    }
    for text_box in &scene.text_boxes {
        render_text_box(&mut output, &viewport, text_box);
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
        render_graphic(output, viewport, graphic, "#1d4ed8", "#dbeafe");
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

fn render_text_box(output: &mut String, viewport: &SvgViewport, text_box: &KicadCanvasTextBox) {
    let Some(at) = text_box.at else {
        return;
    };
    let origin = viewport.project(at_point(at));
    let margin = text_box
        .margins
        .map(|margins| margins.left.max(0.0) * viewport.scale)
        .unwrap_or(6.0);

    output.push_str("    <g data-text-box=\"true\">\n");
    if let Some(size) = text_box.size {
        let stroke = svg_stroke_color(text_box.stroke.as_ref(), "#64748b");
        let stroke_width = svg_stroke_width(text_box.stroke.as_ref(), viewport, 1.4);
        let dash_array = svg_stroke_dasharray(text_box.stroke.as_ref());
        let fill = svg_fill_color(text_box.fill.as_ref(), "#fef3c7");
        let fill_opacity = if fill == "none" { "1" } else { "0.22" };
        output.push_str(&format!(
            "      <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"2\" stroke=\"{}\" stroke-width=\"{}\"{} fill=\"{}\" fill-opacity=\"{}\"/>\n",
            fmt(origin.x),
            fmt(origin.y),
            fmt(size.width.abs() * viewport.scale),
            fmt(size.height.abs() * viewport.scale),
            stroke,
            fmt(stroke_width),
            dash_array,
            fill,
            fill_opacity
        ));
    }

    let text_x = origin.x + margin;
    let mut text_y = origin.y + margin + 11.0;
    let text_fill = text_box
        .effects
        .as_ref()
        .and_then(|effects| effects.font_color)
        .map(svg_color)
        .unwrap_or_else(|| "#334155".to_string());
    output.push_str(&format!(
        "      <text x=\"{}\" y=\"{}\" fill=\"{}\" stroke=\"none\">",
        fmt(text_x),
        fmt(text_y),
        text_fill
    ));
    for (index, line) in text_box.text.lines().enumerate() {
        if index == 0 {
            output.push_str(&html_escape(line));
        } else {
            text_y += 13.0;
            output.push_str(&format!(
                "<tspan x=\"{}\" y=\"{}\">{}</tspan>",
                fmt(text_x),
                fmt(text_y),
                html_escape(line)
            ));
        }
    }
    if text_box.text.ends_with('\n') {
        text_y += 13.0;
        output.push_str(&format!(
            "<tspan x=\"{}\" y=\"{}\"></tspan>",
            fmt(text_x),
            fmt(text_y)
        ));
    }
    output.push_str("</text>\n");
    output.push_str("    </g>\n");
}

fn svg_color(color: KicadColor) -> String {
    format!(
        "rgba({},{},{},{})",
        fmt(color.red.clamp(0.0, 255.0)),
        fmt(color.green.clamp(0.0, 255.0)),
        fmt(color.blue.clamp(0.0, 255.0)),
        fmt(color.alpha.clamp(0.0, 1.0))
    )
}

fn svg_stroke_color(stroke: Option<&KicadStroke>, default: &str) -> String {
    stroke
        .and_then(|stroke| stroke.color)
        .map(svg_color)
        .unwrap_or_else(|| default.to_string())
}

fn svg_stroke_width(stroke: Option<&KicadStroke>, viewport: &SvgViewport, default: f64) -> f64 {
    stroke
        .and_then(|stroke| stroke.width)
        .filter(|width| width.is_finite() && *width > 0.0)
        .map(|width| (width * viewport.scale).max(1.0))
        .unwrap_or(default)
}

fn svg_stroke_dasharray(stroke: Option<&KicadStroke>) -> String {
    match stroke
        .and_then(|stroke| stroke.stroke_type.as_deref())
        .unwrap_or("default")
    {
        "dash" => " stroke-dasharray=\"8 5\"".to_string(),
        "dot" => " stroke-dasharray=\"2 5\"".to_string(),
        "dash_dot" => " stroke-dasharray=\"8 4 2 4\"".to_string(),
        "dash_dot_dot" => " stroke-dasharray=\"8 4 2 4 2 4\"".to_string(),
        _ => String::new(),
    }
}

fn svg_fill_color(fill: Option<&KicadFill>, default: &str) -> String {
    let Some(fill) = fill else {
        return default.to_string();
    };
    if fill
        .fill_type
        .as_deref()
        .map(|fill_type| fill_type.eq_ignore_ascii_case("none"))
        .unwrap_or(false)
    {
        return "none".to_string();
    }
    fill.color
        .map(svg_color)
        .unwrap_or_else(|| default.to_string())
}

fn render_image(output: &mut String, viewport: &SvgViewport, image: &KicadCanvasImage) {
    let Some(center) = image.at else {
        return;
    };
    let Some(size) = image.image_size else {
        return;
    };
    if image.data_base64.is_empty() {
        return;
    }

    let center = viewport.project(center);
    let width = size.width * viewport.scale;
    let height = size.height * viewport.scale;
    output.push_str(&format!(
        "    <image data-kicad-image=\"true\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" href=\"data:{};base64,{}\"/>\n",
        fmt(center.x - width / 2.0),
        fmt(center.y - height / 2.0),
        fmt(width),
        fmt(height),
        html_escape(&image.mime_type),
        html_escape(&image.data_base64)
    ));
}

fn render_table(output: &mut String, viewport: &SvgViewport, table: &KicadCanvasTable) {
    if table.cells.is_empty() {
        return;
    }

    output.push_str(&format!(
        "    <g data-kicad-table=\"true\" data-column-count=\"{}\">\n",
        table.column_count
    ));
    for cell in &table.cells {
        let Some(at) = cell.at else {
            continue;
        };
        let Some(size) = cell.size else {
            continue;
        };
        let origin = viewport.project(at_point(at));
        let width = size.width * viewport.scale;
        let height = size.height * viewport.scale;
        output.push_str(&format!(
            "      <rect data-table-cell=\"true\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" stroke=\"#64748b\" stroke-width=\"1\" fill=\"#ffffff\" fill-opacity=\"0.55\"/>\n",
            fmt(origin.x),
            fmt(origin.y),
            fmt(width),
            fmt(height)
        ));
        if !cell.text.is_empty() {
            let margin = cell
                .margins
                .map(|margins| margins.left.max(0.0) * viewport.scale)
                .unwrap_or(4.0);
            output.push_str(&format!(
                "      <text x=\"{}\" y=\"{}\" fill=\"#334155\" stroke=\"none\">{}</text>\n",
                fmt(origin.x + margin),
                fmt(origin.y + margin + 10.0),
                html_escape(&cell.text)
            ));
        }
    }
    output.push_str("    </g>\n");
}

fn render_graphic(
    output: &mut String,
    viewport: &SvgViewport,
    graphic: &KicadCanvasGraphic,
    stroke: &str,
    fill: &str,
) {
    match graphic {
        KicadCanvasGraphic::Polyline { points } => {
            render_polyline(output, viewport, points, stroke, 1.8);
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
                "      <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" stroke=\"{}\" stroke-width=\"1.8\" fill=\"{}\" fill-opacity=\"0.25\"/>\n",
                fmt(left_top.x),
                fmt(left_top.y),
                fmt((right_bottom.x - left_top.x).abs()),
                fmt((right_bottom.y - left_top.y).abs()),
                stroke,
                fill
            ));
        }
        KicadCanvasGraphic::Circle { center, radius } => {
            let center = viewport.project(*center);
            output.push_str(&format!(
                "      <circle cx=\"{}\" cy=\"{}\" r=\"{}\" stroke=\"{}\" stroke-width=\"1.8\" fill=\"none\"/>\n",
                fmt(center.x),
                fmt(center.y),
                fmt(radius * viewport.scale),
                stroke
            ));
        }
        KicadCanvasGraphic::Arc { start, mid, end } => {
            let mut points = vec![*start];
            if let Some(mid) = mid {
                points.push(*mid);
            }
            points.push(*end);
            render_polyline(output, viewport, &points, stroke, 1.8);
        }
        KicadCanvasGraphic::Text { text, at } => {
            if let Some(at) = at {
                let point = viewport.project(at_point(*at));
                output.push_str(&format!(
                    "      <text x=\"{}\" y=\"{}\" fill=\"{}\" stroke=\"none\">{}</text>\n",
                    fmt(point.x),
                    fmt(point.y),
                    stroke,
                    html_escape(text)
                ));
            }
        }
    }
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

    #[test]
    fn renders_kicad_bus_items_to_svg() {
        let schematic = parse_kicad_schematic(
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols)
  (bus (pts (xy 10 10) (xy 30 10)) (uuid "11111111-1111-4111-8111-111111111111"))
  (bus_entry (at 30 10) (size 2.54 -2.54) (uuid "22222222-2222-4222-8222-222222222222"))
)"#,
            "bus.kicad_sch",
        )
        .unwrap();

        let svg = render_kicad_scene_svg(&schematic.canvas_scene());

        assert!(svg.contains("<polyline"));
        assert!(svg.contains("stroke=\"#2563eb\""));
        assert!(svg.contains("data-bus-entry=\"true\""));
    }

    #[test]
    fn renders_schematic_graphics_to_svg() {
        let schematic = parse_kicad_schematic(
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols)
  (polyline (pts (xy 10 10) (xy 20 10)) (uuid "11111111-1111-4111-8111-111111111111"))
  (rectangle (start 25 10) (end 35 20) (uuid "22222222-2222-4222-8222-222222222222"))
  (circle (center 45 15) (radius 5) (uuid "33333333-3333-4333-8333-333333333333"))
)"#,
            "graphics.kicad_sch",
        )
        .unwrap();

        let svg = render_kicad_scene_svg(&schematic.canvas_scene());

        assert!(svg.contains("<polyline"));
        assert!(svg.contains("<rect"));
        assert!(svg.contains("<circle"));
        assert!(svg.contains("stroke=\"#64748b\""));
    }

    #[test]
    fn renders_schematic_text_items_to_svg() {
        let schematic = parse_kicad_schematic(
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols)
  (text ".tran 1u 1m\n.save v(out)" (at 10 10 0) (uuid "11111111-1111-4111-8111-111111111111"))
  (text "Output note" (at 20 20 0) (uuid "22222222-2222-4222-8222-222222222222"))
)"#,
            "text.kicad_sch",
        )
        .unwrap();

        let svg = render_kicad_scene_svg(&schematic.canvas_scene());

        assert!(svg.contains("data-schematic-text=\"true\""));
        assert!(svg.contains(".tran 1u 1m"));
        assert!(svg.contains(".save v(out)"));
        assert!(svg.contains("Output note"));
        assert!(svg.contains("fill=\"#b91c1c\""));
        assert!(svg.contains("fill=\"#475569\""));
    }

    #[test]
    fn renders_schematic_text_boxes_to_svg() {
        let schematic = parse_kicad_schematic(
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols)
  (text_box "Bigger\nMultiline\nText"
    (at 10 10 0)
    (size 20 10)
    (margins 1 1 1 1)
    (uuid "33333333-3333-4333-8333-333333333333")
  )
)"#,
            "text_box.kicad_sch",
        )
        .unwrap();

        let svg = render_kicad_scene_svg(&schematic.canvas_scene());

        assert!(svg.contains("data-text-box=\"true\""));
        assert!(svg.contains("<rect"));
        assert!(svg.contains("Bigger"));
        assert!(svg.contains("Multiline"));
        assert!(svg.contains("Text"));
    }

    #[test]
    fn renders_schematic_images_to_svg() {
        let schematic = parse_kicad_schematic(
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols)
  (image
    (at 36.83 39.37)
    (scale 1.5)
    (uuid "56565656-5656-4656-8656-565656565656")
    (data
      "iVBORw0KGgoAAAANSUhEUgAAADAAAAAwCAYAAABXAvmH"
    )
  )
)"#,
            "image.kicad_sch",
        )
        .unwrap();

        let svg = render_kicad_scene_svg(&schematic.canvas_scene());

        assert!(svg.contains("data-kicad-image=\"true\""));
        assert!(svg.contains("<image"));
        assert!(svg.contains("href=\"data:image/png;base64,"));
        assert!(svg.contains("iVBORw0KGgoAAAANSUhEUgAAADAAAAAwCAYAAABXAvmH"));
    }

    #[test]
    fn renders_schematic_tables_to_svg() {
        let schematic = parse_kicad_schematic(
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols)
  (table
    (column_count 2)
    (border (external yes) (header yes) (stroke (width 0) (type solid)))
    (separators (rows yes) (cols yes) (stroke (width 0) (type solid)))
    (column_widths 26.67 21.59)
    (row_heights 2.54)
    (cells
      (table_cell "LED pin"
        (at 122.555 29.21 0)
        (size 26.67 2.54)
        (margins 0.9525 0.9525 0.9525 0.9525)
        (span 1 1)
      )
      (table_cell "Expected net"
        (at 149.225 29.21 0)
        (size 21.59 2.54)
        (margins 0.9525 0.9525 0.9525 0.9525)
        (span 1 1)
      )
    )
  )
)"#,
            "table.kicad_sch",
        )
        .unwrap();

        let svg = render_kicad_scene_svg(&schematic.canvas_scene());

        assert!(svg.contains("data-kicad-table=\"true\""));
        assert!(svg.contains("data-table-cell=\"true\""));
        assert!(svg.contains("LED pin"));
        assert!(svg.contains("Expected net"));
    }
}
