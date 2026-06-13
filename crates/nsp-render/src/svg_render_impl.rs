// SVG rendering implementation — main render functions and element renderers.
// SVG helper utilities are in svg_helpers_impl.rs.

// SVG rendering implementation (extracted from lib.rs).
// Covers: render_schema_scene_svg_with_options and all SVG helpers.

pub fn render_schema_scene_svg_with_options(
    scene: &NspCanvasScene,
    options: SvgRenderOptions,
) -> String {
    let bounds = scene.bounds.unwrap_or(NspBoundingBox {
        min: NspPoint { x: 0.0, y: 0.0 },
        max: NspPoint { x: 20.0, y: 20.0 },
    });
    let viewport = SvgViewport::new(bounds, options);
    let mut output = String::new();

    output.push_str(&format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{:.0}" height="{:.0}" viewBox="0 0 {:.3} {:.3}" role="img" aria-label="{}">"#,
        viewport.width_px,
        viewport.height_px,
        viewport.width_px,
        viewport.height_px,
        html_escape(&format!("schema schematic {}", scene.source))
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
    for rule_area in &scene.rule_areas {
        render_rule_area(&mut output, &viewport, rule_area);
    }
    for bus in &scene.buses {
        render_stroked_polyline(
            &mut output,
            &viewport,
            &bus.points,
            bus.stroke.as_ref(),
            "#2563eb",
            3.2,
        );
    }
    for wire in &scene.wires {
        render_stroked_polyline(
            &mut output,
            &viewport,
            &wire.points,
            wire.stroke.as_ref(),
            "#0f172a",
            2.0,
        );
    }
    for entry in &scene.bus_entries {
        let start = viewport.project(entry.at);
        let end = viewport.project(entry.end());
        let stroke = svg_stroke_color(entry.stroke.as_ref(), "#2563eb");
        let stroke_width = svg_stroke_width(entry.stroke.as_ref(), &viewport, 2.0);
        let dash_array = svg_stroke_dasharray(entry.stroke.as_ref());
        output.push_str(&format!(
            "    <line data-bus-entry=\"true\" x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{}/>\n",
            fmt(start.x),
            fmt(start.y),
            fmt(end.x),
            fmt(end.y),
            stroke,
            fmt(stroke_width),
            dash_array
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
        let radius = junction
            .diameter
            .filter(|diameter| diameter.is_finite() && *diameter > 0.0)
            .map(|diameter| diameter * viewport.scale / 2.0)
            .unwrap_or(3.0);
        let fill = junction
            .color
            .map(svg_color)
            .unwrap_or_else(|| "#0f172a".to_string());
        output.push_str(&format!(
            "    <circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"{}\" stroke=\"none\"/>\n",
            fmt(point.x),
            fmt(point.y),
            fmt(radius),
            fill
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
                NspLabelKind::Local => "#0369a1",
                NspLabelKind::Global => "#7c3aed",
                NspLabelKind::Hierarchical => "#b45309",
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
    for label in &scene.directive_labels {
        render_directive_label(&mut output, &viewport, label);
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

fn render_sheet(output: &mut String, viewport: &SvgViewport, sheet: &NspCanvasSheet) {
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
    let stroke = svg_stroke_color(sheet.stroke.as_ref(), "#b45309");
    let stroke_width = svg_stroke_width(sheet.stroke.as_ref(), viewport, 1.8);
    let dash_array = svg_stroke_dasharray(sheet.stroke.as_ref());
    let fill = svg_fill_color(sheet.fill.as_ref(), "#fef3c7");
    let fill_opacity = if fill == "none" { "1" } else { "0.18" };
    output.push_str(&format!(
        "      <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{} fill=\"{}\" fill-opacity=\"{}\"/>\n",
        fmt(origin.x),
        fmt(origin.y),
        fmt(size.width * viewport.scale),
        fmt(size.height * viewport.scale),
        stroke,
        fmt(stroke_width),
        dash_array,
        fill,
        fill_opacity
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

fn render_directive_label(
    output: &mut String,
    viewport: &SvgViewport,
    label: &NspCanvasDirectiveLabel,
) {
    let Some(at) = label.at else {
        return;
    };

    let start = viewport.project(at_point(at));
    let end = viewport.project(pin_body_end(at, label.length.unwrap_or(2.54)));
    let text = if label.text.is_empty() {
        label
            .properties
            .iter()
            .find(|property| {
                matches!(
                    property.name.as_str(),
                    "Netclass" | "Net Class" | "Component Class"
                ) && !property.value.is_empty()
            })
            .map(|property| property.value.as_str())
            .unwrap_or("")
    } else {
        label.text.as_str()
    };
    let fill = label
        .effects
        .as_ref()
        .and_then(|effects| effects.font_color)
        .map(svg_color)
        .unwrap_or_else(|| "#0f766e".to_string());
    let shape = label.shape.as_deref().unwrap_or("round");

    output.push_str(&format!(
        "    <g data-directive-label=\"true\" data-shape=\"{}\">\n",
        html_escape(shape)
    ));
    output.push_str(&format!(
        "      <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.6\"/>\n",
        fmt(start.x),
        fmt(start.y),
        fmt(end.x),
        fmt(end.y),
        fill
    ));
    output.push_str(&format!(
        "      <circle cx=\"{}\" cy=\"{}\" r=\"3.2\" fill=\"{}\" stroke=\"none\"/>\n",
        fmt(start.x),
        fmt(start.y),
        fill
    ));
    if !text.is_empty() {
        output.push_str(&format!(
            "      <text x=\"{}\" y=\"{}\" fill=\"{}\" stroke=\"none\">{}</text>\n",
            fmt(start.x + 4.0),
            fmt(start.y - 4.0),
            fill,
            html_escape(text)
        ));
    }
    output.push_str("    </g>\n");
}

fn render_symbol(output: &mut String, viewport: &SvgViewport, symbol: &NspCanvasSymbol) {
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

fn render_text_box(output: &mut String, viewport: &SvgViewport, text_box: &NspCanvasTextBox) {
    let Some(at) = text_box.at else {
        return;
    };
    let margin = text_box
        .margins
        .map(|margins| margins.left.max(0.0) * viewport.scale)
        .unwrap_or(6.0);

    output.push_str(&format!(
        "    <g data-text-box=\"true\"{}>\n",
        svg_local_transform(at, viewport)
    ));
    if let Some(size) = text_box.size {
        let stroke = svg_stroke_color(text_box.stroke.as_ref(), "#64748b");
        let stroke_width = svg_stroke_width(text_box.stroke.as_ref(), viewport, 1.4);
        let dash_array = svg_stroke_dasharray(text_box.stroke.as_ref());
        let fill = svg_fill_color(text_box.fill.as_ref(), "#fef3c7");
        let fill_opacity = if fill == "none" { "1" } else { "0.22" };
        output.push_str(&format!(
            "      <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"2\" stroke=\"{}\" stroke-width=\"{}\"{} fill=\"{}\" fill-opacity=\"{}\"/>\n",
            fmt(0.0),
            fmt(0.0),
            fmt(size.width.abs() * viewport.scale),
            fmt(size.height.abs() * viewport.scale),
            stroke,
            fmt(stroke_width),
            dash_array,
            fill,
            fill_opacity
        ));
    }

    let text_x = margin;
    let mut text_y = margin + 11.0;
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

fn svg_text_fill(
    effects: Option<&NspTextEffects>,
    fill: Option<&NspFill>,
    stroke: Option<&NspStroke>,
    default: &str,
) -> String {
    if let Some(color) = effects.and_then(|effects| effects.font_color) {
        return svg_color(color);
    }
    let text_fill = svg_fill_color(fill, default);
    if text_fill == "none" {
        svg_stroke_color(stroke, default)
    } else {
        text_fill
    }
}

fn svg_text_effect_attrs(effects: Option<&NspTextEffects>, viewport: &SvgViewport) -> String {
    let Some(effects) = effects else {
        return String::new();
    };

    let mut attrs = String::new();
    if let Some(size) = effects.font_size
        && size.height.is_finite()
        && size.height > 0.0
    {
        attrs.push_str(&format!(
            " font-size=\"{}\"",
            fmt((size.height * viewport.scale).max(1.0))
        ));
    }
    if effects.font_bold == Some(true) {
        attrs.push_str(" font-weight=\"700\"");
    }
    if effects.font_italic == Some(true) {
        attrs.push_str(" font-style=\"italic\"");
    }
    if let Some(anchor) = svg_text_anchor(effects) {
        attrs.push_str(&format!(" text-anchor=\"{}\"", anchor));
    }
    if let Some(baseline) = svg_text_baseline(effects) {
        attrs.push_str(&format!(" dominant-baseline=\"{}\"", baseline));
    }
    if let Some(href) = &effects.href {
        attrs.push_str(&format!(" data-href=\"{}\"", html_escape(href)));
    }
    attrs
}

fn svg_text_anchor(effects: &NspTextEffects) -> Option<&'static str> {
    if effects.justify.iter().any(|token| token == "right") {
        Some("end")
    } else if effects.justify.iter().any(|token| token == "center") {
        Some("middle")
    } else if effects.justify.iter().any(|token| token == "left") {
        Some("start")
    } else {
        None
    }
}

fn svg_text_baseline(effects: &NspTextEffects) -> Option<&'static str> {
    if effects.justify.iter().any(|token| token == "top") {
        Some("hanging")
    } else if effects.justify.iter().any(|token| token == "bottom") {
        Some("text-after-edge")
    } else {
        None
    }
}

fn svg_local_transform(at: NspAt, viewport: &SvgViewport) -> String {
    let origin = viewport.project(at_point(at));
    if at.rotation == 0.0 {
        format!(
            " transform=\"translate({} {})\"",
            fmt(origin.x),
            fmt(origin.y)
        )
    } else {
        format!(
            " transform=\"translate({} {}) rotate({})\"",
            fmt(origin.x),
            fmt(origin.y),
            fmt(at.rotation)
        )
    }
}

fn svg_color(color: NspColor) -> String {
    format!(
        "rgba({},{},{},{})",
        fmt(color.red.clamp(0.0, 255.0)),
        fmt(color.green.clamp(0.0, 255.0)),
        fmt(color.blue.clamp(0.0, 255.0)),
        fmt(color.alpha.clamp(0.0, 1.0))
    )
}

fn svg_stroke_color(stroke: Option<&NspStroke>, default: &str) -> String {
    stroke
        .and_then(|stroke| stroke.color)
        .map(svg_color)
        .unwrap_or_else(|| default.to_string())
}

fn svg_stroke_width(stroke: Option<&NspStroke>, viewport: &SvgViewport, default: f64) -> f64 {
    stroke
        .and_then(|stroke| stroke.width)
        .filter(|width| width.is_finite() && *width > 0.0)
        .map(|width| (width * viewport.scale).max(1.0))
        .unwrap_or(default)
}

fn svg_stroke_dasharray(stroke: Option<&NspStroke>) -> String {
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

fn svg_fill_color(fill: Option<&NspFill>, default: &str) -> String {
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


include!("svg_helpers_impl.rs");
