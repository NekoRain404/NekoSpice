// SVG helper utilities for color, stroke, fill, and text rendering.
// These are used by the main render functions in svg_render_impl.rs.

fn render_image(output: &mut String, viewport: &SvgViewport, image: &NspCanvasImage) {
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
        "    <image data-schema-image=\"true\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" href=\"data:{};base64,{}\"/>\n",
        fmt(center.x - width / 2.0),
        fmt(center.y - height / 2.0),
        fmt(width),
        fmt(height),
        html_escape(&image.mime_type),
        html_escape(&image.data_base64)
    ));
}

fn render_table(output: &mut String, viewport: &SvgViewport, table: &NspCanvasTable) {
    if table.cells.is_empty() {
        return;
    }

    output.push_str(&format!(
        "    <g data-schema-table=\"true\" data-column-count=\"{}\">\n",
        table.column_count
    ));
    for cell in &table.cells {
        let Some(at) = cell.at else {
            continue;
        };
        let Some(size) = cell.size else {
            continue;
        };
        let width = size.width.abs() * viewport.scale;
        let height = size.height.abs() * viewport.scale;
        let fill = svg_fill_color(cell.fill.as_ref(), "#ffffff");
        let fill_opacity = if fill == "none" { "1" } else { "0.55" };
        output.push_str(&format!(
            "      <g data-table-cell-transform=\"true\"{}>\n",
            svg_local_transform(at, viewport)
        ));
        output.push_str(&format!(
            "        <rect data-table-cell=\"true\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" stroke=\"#64748b\" stroke-width=\"1\" fill=\"{}\" fill-opacity=\"{}\"/>\n",
            fmt(0.0),
            fmt(0.0),
            fmt(width),
            fmt(height),
            fill,
            fill_opacity
        ));
        if !cell.text.is_empty() {
            let margin = cell
                .margins
                .map(|margins| margins.left.max(0.0) * viewport.scale)
                .unwrap_or(4.0);
            let text_fill = cell
                .effects
                .as_ref()
                .and_then(|effects| effects.font_color)
                .map(svg_color)
                .unwrap_or_else(|| "#334155".to_string());
            output.push_str(&format!(
                "        <text x=\"{}\" y=\"{}\" fill=\"{}\" stroke=\"none\">{}</text>\n",
                fmt(margin),
                fmt(margin + 10.0),
                text_fill,
                html_escape(&cell.text)
            ));
        }
        output.push_str("      </g>\n");
    }
    output.push_str("    </g>\n");
}

fn render_rule_area(output: &mut String, viewport: &SvgViewport, rule_area: &NspCanvasRuleArea) {
    if rule_area.points.len() < 3 {
        return;
    }

    let points = rule_area
        .points
        .iter()
        .map(|point| {
            let point = viewport.project(*point);
            format!("{},{}", fmt(point.x), fmt(point.y))
        })
        .collect::<Vec<_>>()
        .join(" ");
    let stroke = svg_stroke_color(rule_area.stroke.as_ref(), "#0f766e");
    let stroke_width = svg_stroke_width(rule_area.stroke.as_ref(), viewport, 1.6);
    let dash_array = svg_stroke_dasharray(rule_area.stroke.as_ref());
    let fill = svg_fill_color(rule_area.fill.as_ref(), "#ccfbf1");
    let fill_opacity = if fill == "none" { "1" } else { "0.18" };
    output.push_str(&format!(
        "      <polygon data-rule-area=\"true\" points=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{} fill=\"{}\" fill-opacity=\"{}\"/>\n",
        points,
        stroke,
        fmt(stroke_width),
        dash_array,
        fill,
        fill_opacity
    ));
}

fn render_graphic(
    output: &mut String,
    viewport: &SvgViewport,
    graphic: &NspCanvasGraphic,
    stroke: &str,
    fill: &str,
) {
    match graphic {
        NspCanvasGraphic::Polyline {
            points,
            stroke: graphic_stroke,
            ..
        } => {
            render_stroked_polyline(
                output,
                viewport,
                points,
                graphic_stroke.as_ref(),
                stroke,
                1.8,
            );
        }
        NspCanvasGraphic::Bezier {
            points,
            stroke: graphic_stroke,
            ..
        } => {
            let color = svg_stroke_color(graphic_stroke.as_ref(), stroke);
            let stroke_width = svg_stroke_width(graphic_stroke.as_ref(), viewport, 1.8);
            let dash_array = svg_stroke_dasharray(graphic_stroke.as_ref());
            render_bezier(output, viewport, points, &color, stroke_width, &dash_array);
        }
        NspCanvasGraphic::Rectangle {
            start,
            end,
            stroke: graphic_stroke,
            fill: graphic_fill,
            ..
        } => {
            let left_top = viewport.project(NspPoint {
                x: start.x.min(end.x),
                y: start.y.min(end.y),
            });
            let right_bottom = viewport.project(NspPoint {
                x: start.x.max(end.x),
                y: start.y.max(end.y),
            });
            let stroke = svg_stroke_color(graphic_stroke.as_ref(), stroke);
            let stroke_width = svg_stroke_width(graphic_stroke.as_ref(), viewport, 1.8);
            let dash_array = svg_stroke_dasharray(graphic_stroke.as_ref());
            let fill = svg_fill_color(graphic_fill.as_ref(), fill);
            let fill_opacity = if fill == "none" { "1" } else { "0.25" };
            output.push_str(&format!(
                "      <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{} fill=\"{}\" fill-opacity=\"{}\"/>\n",
                fmt(left_top.x),
                fmt(left_top.y),
                fmt((right_bottom.x - left_top.x).abs()),
                fmt((right_bottom.y - left_top.y).abs()),
                stroke,
                fmt(stroke_width),
                dash_array,
                fill,
                fill_opacity
            ));
        }
        NspCanvasGraphic::Circle {
            center,
            radius,
            stroke: graphic_stroke,
            fill: graphic_fill,
            ..
        } => {
            let center = viewport.project(*center);
            let stroke = svg_stroke_color(graphic_stroke.as_ref(), stroke);
            let stroke_width = svg_stroke_width(graphic_stroke.as_ref(), viewport, 1.8);
            let dash_array = svg_stroke_dasharray(graphic_stroke.as_ref());
            let fill = svg_fill_color(graphic_fill.as_ref(), "none");
            output.push_str(&format!(
                "      <circle cx=\"{}\" cy=\"{}\" r=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{} fill=\"{}\"/>\n",
                fmt(center.x),
                fmt(center.y),
                fmt(radius * viewport.scale),
                stroke,
                fmt(stroke_width),
                dash_array,
                fill
            ));
        }
        NspCanvasGraphic::Arc {
            start,
            mid,
            end,
            stroke: graphic_stroke,
            ..
        } => {
            let points = sample_arc_points(*start, *mid, *end);
            let color = svg_stroke_color(graphic_stroke.as_ref(), stroke);
            let stroke_width = svg_stroke_width(graphic_stroke.as_ref(), viewport, 1.8);
            let dash_array = svg_stroke_dasharray(graphic_stroke.as_ref());
            render_polyline_with_dash_and_attrs(
                output,
                viewport,
                &points,
                &color,
                stroke_width,
                &dash_array,
                " data-arc=\"true\"",
            );
        }
        NspCanvasGraphic::Text {
            text,
            at,
            effects,
            stroke: graphic_stroke,
            fill: graphic_fill,
            ..
        } => {
            if effects.as_ref().is_some_and(|effects| effects.hide) {
                return;
            }
            if let Some(at) = at {
                let point = viewport.project(at_point(*at));
                let text_fill = svg_text_fill(
                    effects.as_ref(),
                    graphic_fill.as_ref(),
                    graphic_stroke.as_ref(),
                    stroke,
                );
                let attrs = svg_text_effect_attrs(effects.as_ref(), viewport);
                let transform = if at.rotation != 0.0 {
                    format!(
                        " transform=\"rotate({} {} {})\"",
                        fmt(at.rotation),
                        fmt(point.x),
                        fmt(point.y)
                    )
                } else {
                    String::new()
                };
                output.push_str(&format!(
                    "      <text data-graphic-text=\"true\" x=\"{}\" y=\"{}\" fill=\"{}\" stroke=\"none\"{}{}>{}</text>\n",
                    fmt(point.x),
                    fmt(point.y),
                    text_fill,
                    attrs,
                    transform,
                    html_escape(text)
                ));
            }
        }
    }
}

fn render_bezier(
    output: &mut String,
    viewport: &SvgViewport,
    points: &[NspPoint],
    color: &str,
    stroke_width: f64,
    dash_array: &str,
) {
    if points.len() != 4 {
        render_polyline_with_dash(output, viewport, points, color, stroke_width, dash_array);
        return;
    }

    let start = viewport.project(points[0]);
    let control_1 = viewport.project(points[1]);
    let control_2 = viewport.project(points[2]);
    let end = viewport.project(points[3]);
    output.push_str(&format!(
        "      <path data-bezier=\"true\" d=\"M {} {} C {} {}, {} {}, {} {}\" stroke=\"{}\" stroke-width=\"{}\"{} fill=\"none\"/>\n",
        fmt(start.x),
        fmt(start.y),
        fmt(control_1.x),
        fmt(control_1.y),
        fmt(control_2.x),
        fmt(control_2.y),
        fmt(end.x),
        fmt(end.y),
        color,
        fmt(stroke_width),
        dash_array
    ));
}

fn render_stroked_polyline(
    output: &mut String,
    viewport: &SvgViewport,
    points: &[NspPoint],
    stroke: Option<&NspStroke>,
    default_color: &str,
    default_width: f64,
) {
    let color = svg_stroke_color(stroke, default_color);
    let stroke_width = svg_stroke_width(stroke, viewport, default_width);
    let dash_array = svg_stroke_dasharray(stroke);
    render_polyline_with_dash(output, viewport, points, &color, stroke_width, &dash_array);
}

fn render_polyline_with_dash(
    output: &mut String,
    viewport: &SvgViewport,
    points: &[NspPoint],
    color: &str,
    stroke_width: f64,
    dash_array: &str,
) {
    render_polyline_with_dash_and_attrs(
        output,
        viewport,
        points,
        color,
        stroke_width,
        dash_array,
        "",
    );
}

fn render_polyline_with_dash_and_attrs(
    output: &mut String,
    viewport: &SvgViewport,
    points: &[NspPoint],
    color: &str,
    stroke_width: f64,
    dash_array: &str,
    attrs: &str,
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
        "      <polyline{} points=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{}/>\n",
        attrs,
        points,
        color,
        fmt(stroke_width),
        dash_array
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
    min: NspPoint,
    width_px: f64,
    height_px: f64,
    padding_px: f64,
    scale: f64,
}

impl SvgViewport {
    fn new(bounds: NspBoundingBox, options: SvgRenderOptions) -> Self {
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

    fn project(self, point: NspPoint) -> SvgPoint {
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

fn at_point(at: NspAt) -> NspPoint {
    NspPoint { x: at.x, y: at.y }
}

fn pin_body_end(at: NspAt, length: f64) -> NspPoint {
    let radians = at.rotation.to_radians();
    NspPoint {
        x: at.x + length * radians.cos(),
        y: at.y + length * radians.sin(),
    }
}

#[cfg(test)]
mod tests {
    use super::render_schema_scene_svg;
    use nsp_schema::{parse_schematic, read_schematic};
    use std::path::Path;

    #[test]
    fn renders_schema_canvas_scene_to_svg() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let schematic =
            read_schematic(&workspace_root.join("examples/schema_schematic/rc.kicad_sch"))
                .unwrap();
        let svg = render_schema_scene_svg(&schematic.canvas_scene());

        assert!(svg.starts_with("<svg "));
        assert!(svg.contains("data-reference=\"R1\""));
        assert!(svg.contains(">1k</text>"));
        assert!(svg.contains(">in</text>"));
        assert!(svg.contains("<polyline"));
        assert!(svg.ends_with("</svg>\n"));
    }

    #[test]
    fn renders_schema_hierarchical_sheet_to_svg() {
        let schematic = parse_schematic(
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (sheet
    (at 20 10)
    (size 15 10)
    (stroke (width 0.3048) (type dash) (color 139 160 255 1))
    (fill (color 247 255 168 0.3607843137))
    (property "Sheetname" "gain_stage" (at 20 9 0))
    (property "Sheetfile" "gain_stage.kicad_sch" (at 20 21 0))
    (pin "in" input (at 20 15 180))
    (pin "out" output (at 35 15 0))
  )
)"#,
            "hierarchical.kicad_sch",
        )
        .unwrap();

        let svg = render_schema_scene_svg(&schematic.canvas_scene());

        assert!(svg.contains("data-sheet-name=\"gain_stage\""));
        assert!(svg.contains("gain_stage.kicad_sch"));
        assert!(svg.contains("stroke=\"rgba(139,160,255,1)\""));
        assert!(svg.contains("stroke-width=\"5.486\""));
        assert!(svg.contains("stroke-dasharray=\"8 5\""));
        assert!(svg.contains("fill=\"rgba(247,255,168,0.361)\""));
        assert!(svg.contains(">in</text>"));
        assert!(svg.contains(">out</text>"));
    }

    #[test]
    fn renders_schema_directive_labels_to_svg() {
        let schematic = parse_schematic(
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols)
  (netclass_flag ""
    (length 3.81)
    (shape dot)
    (at 20 10 0)
    (effects (font (size 1.27 1.27) (color 236 104 255 1)) (justify left bottom))
    (uuid "3c7ec402-4c06-4b52-9acd-ed760671ff85")
    (property "Netclass" "HV" (at 20 8 0))
  )
)"#,
            "directive_label.kicad_sch",
        )
        .unwrap();

        let svg = render_schema_scene_svg(&schematic.canvas_scene());

        assert!(svg.contains("data-directive-label=\"true\""));
        assert!(svg.contains("data-shape=\"dot\""));
        assert!(svg.contains("stroke=\"rgba(236,104,255,1)\""));
        assert!(svg.contains("fill=\"rgba(236,104,255,1)\""));
        assert!(svg.contains(">HV</text>"));
    }

    #[test]
    fn renders_schema_rule_areas_to_svg() {
        let schematic = parse_schematic(
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols)
  (rule_area
    (polyline
      (pts (xy 10 10) (xy 25 10) (xy 25 20) (xy 10 20))
      (stroke (width 0.127) (type dash) (color 10 20 30 1))
      (fill (type color) (color 20 200 170 0.25))
      (uuid "c41fc141-ff73-4a8e-9714-30fcb0d8076b")
    )
  )
)"#,
            "rule_area.kicad_sch",
        )
        .unwrap();

        let svg = render_schema_scene_svg(&schematic.canvas_scene());

        assert!(svg.contains("data-rule-area=\"true\""));
        assert!(svg.contains("stroke=\"rgba(10,20,30,1)\""));
        assert!(svg.contains("stroke-width=\"2.286\""));
        assert!(svg.contains("stroke-dasharray=\"8 5\""));
        assert!(svg.contains("fill=\"rgba(20,200,170,0.25)\""));
    }

    #[test]
    fn renders_schema_bus_items_to_svg() {
        let schematic = parse_schematic(
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols)
  (bus
    (pts (xy 10 10) (xy 30 10))
    (stroke (width 0.254) (type dash) (color 58 104 255 1))
    (uuid "11111111-1111-4111-8111-111111111111")
  )
  (bus_entry
    (at 30 10)
    (size 2.54 -2.54)
    (stroke (width 0.127) (type dot) (color 255 89 101 1))
    (uuid "22222222-2222-4222-8222-222222222222")
  )
  (wire
    (pts (xy 10 15) (xy 30 15))
    (stroke (width 0.1778) (type dash_dot) (color 255 176 0 1))
    (uuid "33333333-3333-4333-8333-333333333333")
  )
)"#,
            "bus.kicad_sch",
        )
        .unwrap();

        let svg = render_schema_scene_svg(&schematic.canvas_scene());

        assert!(svg.contains("<polyline"));
        assert!(svg.contains("stroke=\"rgba(58,104,255,1)\""));
        assert!(svg.contains("stroke=\"rgba(255,176,0,1)\""));
        assert!(svg.contains("stroke-dasharray=\"8 5\""));
        assert!(svg.contains("stroke-dasharray=\"8 4 2 4\""));
        assert!(svg.contains("stroke=\"rgba(255,89,101,1)\""));
        assert!(svg.contains("data-bus-entry=\"true\""));
    }

    #[test]
    fn renders_styled_junctions_to_svg() {
        let schematic = parse_schematic(
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (junction (at 10 10) (diameter 0.8128) (color 255 0 239 1))
)"#,
            "junction.kicad_sch",
        )
        .unwrap();

        let svg = render_schema_scene_svg(&schematic.canvas_scene());

        assert!(svg.contains("<circle"));
        assert!(svg.contains("r=\"7.315\""));
        assert!(svg.contains("fill=\"rgba(255,0,239,1)\""));
    }

    #[test]
    fn renders_schematic_graphics_to_svg() {
        let schematic = parse_schematic(
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols)
  (polyline (pts (xy 10 10) (xy 20 10)) (uuid "11111111-1111-4111-8111-111111111111"))
  (bezier (pts (xy 12 20) (xy 16 12) (xy 24 12) (xy 28 20)) (uuid "44444444-4444-4444-8444-444444444444"))
  (rectangle (start 25 10) (end 35 20) (uuid "22222222-2222-4222-8222-222222222222"))
  (circle (center 45 15) (radius 5) (uuid "33333333-3333-4333-8333-333333333333"))
  (arc (start 50 20) (mid 60 10) (end 70 20) (uuid "55555555-5555-4555-8555-555555555555"))
)"#,
            "graphics.kicad_sch",
        )
        .unwrap();

        let svg = render_schema_scene_svg(&schematic.canvas_scene());

        assert!(svg.contains("<polyline"));
        assert!(svg.contains("<path data-bezier=\"true\""));
        assert!(svg.contains(" C "));
        assert!(svg.contains("<rect"));
        assert!(svg.contains("<circle"));
        assert!(svg.contains("data-arc=\"true\""));
        let arc_points = svg
            .split("data-arc=\"true\" points=\"")
            .nth(1)
            .and_then(|tail| tail.split('"').next())
            .unwrap();
        assert!(arc_points.split_whitespace().count() > 3);
        assert!(svg.contains("stroke=\"#64748b\""));
    }

    #[test]
    fn renders_styled_schematic_and_symbol_graphics_to_svg() {
        let schematic = parse_schematic(
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols
    (symbol "NekoSpice:Styled"
      (property "Reference" "U" (at 0 0 0))
      (property "Value" "Styled" (at 0 -2.54 0))
      (symbol "Styled_0_1"
        (polyline
          (pts (xy -2.54 -1.27) (xy 0 1.27) (xy 2.54 -1.27))
          (stroke (width 0.0254) (type dash_dot) (color 58 104 255 0.5))
          (fill (type outline))
        )
        (text "ALT"
          (at 1.27 2.54 90)
          (effects
            (font (size 1.524 1.016) bold italic (color 255 89 101 0.75))
            (justify right bottom)
            (href "https://nekospice.test/symbol-text")
          )
        )
      )
    )
  )
  (rectangle
    (start 25 10)
    (end 35 20)
    (stroke (width 0.127) (type dash) (color 255 89 101 1))
    (fill (type color) (color 255 176 0 0.35))
    (uuid "22222222-2222-4222-8222-222222222222")
  )
  (symbol
    (lib_id "NekoSpice:Styled")
    (at 10 10 0)
    (property "Reference" "U1" (at 10 7 0))
    (property "Value" "Styled" (at 10 13 0))
  )
)"#,
            "styled_graphics.kicad_sch",
        )
        .unwrap();

        let svg = render_schema_scene_svg(&schematic.canvas_scene());

        assert!(svg.contains("stroke=\"rgba(255,89,101,1)\""));
        assert!(svg.contains("fill=\"rgba(255,176,0,0.35)\""));
        assert!(svg.contains("stroke-dasharray=\"8 5\""));
        assert!(svg.contains("stroke=\"rgba(58,104,255,0.5)\""));
        assert!(svg.contains("stroke-dasharray=\"8 4 2 4\""));
        assert!(svg.contains("data-graphic-text=\"true\""));
        assert!(svg.contains(">ALT</text>"));
        assert!(svg.contains("fill=\"rgba(255,89,101,0.75)\""));
        assert!(svg.contains("font-size=\"18.288\""));
        assert!(svg.contains("font-weight=\"700\""));
        assert!(svg.contains("font-style=\"italic\""));
        assert!(svg.contains("text-anchor=\"end\""));
        assert!(svg.contains("dominant-baseline=\"text-after-edge\""));
        assert!(svg.contains("data-href=\"https://nekospice.test/symbol-text\""));
        assert!(svg.contains("transform=\"rotate(90"));
    }

    #[test]
    fn renders_schematic_text_items_to_svg() {
        let schematic = parse_schematic(
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

        let svg = render_schema_scene_svg(&schematic.canvas_scene());

        assert!(svg.contains("data-schematic-text=\"true\""));
        assert!(svg.contains(".tran 1u 1m"));
        assert!(svg.contains(".save v(out)"));
        assert!(svg.contains("Output note"));
        assert!(svg.contains("fill=\"#b91c1c\""));
        assert!(svg.contains("fill=\"#475569\""));
    }

    #[test]
    fn renders_schematic_text_boxes_to_svg() {
        let schematic = parse_schematic(
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

        let svg = render_schema_scene_svg(&schematic.canvas_scene());

        assert!(svg.contains("data-text-box=\"true\""));
        assert!(svg.contains("<rect"));
        assert!(svg.contains("Bigger"));
        assert!(svg.contains("Multiline"));
        assert!(svg.contains("Text"));
    }

    #[test]
    fn renders_rotated_schematic_text_boxes_to_svg() {
        let schematic = parse_schematic(
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols)
  (text_box "Rotated"
    (at 20 10 45)
    (size 10 4)
    (margins 1 1 1 1)
    (uuid "33333333-3333-4333-8333-333333333333")
  )
)"#,
            "rotated_text_box.kicad_sch",
        )
        .unwrap();

        let svg = render_schema_scene_svg(&schematic.canvas_scene());

        assert!(svg.contains("data-text-box=\"true\""));
        assert!(svg.contains("rotate(45)"));
        assert!(svg.contains("<rect x=\"0\" y=\"0\""));
        assert!(svg.contains("Rotated"));
    }

    #[test]
    fn renders_schematic_images_to_svg() {
        let schematic = parse_schematic(
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

        let svg = render_schema_scene_svg(&schematic.canvas_scene());

        assert!(svg.contains("data-schema-image=\"true\""));
        assert!(svg.contains("<image"));
        assert!(svg.contains("href=\"data:image/png;base64,"));
        assert!(svg.contains("iVBORw0KGgoAAAANSUhEUgAAADAAAAAwCAYAAABXAvmH"));
    }

    #[test]
    fn renders_schematic_tables_to_svg() {
        let schematic = parse_schematic(
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

        let svg = render_schema_scene_svg(&schematic.canvas_scene());

        assert!(svg.contains("data-schema-table=\"true\""));
        assert!(svg.contains("data-table-cell=\"true\""));
        assert!(svg.contains("LED pin"));
        assert!(svg.contains("Expected net"));
    }

    #[test]
    fn renders_rotated_schematic_table_cells_to_svg() {
        let schematic = parse_schematic(
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols)
  (table
    (column_count 1)
    (column_widths 10)
    (row_heights 4)
    (cells
      (table_cell "Rotated cell"
        (at 40 10 45)
        (size 10 4)
        (margins 1 1 1 1)
        (span 1 1)
      )
    )
  )
)"#,
            "rotated_table.kicad_sch",
        )
        .unwrap();

        let svg = render_schema_scene_svg(&schematic.canvas_scene());

        assert!(svg.contains("data-schema-table=\"true\""));
        assert!(svg.contains("data-table-cell-transform=\"true\""));
        assert!(svg.contains("rotate(45)"));
        assert!(svg.contains("<rect data-table-cell=\"true\" x=\"0\" y=\"0\""));
        assert!(svg.contains("Rotated cell"));
    }
}
