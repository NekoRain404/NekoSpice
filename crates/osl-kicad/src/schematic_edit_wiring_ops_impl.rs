impl KicadSchematic {
    pub fn add_wire(
        &mut self,
        points: Vec<KicadPoint>,
        uuid: Option<String>,
    ) -> OslResult<KicadEditSummary> {
        if points.len() < 2 {
            return Err(OslError::InvalidInput(
                "KiCad wire edit requires at least two points".to_string(),
            ));
        }
        for point in &points {
            validate_point(*point, "wire point")?;
        }

        let payload = points_payload(&points);
        let uuid = Some(self.edit_uuid(uuid, "wire", &payload)?);
        self.wires.push(KicadWire {
            points,
            stroke: None,
            uuid,
        });

        Ok(KicadEditSummary {
            operation: "add-wire".to_string(),
            target: payload,
        })
    }

    pub fn add_bus(
        &mut self,
        points: Vec<KicadPoint>,
        uuid: Option<String>,
    ) -> OslResult<KicadEditSummary> {
        if points.len() < 2 {
            return Err(OslError::InvalidInput(
                "KiCad bus edit requires at least two points".to_string(),
            ));
        }
        for point in &points {
            validate_point(*point, "bus point")?;
        }

        let payload = points_payload(&points);
        let uuid = Some(self.edit_uuid(uuid, "bus", &payload)?);
        self.buses.push(KicadBus {
            points,
            stroke: None,
            uuid,
        });

        Ok(KicadEditSummary {
            operation: "add-bus".to_string(),
            target: payload,
        })
    }

    pub fn add_bus_entry(
        &mut self,
        at: KicadPoint,
        size: KicadSize,
        uuid: Option<String>,
    ) -> OslResult<KicadEditSummary> {
        validate_point(at, "bus entry")?;
        validate_bus_entry_size(size, "bus entry")?;
        if self
            .bus_entries
            .iter()
            .any(|entry| same_point(entry.at, at) && same_size(entry.size, size))
        {
            return Err(OslError::InvalidInput(format!(
                "KiCad bus entry already exists at {},{} with size {},{}",
                at.x, at.y, size.width, size.height
            )));
        }

        let payload = format!(
            "{},{}:{},{}",
            format_number(at.x),
            format_number(at.y),
            format_number(size.width),
            format_number(size.height)
        );
        let uuid = Some(self.edit_uuid(uuid, "bus-entry", &payload)?);
        self.bus_entries.push(KicadBusEntry {
            at,
            size,
            stroke: None,
            uuid,
        });

        Ok(KicadEditSummary {
            operation: "add-bus-entry".to_string(),
            target: payload,
        })
    }

    pub fn add_junction(
        &mut self,
        at: KicadPoint,
        uuid: Option<String>,
    ) -> OslResult<KicadEditSummary> {
        validate_point(at, "junction")?;
        if self.junctions.iter().any(|junction| {
            coordinate_key(junction.at.x) == coordinate_key(at.x)
                && coordinate_key(junction.at.y) == coordinate_key(at.y)
        }) {
            return Err(OslError::InvalidInput(format!(
                "KiCad junction already exists at {},{}",
                at.x, at.y
            )));
        }

        let payload = format!("{},{}", at.x, at.y);
        let uuid = Some(self.edit_uuid(uuid, "junction", &payload)?);
        self.junctions.push(KicadJunction {
            at,
            diameter: None,
            color: None,
            uuid,
        });

        Ok(KicadEditSummary {
            operation: "add-junction".to_string(),
            target: payload,
        })
    }

    pub fn add_no_connect(
        &mut self,
        at: KicadPoint,
        uuid: Option<String>,
    ) -> OslResult<KicadEditSummary> {
        validate_point(at, "no-connect")?;
        if self.no_connects.iter().any(|marker| {
            coordinate_key(marker.at.x) == coordinate_key(at.x)
                && coordinate_key(marker.at.y) == coordinate_key(at.y)
        }) {
            return Err(OslError::InvalidInput(format!(
                "KiCad no-connect marker already exists at {},{}",
                at.x, at.y
            )));
        }

        let payload = format!("{},{}", at.x, at.y);
        let uuid = Some(self.edit_uuid(uuid, "no-connect", &payload)?);
        self.no_connects.push(KicadNoConnect { at, uuid });

        Ok(KicadEditSummary {
            operation: "add-no-connect".to_string(),
            target: payload,
        })
    }

    pub fn add_label(
        &mut self,
        text: impl Into<String>,
        kind: KicadLabelKind,
        at: KicadAt,
        uuid: Option<String>,
    ) -> OslResult<KicadEditSummary> {
        validate_at(at, "label")?;
        let text = text.into();
        if text.trim().is_empty() {
            return Err(OslError::InvalidInput(
                "KiCad label text must not be empty".to_string(),
            ));
        }

        let payload = format!("{}@{},{},{}", text, at.x, at.y, at.rotation);
        let uuid = Some(self.edit_uuid(uuid, kind.sexpr_name(), &payload)?);
        self.labels.push(KicadLabel {
            text: text.clone(),
            kind,
            at: Some(at),
            uuid,
            shape: None,
            fields_autoplaced: None,
            effects: None,
            properties: Vec::new(),
        });

        Ok(KicadEditSummary {
            operation: "add-label".to_string(),
            target: text,
        })
    }

    pub fn add_text(
        &mut self,
        text: impl Into<String>,
        at: KicadAt,
        uuid: Option<String>,
    ) -> OslResult<KicadEditSummary> {
        validate_at(at, "text")?;
        let text = text.into();
        if text.trim().is_empty() {
            return Err(OslError::InvalidInput(
                "KiCad text item must not be empty".to_string(),
            ));
        }

        let payload = format!("{}@{},{},{}", text, at.x, at.y, at.rotation);
        let uuid = Some(self.edit_uuid(uuid, "text", &payload)?);
        self.text_items.push(KicadTextItem {
            text: text.clone(),
            at: Some(at),
            uuid,
            effects: None,
        });

        Ok(KicadEditSummary {
            operation: "add-text".to_string(),
            target: text,
        })
    }

    pub fn add_sheet(
        &mut self,
        name: &str,
        file: &str,
        at: KicadAt,
        size: KicadSize,
        pins: Vec<KicadSheetPin>,
        uuid: Option<String>,
    ) -> OslResult<KicadEditSummary> {
        validate_at(at, "sheet")?;
        validate_size(size, "sheet")?;
        let name = name.trim();
        let file = file.trim();
        if name.is_empty() {
            return Err(OslError::InvalidInput(
                "KiCad sheet name must not be empty".to_string(),
            ));
        }
        if file.is_empty() {
            return Err(OslError::InvalidInput(
                "KiCad sheet file must not be empty".to_string(),
            ));
        }
        if self
            .sheets
            .iter()
            .any(|sheet| sheet.sheet_name() == Some(name))
        {
            return Err(OslError::InvalidInput(format!(
                "KiCad sheet name '{name}' already exists"
            )));
        }

        let sheet_payload = format!(
            "{}:{}@{},{},{}:{}x{}",
            name, file, at.x, at.y, at.rotation, size.width, size.height
        );
        let sheet_uuid = self.edit_uuid(uuid, "sheet", &sheet_payload)?;
        let mut reserved_uuids = BTreeSet::from([sheet_uuid.clone()]);
        let mut checked_pins = Vec::new();
        for (index, pin) in pins.into_iter().enumerate() {
            let pin_name = pin.name.trim();
            if pin_name.is_empty() {
                return Err(OslError::InvalidInput(
                    "KiCad sheet pin name must not be empty".to_string(),
                ));
            }
            let pin_type = pin.pin_type.trim();
            if pin_type.is_empty() {
                return Err(OslError::InvalidInput(format!(
                    "KiCad sheet pin '{pin_name}' type must not be empty"
                )));
            }
            let at = pin.at.ok_or_else(|| {
                OslError::InvalidInput(format!("KiCad sheet pin '{pin_name}' requires a position"))
            })?;
            validate_at(at, "sheet pin")?;
            let pin_payload = format!(
                "{}:{}:{}@{},{},{}",
                sheet_uuid, pin_name, pin_type, at.x, at.y, at.rotation
            );
            let pin_uuid =
                self.edit_uuid_excluding(pin.uuid, "sheet-pin", &pin_payload, &reserved_uuids)?;
            reserved_uuids.insert(pin_uuid.clone());
            checked_pins.push(KicadSheetPin {
                name: pin_name.to_string(),
                pin_type: pin_type.to_string(),
                at: Some(at),
                uuid: Some(pin_uuid),
                effects: pin.effects.clone(),
            });
            if checked_pins[..index]
                .iter()
                .any(|existing| existing.name == pin_name)
            {
                return Err(OslError::InvalidInput(format!(
                    "KiCad sheet pin '{pin_name}' is duplicated"
                )));
            }
        }

        self.sheets.push(KicadSheet {
            at: Some(at),
            size: Some(size),
            uuid: Some(sheet_uuid),
            exclude_from_sim: None,
            in_bom: None,
            on_board: None,
            dnp: None,
            fields_autoplaced: None,
            stroke: None,
            fill: None,
            properties: sheet_properties(name, file, at, size),
            pins: checked_pins,
            instances: Vec::new(),
        });

        Ok(KicadEditSummary {
            operation: "add-sheet".to_string(),
            target: format!("{name} {file}"),
        })
    }

}
