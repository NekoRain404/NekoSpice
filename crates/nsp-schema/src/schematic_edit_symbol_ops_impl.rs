impl NspSchematic {
    pub fn apply_edit(&mut self, edit: NspSchematicEdit) -> OslResult<NspEditSummary> {
        match edit {
            NspSchematicEdit::MoveSymbol {
                reference,
                to,
                rotation,
            } => self.move_symbol(&reference, to, rotation),
            NspSchematicEdit::MoveItem { uuid, delta } => self.move_item_by_uuid(&uuid, delta),
            NspSchematicEdit::DeleteItem { uuid } => self.delete_item_by_uuid(&uuid),
            NspSchematicEdit::ConfigureSymbol {
                reference,
                unit,
                body_style,
                mirror,
                pin_alternates,
            } => self.configure_symbol(&reference, unit, body_style, mirror, pin_alternates),
            NspSchematicEdit::SetSymbolProperty {
                reference,
                name,
                value,
                at,
            } => self.set_symbol_property(&reference, &name, &value, at),
            NspSchematicEdit::PlaceSymbol {
                definition,
                library_symbols,
                reference,
                value,
                at,
                unit,
                body_style,
                pin_alternates,
                uuid,
            } => self.place_symbol(NspSymbolPlacement {
                definition: *definition,
                library_symbols,
                reference,
                value,
                at,
                unit,
                body_style,
                pin_alternates,
                uuid,
            }),
            NspSchematicEdit::AddWire { points, uuid } => self.add_wire(points, uuid),
            NspSchematicEdit::AddBus { points, uuid } => self.add_bus(points, uuid),
            NspSchematicEdit::AddBusEntry { at, size, uuid } => {
                self.add_bus_entry(at, size, uuid)
            }
            NspSchematicEdit::AddJunction { at, uuid } => self.add_junction(at, uuid),
            NspSchematicEdit::AddNoConnect { at, uuid } => self.add_no_connect(at, uuid),
            NspSchematicEdit::AddLabel {
                text,
                kind,
                at,
                uuid,
            } => self.add_label(text, kind, at, uuid),
            NspSchematicEdit::AddSheet {
                name,
                file,
                at,
                size,
                pins,
                uuid,
            } => self.add_sheet(&name, &file, at, size, pins, uuid),
            NspSchematicEdit::AddText { text, at, uuid } => self.add_text(text, at, uuid),
            NspSchematicEdit::SetSimulationDirective {
                kind,
                body,
                at,
                uuid,
            } => self.set_simulation_directive(NspSimulationDirectiveUpdate {
                kind,
                body,
                at,
                uuid,
            }),
            NspSchematicEdit::RotateItem { uuid, angle } => {
                self.rotate_item_by_uuid(&uuid, angle)
            }
        }
    }

    pub fn move_symbol(
        &mut self,
        reference: &str,
        to: NspPoint,
        rotation: Option<f64>,
    ) -> OslResult<NspEditSummary> {
        validate_point(to, "symbol target")?;
        let index = self.symbol_index_by_reference(reference)?;
        let symbol = &mut self.symbols[index];
        let old_at = symbol.at.unwrap_or(NspAt {
            x: 0.0,
            y: 0.0,
            rotation: 0.0,
        });
        let dx = to.x - old_at.x;
        let dy = to.y - old_at.y;
        symbol.at = Some(NspAt {
            x: to.x,
            y: to.y,
            rotation: rotation.unwrap_or(old_at.rotation),
        });

        for property in &mut symbol.properties {
            if let Some(at) = &mut property.at {
                at.x += dx;
                at.y += dy;
            }
        }

        Ok(NspEditSummary {
            operation: "move-symbol".to_string(),
            target: reference.to_string(),
        })
    }

    pub fn move_item_by_uuid(
        &mut self,
        uuid: &str,
        delta: NspPoint,
    ) -> OslResult<NspEditSummary> {
        let uuid = uuid.trim();
        if uuid.is_empty() {
            return Err(OslError::InvalidInput(
                "schema move-item UUID must not be empty".to_string(),
            ));
        }
        validate_point(delta, "item move delta")?;

        if let Some(symbol) = self
            .symbols
            .iter_mut()
            .find(|symbol| symbol.uuid.as_deref() == Some(uuid))
        {
            if let Some(at) = &mut symbol.at {
                translate_at(at, delta);
            } else {
                symbol.at = Some(NspAt {
                    x: delta.x,
                    y: delta.y,
                    rotation: 0.0,
                });
            }
            translate_properties(&mut symbol.properties, delta);
            return Ok(move_summary("symbol", uuid));
        }
        if let Some(wire) = self
            .wires
            .iter_mut()
            .find(|wire| wire.uuid.as_deref() == Some(uuid))
        {
            translate_points(&mut wire.points, delta);
            return Ok(move_summary("wire", uuid));
        }
        if let Some(bus) = self
            .buses
            .iter_mut()
            .find(|bus| bus.uuid.as_deref() == Some(uuid))
        {
            translate_points(&mut bus.points, delta);
            return Ok(move_summary("bus", uuid));
        }
        if let Some(entry) = self
            .bus_entries
            .iter_mut()
            .find(|entry| entry.uuid.as_deref() == Some(uuid))
        {
            translate_point(&mut entry.at, delta);
            return Ok(move_summary("bus-entry", uuid));
        }
        if let Some(junction) = self
            .junctions
            .iter_mut()
            .find(|junction| junction.uuid.as_deref() == Some(uuid))
        {
            translate_point(&mut junction.at, delta);
            return Ok(move_summary("junction", uuid));
        }
        if let Some(marker) = self
            .no_connects
            .iter_mut()
            .find(|marker| marker.uuid.as_deref() == Some(uuid))
        {
            translate_point(&mut marker.at, delta);
            return Ok(move_summary("no-connect", uuid));
        }
        if let Some(label) = self
            .labels
            .iter_mut()
            .find(|label| label.uuid.as_deref() == Some(uuid))
        {
            translate_optional_at(&mut label.at, delta);
            translate_properties(&mut label.properties, delta);
            return Ok(move_summary("label", uuid));
        }
        if let Some(label) = self
            .directive_labels
            .iter_mut()
            .find(|label| label.uuid.as_deref() == Some(uuid))
        {
            translate_optional_at(&mut label.at, delta);
            translate_properties(&mut label.properties, delta);
            return Ok(move_summary("directive-label", uuid));
        }
        if let Some(text) = self
            .text_items
            .iter_mut()
            .find(|text| text.uuid.as_deref() == Some(uuid))
        {
            translate_optional_at(&mut text.at, delta);
            return Ok(move_summary("text", uuid));
        }
        if let Some(text_box) = self
            .text_boxes
            .iter_mut()
            .find(|text_box| text_box.uuid.as_deref() == Some(uuid))
        {
            translate_optional_at(&mut text_box.at, delta);
            return Ok(move_summary("text-box", uuid));
        }
        if let Some(sheet) = self
            .sheets
            .iter_mut()
            .find(|sheet| sheet.uuid.as_deref() == Some(uuid))
        {
            translate_optional_at(&mut sheet.at, delta);
            translate_properties(&mut sheet.properties, delta);
            for pin in &mut sheet.pins {
                translate_optional_at(&mut pin.at, delta);
            }
            return Ok(move_summary("sheet", uuid));
        }
        if move_sheet_pin_by_uuid(&mut self.sheets, uuid, delta) {
            return Ok(move_summary("sheet-pin", uuid));
        }
        if let Some(graphic) = self
            .graphics
            .iter_mut()
            .find(|graphic| graphic.uuid.as_deref() == Some(uuid))
        {
            translate_graphic(&mut graphic.graphic, delta);
            return Ok(move_summary("graphic", uuid));
        }
        if let Some(rule_area) = self
            .rule_areas
            .iter_mut()
            .find(|rule_area| rule_area.uuid.as_deref() == Some(uuid))
        {
            translate_points(&mut rule_area.points, delta);
            return Ok(move_summary("rule-area", uuid));
        }
        if let Some(image) = self
            .images
            .iter_mut()
            .find(|image| image.uuid.as_deref() == Some(uuid))
        {
            translate_optional_point(&mut image.at, delta);
            return Ok(move_summary("image", uuid));
        }
        if let Some(table) = self
            .tables
            .iter_mut()
            .find(|table| table.uuid.as_deref() == Some(uuid))
        {
            for cell in &mut table.cells {
                translate_optional_at(&mut cell.at, delta);
            }
            return Ok(move_summary("table", uuid));
        }
        if move_table_cell_by_uuid(&mut self.tables, uuid, delta) {
            return Ok(move_summary("table-cell", uuid));
        }
        if self
            .groups
            .iter()
            .any(|group| group.uuid.as_deref() == Some(uuid))
        {
            return Err(OslError::InvalidInput(format!(
                "schema schematic group UUID '{uuid}' has no geometry; move its member items instead"
            )));
        }

        Err(OslError::InvalidInput(format!(
            "schema schematic item UUID '{uuid}' was not found"
        )))
    }

    /// Rotate an item identified by UUID by the given angle in degrees.
    ///
    /// For symbols this adds to their existing rotation; for other items it
    /// only rotates those that carry an explicit rotation field (labels, text).
    pub fn rotate_item_by_uuid(
        &mut self,
        uuid: &str,
        angle: f64,
    ) -> OslResult<NspEditSummary> {
        let uuid = uuid.trim();
        if uuid.is_empty() {
            return Err(OslError::InvalidInput(
                "schema rotate-item UUID must not be empty".to_string(),
            ));
        }

        // Symbols: add to the rotation in their `at` field
        if let Some(symbol) = self
            .symbols
            .iter_mut()
            .find(|symbol| symbol.uuid.as_deref() == Some(uuid))
        {
            if let Some(at) = &mut symbol.at {
                at.rotation = (at.rotation + angle) % 360.0;
            }
            return Ok(NspEditSummary {
                operation: "rotate-symbol".to_string(),
                target: uuid.to_string(),
            });
        }

        // Labels: rotate their `at` field
        if let Some(label) = self
            .labels
            .iter_mut()
            .find(|label| label.uuid.as_deref() == Some(uuid))
        {
            if let Some(at) = &mut label.at {
                at.rotation = (at.rotation + angle) % 360.0;
            }
            return Ok(NspEditSummary {
                operation: "rotate-label".to_string(),
                target: uuid.to_string(),
            });
        }

        // Text items: rotate their `at` field
        if let Some(text) = self
            .text_items
            .iter_mut()
            .find(|text| text.uuid.as_deref() == Some(uuid))
        {
            if let Some(at) = &mut text.at {
                at.rotation = (at.rotation + angle) % 360.0;
            }
            return Ok(NspEditSummary {
                operation: "rotate-text".to_string(),
                target: uuid.to_string(),
            });
        }

        // Sheets: rotate their `at` field
        if let Some(sheet) = self
            .sheets
            .iter_mut()
            .find(|sheet| sheet.uuid.as_deref() == Some(uuid))
        {
            if let Some(at) = &mut sheet.at {
                at.rotation = (at.rotation + angle) % 360.0;
            }
            return Ok(NspEditSummary {
                operation: "rotate-sheet".to_string(),
                target: uuid.to_string(),
            });
        }

        Err(OslError::InvalidInput(format!(
            "schema schematic item UUID '{uuid}' was not found or does not support rotation"
        )))
    }

    pub fn delete_item_by_uuid(&mut self, uuid: &str) -> OslResult<NspEditSummary> {
        let uuid = uuid.trim();
        if uuid.is_empty() {
            return Err(OslError::InvalidInput(
                "schema delete-item UUID must not be empty".to_string(),
            ));
        }

        if remove_by_uuid(&mut self.symbols, uuid, |symbol| symbol.uuid.as_deref()) {
            return Ok(delete_summary("symbol", uuid));
        }
        if remove_by_uuid(&mut self.wires, uuid, |wire| wire.uuid.as_deref()) {
            return Ok(delete_summary("wire", uuid));
        }
        if remove_by_uuid(&mut self.buses, uuid, |bus| bus.uuid.as_deref()) {
            return Ok(delete_summary("bus", uuid));
        }
        if remove_by_uuid(&mut self.bus_entries, uuid, |entry| entry.uuid.as_deref()) {
            return Ok(delete_summary("bus-entry", uuid));
        }
        if remove_by_uuid(&mut self.junctions, uuid, |junction| {
            junction.uuid.as_deref()
        }) {
            return Ok(delete_summary("junction", uuid));
        }
        if remove_by_uuid(&mut self.no_connects, uuid, |marker| marker.uuid.as_deref()) {
            return Ok(delete_summary("no-connect", uuid));
        }
        if remove_by_uuid(&mut self.labels, uuid, |label| label.uuid.as_deref()) {
            return Ok(delete_summary("label", uuid));
        }
        if remove_by_uuid(&mut self.directive_labels, uuid, |label| {
            label.uuid.as_deref()
        }) {
            return Ok(delete_summary("directive-label", uuid));
        }
        if remove_by_uuid(&mut self.text_items, uuid, |text| text.uuid.as_deref()) {
            return Ok(delete_summary("text", uuid));
        }
        if remove_by_uuid(&mut self.text_boxes, uuid, |text_box| {
            text_box.uuid.as_deref()
        }) {
            return Ok(delete_summary("text-box", uuid));
        }
        if remove_by_uuid(&mut self.sheets, uuid, |sheet| sheet.uuid.as_deref()) {
            return Ok(delete_summary("sheet", uuid));
        }
        if remove_sheet_pin_by_uuid(&mut self.sheets, uuid) {
            return Ok(delete_summary("sheet-pin", uuid));
        }
        if remove_by_uuid(&mut self.graphics, uuid, |graphic| graphic.uuid.as_deref()) {
            return Ok(delete_summary("graphic", uuid));
        }
        if remove_by_uuid(&mut self.rule_areas, uuid, |rule_area| {
            rule_area.uuid.as_deref()
        }) {
            return Ok(delete_summary("rule-area", uuid));
        }
        if remove_by_uuid(&mut self.images, uuid, |image| image.uuid.as_deref()) {
            return Ok(delete_summary("image", uuid));
        }
        if remove_by_uuid(&mut self.tables, uuid, |table| table.uuid.as_deref()) {
            return Ok(delete_summary("table", uuid));
        }
        if remove_table_cell_by_uuid(&mut self.tables, uuid) {
            return Ok(delete_summary("table-cell", uuid));
        }
        if remove_by_uuid(&mut self.groups, uuid, |group| group.uuid.as_deref()) {
            return Ok(delete_summary("group", uuid));
        }

        Err(OslError::InvalidInput(format!(
            "schema schematic item UUID '{uuid}' was not found"
        )))
    }

    pub fn configure_symbol(
        &mut self,
        reference: &str,
        unit: Option<u32>,
        body_style: Option<Option<u32>>,
        mirror: Option<Option<String>>,
        pin_alternates: Option<BTreeMap<String, String>>,
    ) -> OslResult<NspEditSummary> {
        if unit == Some(0) {
            return Err(OslError::InvalidInput(
                "schema symbol unit must be positive".to_string(),
            ));
        }
        if body_style == Some(Some(0)) {
            return Err(OslError::InvalidInput(
                "schema symbol body style must be positive".to_string(),
            ));
        }
        let normalized_mirror = match mirror {
            Some(Some(mirror)) => Some(normalize_symbol_mirror(&mirror)?),
            Some(None) => Some(None),
            None => None,
        };

        let index = self.symbol_index_by_reference(reference)?;
        let current_symbol = self.symbols[index].clone();
        let definition = self
            .resolved_symbol_definition(&current_symbol.lib_id)
            .ok_or_else(|| {
                OslError::InvalidInput(format!(
                    "schema symbol reference '{reference}' uses missing library symbol '{}'",
                    current_symbol.lib_id
                ))
            })?;
        let selected_unit = unit.or(current_symbol.unit).unwrap_or(1);
        let selected_body_style = body_style.unwrap_or(current_symbol.body_style);
        let selected_alternates = pin_alternates.unwrap_or_else(|| {
            current_symbol
                .pins
                .iter()
                .filter_map(|pin| Some((pin.number.clone()?, pin.alternate.clone()?)))
                .collect()
        });
        let pins = self.configured_symbol_pin_refs(
            &current_symbol,
            &definition,
            selected_unit,
            selected_body_style,
            &selected_alternates,
        )?;

        let symbol = &mut self.symbols[index];
        if unit.is_some() {
            symbol.unit = Some(selected_unit);
        }
        if body_style.is_some() {
            symbol.body_style = selected_body_style;
        }
        if let Some(mirror) = normalized_mirror {
            symbol.mirror = mirror;
        }
        symbol.pins = pins;

        Ok(NspEditSummary {
            operation: "configure-symbol".to_string(),
            target: reference.to_string(),
        })
    }

    pub fn set_symbol_property(
        &mut self,
        reference: &str,
        name: &str,
        value: &str,
        at: Option<NspAt>,
    ) -> OslResult<NspEditSummary> {
        if name.trim().is_empty() {
            return Err(OslError::InvalidInput(
                "schema symbol property name must not be empty".to_string(),
            ));
        }
        if let Some(at) = at {
            validate_at(at, "symbol property")?;
        }

        let index = self.symbol_index_by_reference(reference)?;
        let symbol = &mut self.symbols[index];
        if let Some(property) = symbol
            .properties
            .iter_mut()
            .find(|property| property.name == name)
        {
            property.value = value.to_string();
            if let Some(at) = at {
                property.at = Some(at);
            }
        } else {
            symbol.properties.push(NspProperty {
                name: name.to_string(),
                value: value.to_string(),
                id: None,
                at,
                hide: None,
                show_name: None,
                do_not_autoplace: None,
                effects: None,
            });
        }

        Ok(NspEditSummary {
            operation: "set-property".to_string(),
            target: format!("{reference}.{name}"),
        })
    }

    pub fn place_symbol(&mut self, placement: NspSymbolPlacement) -> OslResult<NspEditSummary> {
        let NspSymbolPlacement {
            definition,
            library_symbols,
            reference,
            value,
            at,
            unit,
            body_style,
            pin_alternates,
            uuid,
        } = placement;
        validate_at(at, "symbol placement")?;
        if unit == Some(0) {
            return Err(OslError::InvalidInput(
                "schema symbol placement unit must be positive".to_string(),
            ));
        }
        if body_style == Some(0) {
            return Err(OslError::InvalidInput(
                "schema symbol placement body style must be positive".to_string(),
            ));
        }
        if reference.trim().is_empty() {
            return Err(OslError::InvalidInput(
                "schema placed symbol reference must not be empty".to_string(),
            ));
        }
        if self
            .symbols
            .iter()
            .any(|symbol| symbol.reference() == Some(reference.as_str()))
        {
            return Err(OslError::InvalidInput(format!(
                "schema symbol reference '{reference}' already exists"
            )));
        }

        let lib_id = definition.name.clone();
        self.merge_symbol_placement_library_symbol(&definition)?;
        for dependency in library_symbols {
            if dependency.name == lib_id {
                continue;
            }
            self.merge_symbol_placement_library_symbol(&dependency)?;
        }

        let resolved_definition = resolve_symbol_definition(&definition, &self.library_symbols)
            .unwrap_or_else(|| NspResolvedSymbolDef::from_symbol(&definition));
        let instance_payload = format!(
            "{}:{}:{}@{},{},{}",
            lib_id, reference, value, at.x, at.y, at.rotation
        );
        let instance_uuid = self.edit_uuid(uuid, "symbol", &instance_payload)?;
        let properties = symbol_instance_properties(&definition, &reference, &value, at);
        let unit = unit.unwrap_or(1);
        let mut sorted_pins = resolved_definition
            .scoped_pins(Some(unit), body_style)
            .collect::<Vec<_>>();
        sorted_pins.sort_by(compare_pin_numbers);
        for pin_number in pin_alternates.keys() {
            let Some(pin) = sorted_pins
                .iter()
                .find(|pin| pin.number() == pin_number.as_str())
            else {
                return Err(OslError::InvalidInput(format!(
                    "schema symbol placement pin '{pin_number}' is not present in selected unit/body style"
                )));
            };
            let alternate = pin_alternates
                .get(pin_number)
                .expect("pin alternate was just looked up");
            if !pin
                .alternates
                .iter()
                .any(|candidate| candidate.name == *alternate)
            {
                return Err(OslError::InvalidInput(format!(
                    "schema symbol placement pin '{pin_number}' has no alternate '{alternate}'"
                )));
            }
        }
        let mut generated_pin_uuids = BTreeSet::new();
        let mut pins = Vec::new();
        for (index, pin) in sorted_pins.into_iter().enumerate() {
            let pin_number = pin.number().to_string();
            let pin_uuid = self.edit_uuid_excluding(
                None,
                "symbol-pin",
                &format!("{instance_uuid}:{pin_number}:{index}"),
                &generated_pin_uuids,
            )?;
            generated_pin_uuids.insert(pin_uuid.clone());
            pins.push(NspSymbolPinRef {
                number: Some(pin_number.clone()),
                uuid: Some(pin_uuid),
                alternate: pin_alternates.get(&pin_number).cloned(),
            });
        }

        self.symbols.push(NspSymbolInstance {
            lib_id: lib_id.clone(),
            at: Some(at),
            mirror: None,
            unit: Some(unit),
            body_style,
            uuid: Some(instance_uuid),
            exclude_from_sim: None,
            in_bom: None,
            on_board: None,
            dnp: None,
            fields_autoplaced: None,
            properties,
            pins,
            instances: Vec::new(),
        });

        Ok(NspEditSummary {
            operation: "place-symbol".to_string(),
            target: format!("{reference} {lib_id}"),
        })
    }

}
