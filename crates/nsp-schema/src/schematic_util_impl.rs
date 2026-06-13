// Utility and internal helper methods.
// Covers: symbol_index_by_reference, edit_uuid, used_uuids,
// symbol_pin_points, sheet_pin_points, has_no_connect_at.

impl NspSchematic {
    fn symbol_index_by_reference(&self, reference: &str) -> OslResult<usize> {
        if reference.trim().is_empty() {
            return Err(OslError::InvalidInput(
                "schema symbol reference must not be empty".to_string(),
            ));
        }
        self.symbols
            .iter()
            .position(|symbol| symbol.reference() == Some(reference))
            .ok_or_else(|| {
                OslError::InvalidInput(format!(
                    "schema symbol reference '{reference}' was not found"
                ))
            })
    }

    pub(crate) fn edit_uuid(
        &self,
        uuid: Option<String>,
        namespace: &str,
        payload: &str,
    ) -> OslResult<String> {
        self.edit_uuid_excluding(uuid, namespace, payload, &BTreeSet::new())
    }

    fn edit_uuid_excluding(
        &self,
        uuid: Option<String>,
        namespace: &str,
        payload: &str,
        reserved: &BTreeSet<String>,
    ) -> OslResult<String> {
        let used = self.used_uuids();
        if let Some(uuid) = uuid.filter(|uuid| !uuid.trim().is_empty()) {
            if used.contains(&uuid) || reserved.contains(&uuid) {
                return Err(OslError::InvalidInput(format!(
                    "schema UUID '{uuid}' is already used in this schematic"
                )));
            }
            return Ok(uuid);
        }

        for counter in 0.. {
            let seed = format!(
                "{}:{namespace}:{payload}:{}:{}:{}:{counter}",
                self.source,
                self.symbols.len(),
                self.wires.len(),
                self.labels.len()
            );
            let candidate = uuid_from_hashes(fnv1a64(&seed), fnv1a64(&format!("{seed}:b")));
            if !used.contains(&candidate) && !reserved.contains(&candidate) {
                return Ok(candidate);
            }
        }
        unreachable!("unbounded UUID search should always find a free candidate")
    }

    pub(crate) fn used_uuids(&self) -> BTreeSet<String> {
        let mut uuids = BTreeSet::new();
        if let Some(uuid) = &self.uuid {
            uuids.insert(uuid.clone());
        }
        for wire in &self.wires {
            if let Some(uuid) = &wire.uuid {
                uuids.insert(uuid.clone());
            }
        }
        for bus in &self.buses {
            if let Some(uuid) = &bus.uuid {
                uuids.insert(uuid.clone());
            }
        }
        for entry in &self.bus_entries {
            if let Some(uuid) = &entry.uuid {
                uuids.insert(uuid.clone());
            }
        }
        for graphic in &self.graphics {
            if let Some(uuid) = &graphic.uuid {
                uuids.insert(uuid.clone());
            }
        }
        for image in &self.images {
            if let Some(uuid) = &image.uuid {
                uuids.insert(uuid.clone());
            }
        }
        for table in &self.tables {
            if let Some(uuid) = &table.uuid {
                uuids.insert(uuid.clone());
            }
            for cell in &table.cells {
                if let Some(uuid) = &cell.uuid {
                    uuids.insert(uuid.clone());
                }
            }
        }
        for rule_area in &self.rule_areas {
            if let Some(uuid) = &rule_area.uuid {
                uuids.insert(uuid.clone());
            }
        }
        for group in &self.groups {
            if let Some(uuid) = &group.uuid {
                uuids.insert(uuid.clone());
            }
        }
        for label in &self.labels {
            if let Some(uuid) = &label.uuid {
                uuids.insert(uuid.clone());
            }
        }
        for label in &self.directive_labels {
            if let Some(uuid) = &label.uuid {
                uuids.insert(uuid.clone());
            }
        }
        for junction in &self.junctions {
            if let Some(uuid) = &junction.uuid {
                uuids.insert(uuid.clone());
            }
        }
        for marker in &self.no_connects {
            if let Some(uuid) = &marker.uuid {
                uuids.insert(uuid.clone());
            }
        }
        for sheet in &self.sheets {
            if let Some(uuid) = &sheet.uuid {
                uuids.insert(uuid.clone());
            }
            for pin in &sheet.pins {
                if let Some(uuid) = &pin.uuid {
                    uuids.insert(uuid.clone());
                }
            }
        }
        for text in &self.text_items {
            if let Some(uuid) = &text.uuid {
                uuids.insert(uuid.clone());
            }
        }
        for text_box in &self.text_boxes {
            if let Some(uuid) = &text_box.uuid {
                uuids.insert(uuid.clone());
            }
        }
        for symbol in &self.symbols {
            if let Some(uuid) = &symbol.uuid {
                uuids.insert(uuid.clone());
            }
            for pin in &symbol.pins {
                if let Some(uuid) = &pin.uuid {
                    uuids.insert(uuid.clone());
                }
            }
        }
        uuids
    }

    pub(crate) fn symbol_pin_points(&self) -> Vec<NspPoint> {
        self.symbols
            .iter()
            .flat_map(|symbol| {
                let Some(symbol_at) = symbol.at else {
                    return Vec::new();
                };
                self.resolved_symbol_definition_with_fallback(&symbol.lib_id, symbol.lib_name.as_deref())
                    .map(|definition| {
                        definition
                            .scoped_pins(symbol.unit, symbol.body_style)
                            .filter_map(|pin| pin.at)
                            .map(|pin_at| {
                                transform_symbol_point(pin_at, symbol_at, symbol.mirror.as_deref())
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default()
            })
            .collect()
    }

    pub(crate) fn sheet_pin_points(&self) -> Vec<NspPoint> {
        self.sheets
            .iter()
            .flat_map(|sheet| {
                sheet
                    .pins
                    .iter()
                    .filter_map(|pin| pin.at.map(|at| at.point()))
            })
            .collect()
    }

    fn has_no_connect_at(&self, point: NspPoint) -> bool {
        self.no_connects
            .iter()
            .any(|marker| same_point(marker.at, point))
    }
}
