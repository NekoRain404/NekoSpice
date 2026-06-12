//! Document edit operations — all write operations that modify the
//! schematic: delete, move, rotate, place symbols, wire, bus, label, etc.

use crate::document::{KicadGuiDocument, KicadSymbolPlacementResult};
use osl_kicad::{
    KicadAt, KicadEditSummary, KicadLabelKind, KicadPoint, KicadSchematicEdit, KicadSheetPin,
    KicadSimulationDirectiveKind, KicadSize, KicadSymbolDef,
};
use crate::placement_config::SymbolPlacementConfig;

impl KicadGuiDocument {
    pub(crate) fn delete_item(&mut self, uuid: &str) -> Result<KicadEditSummary, String> {
        self.apply_edit(KicadSchematicEdit::DeleteItem {
            uuid: uuid.to_string(),
        })
    }

    /// move item。
    pub(crate) fn move_item(
        &mut self,
        uuid: &str,
        delta: KicadPoint,
    ) -> Result<KicadEditSummary, String> {
        self.apply_edit(KicadSchematicEdit::MoveItem {
            uuid: uuid.to_string(),
            delta,
        })
    }

    /// rotate item。
    pub(crate) fn rotate_item(
        &mut self,
        uuid: &str,
        angle: f64,
    ) -> Result<KicadEditSummary, String> {
        self.apply_edit(KicadSchematicEdit::RotateItem {
            uuid: uuid.to_string(),
            angle,
        })
    }

    /// set symbol property。
    pub(crate) fn set_symbol_property(
        &mut self,
        reference: String,
        name: String,
        value: String,
    ) -> Result<KicadEditSummary, String> {
        self.apply_edit(KicadSchematicEdit::SetSymbolProperty {
            reference,
            name,
            value,
            at: None,
        })
    }

    /// configure symbol mirror。
    pub(crate) fn configure_symbol_mirror(
        &mut self,
        reference: String,
        mirror: Option<String>,
    ) -> Result<KicadEditSummary, String> {
        self.apply_edit(KicadSchematicEdit::ConfigureSymbol {
            reference,
            unit: None,
            body_style: None,
            mirror: Some(mirror),
            pin_alternates: None,
        })
    }

    /// place symbol from definition。
    pub(crate) fn place_symbol_from_definition(
        &mut self,
        definition: KicadSymbolDef,
        library_symbols: Vec<KicadSymbolDef>,
        at: KicadAt,
        config: SymbolPlacementConfig,
    ) -> Result<KicadSymbolPlacementResult, String> {
        let reference = self.next_reference_for_definition(&definition);
        let lib_id = definition.name.clone();
        let value = definition.property("Value").unwrap_or("").to_string();
        self.apply_edit(KicadSchematicEdit::PlaceSymbol {
            definition: Box::new(definition),
            library_symbols,
            reference: reference.clone(),
            value,
            at,
            unit: config.unit_option(),
            body_style: config.body_style,
            pin_alternates: config.pin_alternates,
            uuid: None,
        })
        .map(|summary| KicadSymbolPlacementResult {
            summary,
            reference,
            lib_id,
        })
    }

    /// add wire。
    pub(crate) fn add_wire(&mut self, points: Vec<KicadPoint>) -> Result<KicadEditSummary, String> {
        self.apply_edit(KicadSchematicEdit::AddWire { points, uuid: None })
    }

    /// add bus。
    pub(crate) fn add_bus(&mut self, points: Vec<KicadPoint>) -> Result<KicadEditSummary, String> {
        self.apply_edit(KicadSchematicEdit::AddBus { points, uuid: None })
    }

    /// add bus entry。
    pub(crate) fn add_bus_entry(
        &mut self,
        at: KicadPoint,
        size: KicadSize,
    ) -> Result<KicadEditSummary, String> {
        self.apply_edit(KicadSchematicEdit::AddBusEntry {
            at,
            size,
            uuid: None,
        })
    }

    /// add junction。
    pub(crate) fn add_junction(&mut self, at: KicadPoint) -> Result<KicadEditSummary, String> {
        self.apply_edit(KicadSchematicEdit::AddJunction { at, uuid: None })
    }

    /// add no connect。
    pub(crate) fn add_no_connect(&mut self, at: KicadPoint) -> Result<KicadEditSummary, String> {
        self.apply_edit(KicadSchematicEdit::AddNoConnect { at, uuid: None })
    }

    /// add label。
    pub(crate) fn add_label(
        &mut self,
        text: String,
        kind: KicadLabelKind,
        at: KicadAt,
    ) -> Result<KicadEditSummary, String> {
        self.apply_edit(KicadSchematicEdit::AddLabel {
            text,
            kind,
            at,
            uuid: None,
        })
    }

    /// add text。
    pub(crate) fn add_text(
        &mut self,
        text: String,
        at: KicadAt,
    ) -> Result<KicadEditSummary, String> {
        self.apply_edit(KicadSchematicEdit::AddText {
            text,
            at,
            uuid: None,
        })
    }

    /// set simulation directive。
    pub(crate) fn set_simulation_directive(
        &mut self,
        kind: KicadSimulationDirectiveKind,
        body: String,
        at: Option<KicadAt>,
    ) -> Result<KicadEditSummary, String> {
        self.apply_edit(KicadSchematicEdit::SetSimulationDirective {
            kind,
            body,
            at,
            uuid: None,
        })
    }

    /// add sheet。
    pub(crate) fn add_sheet(
        &mut self,
        name: String,
        file: String,
        at: KicadAt,
        size: KicadSize,
        pins: Vec<KicadSheetPin>,
    ) -> Result<KicadEditSummary, String> {
        self.apply_edit(KicadSchematicEdit::AddSheet {
            name,
            file,
            at,
            size,
            pins,
            uuid: None,
        })
    }

    /// save。
    fn apply_edit(&mut self, edit: KicadSchematicEdit) -> Result<KicadEditSummary, String> {
        self.schematic
            .apply_edit(edit)
            .inspect(|_| {
                self.dirty = true;
            })
            .map_err(|error| error.to_string())
    }

    fn next_reference_for_definition(&self, definition: &KicadSymbolDef) -> String {
        let prefix = reference_prefix(definition);

        let mut next = 1;
        for symbol in &self.schematic.symbols {
            let Some(reference) = symbol.reference() else {
                continue;
            };
            let Some(suffix) = reference.strip_prefix(prefix) else {
                continue;
            };
            if let Ok(number) = suffix.parse::<u32>() {
                next = next.max(number + 1);
            }
        }

        format!("{prefix}{next}")
    }
}

pub(crate) fn reference_prefix(definition: &KicadSymbolDef) -> &str {
    let prefix = definition
        .property("Reference")
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| definition.local_name())
        .trim_end_matches('?');
    if prefix.is_empty() {
        definition.local_name()
    } else {
        prefix
    }
}
