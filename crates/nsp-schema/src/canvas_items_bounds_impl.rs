#[allow(clippy::too_many_arguments)]
fn schema_canvas_item_bounds(
    symbols: &[NspCanvasSymbol],
    sheets: &[NspCanvasSheet],
    graphics: &[NspCanvasGraphic],
    images: &[NspCanvasImage],
    tables: &[NspCanvasTable],
    rule_areas: &[NspCanvasRuleArea],
    wires: &[NspCanvasWire],
    buses: &[NspCanvasBus],
    bus_entries: &[NspCanvasBusEntry],
    directive_labels: &[NspCanvasDirectiveLabel],
    labels: &[NspCanvasLabel],
    text_items: &[NspCanvasText],
    text_boxes: &[NspCanvasTextBox],
    junctions: &[NspCanvasJunction],
    no_connects: &[NspCanvasNoConnect],
) -> BTreeMap<String, NspBoundingBox> {
    let mut item_bounds = BTreeMap::new();
    for symbol in symbols {
        insert_canvas_item_bounds(&mut item_bounds, symbol.uuid.as_deref(), symbol.bounds);
    }
    for sheet in sheets {
        insert_canvas_item_bounds(&mut item_bounds, sheet.uuid.as_deref(), sheet.bounds);
    }
    for graphic in graphics {
        insert_canvas_item_bounds(
            &mut item_bounds,
            graphic.uuid().as_deref(),
            graphic.bounds(),
        );
    }
    for image in images {
        insert_canvas_item_bounds(&mut item_bounds, image.uuid.as_deref(), image.bounds);
    }
    for table in tables {
        insert_canvas_item_bounds(&mut item_bounds, table.uuid.as_deref(), table.bounds);
        for cell in &table.cells {
            insert_canvas_item_bounds(&mut item_bounds, cell.uuid.as_deref(), cell.bounds);
        }
    }
    for rule_area in rule_areas {
        insert_canvas_item_bounds(
            &mut item_bounds,
            rule_area.uuid.as_deref(),
            rule_area.bounds,
        );
    }
    for wire in wires {
        insert_canvas_item_bounds(&mut item_bounds, wire.uuid.as_deref(), wire.bounds);
    }
    for bus in buses {
        insert_canvas_item_bounds(&mut item_bounds, bus.uuid.as_deref(), bus.bounds);
    }
    for entry in bus_entries {
        insert_canvas_item_bounds(&mut item_bounds, entry.uuid.as_deref(), entry.bounds);
    }
    for label in directive_labels {
        insert_canvas_item_bounds(&mut item_bounds, label.uuid.as_deref(), label.bounds);
    }
    for label in labels {
        insert_canvas_item_bounds(&mut item_bounds, label.uuid.as_deref(), label.bounds);
    }
    for text in text_items {
        insert_canvas_item_bounds(&mut item_bounds, text.uuid.as_deref(), text.bounds);
    }
    for text_box in text_boxes {
        insert_canvas_item_bounds(&mut item_bounds, text_box.uuid.as_deref(), text_box.bounds);
    }
    for junction in junctions {
        insert_canvas_item_bounds(
            &mut item_bounds,
            junction.uuid.as_deref(),
            Some(junction.bounds),
        );
    }
    for marker in no_connects {
        insert_canvas_item_bounds(
            &mut item_bounds,
            marker.uuid.as_deref(),
            Some(marker.bounds),
        );
    }
    item_bounds
}

fn canvas_symbol_bounds(
    graphics: &[NspCanvasGraphic],
    pins: &[NspCanvasPin],
) -> Option<NspBoundingBox> {
    let mut bounds = NspBoundingBoxBuilder::default();
    for graphic in graphics {
        if let Some(graphic_bounds) = graphic.bounds() {
            bounds.include_box(graphic_bounds);
        } else {
            graphic.include_in_bounds(&mut bounds);
        }
    }
    for pin in pins {
        if let Some(pin_bounds) =
            schema_points_bounds(&[pin.start, pin.end], SCHEMA_CANVAS_LINE_BOUNDS_PADDING)
        {
            bounds.include_box(pin_bounds);
        }
    }
    bounds.finish()
}

fn insert_canvas_item_bounds(
    item_bounds: &mut BTreeMap<String, NspBoundingBox>,
    uuid: Option<&str>,
    bounds: Option<NspBoundingBox>,
) {
    if let (Some(uuid), Some(bounds)) = (uuid, bounds) {
        item_bounds.insert(uuid.to_string(), bounds);
    }
}
