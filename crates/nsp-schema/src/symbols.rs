// schema symbol definitions and type system.
// Symbol parsing helpers are in symbols_parse_impl.rs.

use crate::coordinates::{NspAt, parse_at};
use crate::geometry::{NspBoundingBox, NspBoundingBoxBuilder, pin_body_end};
use crate::graphics::{NspSymbolGraphic, parse_symbol_graphic};
use crate::instances::{
    NspProjectInstance, parse_project_instances, write_project_instances_sexpr,
};
use crate::library_index::{NspIndexedSymbolBodyStyle, NspIndexedSymbolPin, NspIndexedSymbolUnit};
use crate::pins::{
    NspPinDef, NspPinDisplay, NspSymbolPinRef, compare_pin_numbers, parse_pin_def,
    parse_pin_display, parse_symbol_pin_ref,
};
use crate::property::{NspProperty, parse_property};
use crate::sexpr::{
    Sexp, atom_text, child, child_value, direct_children, format_number, head, list_items,
    list_value, sexpr_atom_or_string, sexpr_string,
};
use crate::style::write_optional_bool_sexpr;
use crate::symbol_library::NspSymbolLibrary;
use crate::transform::{normalize_symbol_mirror, transform_local_at};
use crate::util::{parse_bool_value, parse_footprint_filters, parse_optional_bool_child};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq)]
pub struct NspSymbolInstance {
    pub lib_id: String,
    pub lib_name: Option<String>,
    pub at: Option<NspAt>,
    pub mirror: Option<String>,
    pub unit: Option<u32>,
    pub body_style: Option<u32>,
    pub uuid: Option<String>,
    pub exclude_from_sim: Option<bool>,
    pub in_bom: Option<bool>,
    pub on_board: Option<bool>,
    pub dnp: Option<bool>,
    pub fields_autoplaced: Option<bool>,
    pub properties: Vec<NspProperty>,
    pub pins: Vec<NspSymbolPinRef>,
    pub instances: Vec<NspProjectInstance>,
}

impl NspSymbolInstance {
    /// property。
    pub fn property(&self, name: &str) -> Option<&str> {
        self.properties
            .iter()
            .find(|property| property.name == name)
            .map(|property| property.value.as_str())
    }

    /// reference。
    pub fn reference(&self) -> Option<&str> {
        self.property("Reference")
    }

    /// value。
    pub fn value(&self) -> Option<&str> {
        self.property("Value")
    }

    fn inherited_property<'a>(
        &'a self,
        definition: Option<&'a impl NspSymbolPropertySource>,
        name: &str,
    ) -> Option<&'a str> {
        self.property(name)
            .or_else(|| definition.and_then(|definition| definition.property_value(name)))
    }

    /// sim enabled。
    pub(crate) fn sim_enabled(
        &self,
        definition: Option<&impl NspSymbolPropertySource>,
    ) -> Option<bool> {
        if let Some(exclude_from_sim) = self.exclude_from_sim {
            return Some(!exclude_from_sim);
        }
        if let Some(exclude_from_sim) =
            definition.and_then(|definition| definition.exclude_from_sim_value())
        {
            return Some(!exclude_from_sim);
        }
        self.inherited_property(definition, "Sim.Enable")
            .or_else(|| self.inherited_property(definition, "Spice_Netlist_Enabled"))
            .and_then(parse_enable_value)
    }

    /// sim device。
    pub(crate) fn sim_device(
        &self,
        definition: Option<&impl NspSymbolPropertySource>,
    ) -> Option<String> {
        self.inherited_property(definition, "Sim.Device")
            .or_else(|| self.inherited_property(definition, "Spice_Primitive"))
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    }

    /// sim type — e.g. "SIN", "PULSE", "DC" for source conversion.
    pub(crate) fn sim_type(
        &self,
        definition: Option<&impl NspSymbolPropertySource>,
    ) -> Option<String> {
        self.inherited_property(definition, "Sim.Type")
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    }

    /// sim model value。
    pub(crate) fn sim_model_value(
        &self,
        definition: Option<&impl NspSymbolPropertySource>,
    ) -> Option<String> {
        if let Some(value) = self
            .inherited_property(definition, "Sim.Name")
            .or_else(|| self.inherited_property(definition, "Spice_Model"))
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            return Some(value.to_string());
        }
        self.inherited_property(definition, "Sim.Params")
            .and_then(|value| extract_named_sim_param(value, "model"))
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    }

    /// sim params value。
    pub(crate) fn sim_params_value(
        &self,
        definition: Option<&impl NspSymbolPropertySource>,
    ) -> Option<String> {
        self.inherited_property(definition, "Sim.Params")
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(strip_schema_sim_model_params)
            .filter(|value| !value.is_empty())
    }

    /// sim library。
    pub(crate) fn sim_library<'a>(
        &'a self,
        definition: Option<&'a impl NspSymbolPropertySource>,
    ) -> Option<&'a str> {
        self.inherited_property(definition, "Sim.Library")
            .or_else(|| self.inherited_property(definition, "Spice_Lib_File"))
    }

    /// sim pins。
    pub(crate) fn sim_pins<'a>(
        &'a self,
        definition: Option<&'a impl NspSymbolPropertySource>,
    ) -> Option<&'a str> {
        self.inherited_property(definition, "Sim.Pins")
            .or_else(|| self.inherited_property(definition, "Spice_Node_Sequence"))
    }

    /// has explicit sim model。
    pub(crate) fn has_explicit_sim_model(
        &self,
        definition: Option<&impl NspSymbolPropertySource>,
    ) -> bool {
        self.inherited_property(definition, "Sim.Device").is_some()
            || self.inherited_property(definition, "Sim.Params").is_some()
            || self.inherited_property(definition, "Sim.Name").is_some()
            || self
                .inherited_property(definition, "Spice_Primitive")
                .is_some()
            || self.inherited_property(definition, "Spice_Model").is_some()
    }

    /// write instance sexpr。
    pub(crate) fn write_instance_sexpr(&self, output: &mut String, indent: usize) {
        let pad = " ".repeat(indent);
        output.push_str(&format!("{}(symbol\n", pad));
        output.push_str(&format!(
            "{}  (lib_id {})\n",
            pad,
            sexpr_string(&self.lib_id)
        ));
        if let Some(at) = self.at {
            output.push_str(&format!(
                "{}  (at {} {} {})\n",
                pad,
                format_number(at.x),
                format_number(at.y),
                format_number(at.rotation)
            ));
        }
        if let Some(mirror) = &self.mirror {
            output.push_str(&format!("{}  (mirror", pad));
            for axis in mirror.split_whitespace() {
                output.push(' ');
                output.push_str(&sexpr_atom_or_string(axis));
            }
            output.push_str(")\n");
        }
        if let Some(unit) = self.unit {
            output.push_str(&format!("{}  (unit {})\n", pad, unit));
        }
        if let Some(body_style) = self.body_style {
            output.push_str(&format!("{}  (body_style {})\n", pad, body_style));
        }
        if let Some(uuid) = &self.uuid {
            output.push_str(&format!("{}  (uuid {})\n", pad, sexpr_string(uuid)));
        }
        if let Some(exclude_from_sim) = self.exclude_from_sim {
            output.push_str(&format!(
                "{}  (exclude_from_sim {})\n",
                pad,
                if exclude_from_sim { "yes" } else { "no" }
            ));
        }
        write_optional_bool_sexpr(output, indent + 2, "in_bom", self.in_bom);
        write_optional_bool_sexpr(output, indent + 2, "on_board", self.on_board);
        write_optional_bool_sexpr(output, indent + 2, "dnp", self.dnp);
        write_optional_bool_sexpr(
            output,
            indent + 2,
            "fields_autoplaced",
            self.fields_autoplaced,
        );
        for property in &self.properties {
            property.write_property_sexpr(output, indent + 2);
        }
        for pin in &self.pins {
            pin.write_pin_ref_sexpr(output, indent + 2);
        }
        write_project_instances_sexpr(output, &self.instances, indent + 2);
        output.push_str(&format!("{})\n", pad));
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NspSymbolDef {
    pub name: String,
    pub extends: Option<String>,
    pub power: Option<NspSymbolPower>,
    pub body_styles: Option<NspSymbolBodyStyles>,
    pub exclude_from_sim: Option<bool>,
    pub in_bom: Option<bool>,
    pub on_board: Option<bool>,
    pub in_pos_files: Option<bool>,
    pub duplicate_pin_numbers_are_jumpers: Option<bool>,
    pub jumper_pin_groups: Vec<Vec<String>>,
    pub embedded_fonts: Option<bool>,
    pub pin_names: Option<NspPinDisplay>,
    pub pin_numbers: Option<NspPinDisplay>,
    pub unit_names: BTreeMap<u32, String>,
    pub properties: Vec<NspProperty>,
    pub graphics: Vec<NspSymbolGraphic>,
    pub pins: Vec<NspPinDef>,
}

impl NspSymbolDef {
    /// property。
    pub fn property(&self, name: &str) -> Option<&str> {
        self.properties
            .iter()
            .find(|property| property.name == name)
            .map(|property| property.value.as_str())
    }

    /// description。
    pub fn description(&self) -> Option<&str> {
        self.property("Description")
            .filter(|value| !value.is_empty())
            .or_else(|| {
                self.property("ki_description")
                    .filter(|value| !value.is_empty())
            })
    }

    /// keywords。
    pub fn keywords(&self) -> Option<&str> {
        self.property("ki_keywords")
            .filter(|value| !value.is_empty())
    }

    /// footprint filters。
    pub fn footprint_filters(&self) -> Vec<String> {
        self.property("ki_fp_filters")
            .map(parse_footprint_filters)
            .unwrap_or_default()
    }

    /// bounding box。
    pub fn bounding_box(&self) -> Option<NspBoundingBox> {
        let mut bounds = NspBoundingBoxBuilder::default();
        for graphic in scoped_definition_graphics(self, Some(1), None) {
            graphic.include_in_bounds(&mut bounds);
        }
        for pin in scoped_symbol_pins(self, Some(1), None) {
            if let Some(at) = pin.at {
                bounds.include(at.point());
                if let Some(length) = pin.length {
                    bounds.include(pin_body_end(at, length));
                }
            }
        }
        bounds.finish()
    }

    /// local name。
    pub fn local_name(&self) -> &str {
        self.name
            .rsplit_once(':')
            .map(|(_, local_name)| local_name)
            .unwrap_or(&self.name)
    }

    /// write symbol sexpr。
    pub(crate) fn write_symbol_sexpr(&self, output: &mut String, indent: usize) {
        let pad = " ".repeat(indent);
        output.push_str(&format!("{}(symbol {}\n", pad, sexpr_string(&self.name)));
        if let Some(extends) = &self.extends {
            output.push_str(&format!("{}  (extends {})\n", pad, sexpr_string(extends)));
        }
        if let Some(power) = self.power {
            match power {
                NspSymbolPower::Bare => output.push_str(&format!("{}  (power)\n", pad)),
                NspSymbolPower::Global => output.push_str(&format!("{}  (power global)\n", pad)),
                NspSymbolPower::Local => output.push_str(&format!("{}  (power local)\n", pad)),
            }
        }
        if let Some(body_styles) = &self.body_styles {
            body_styles.write_body_styles_sexpr(output, indent + 2);
        }
        if let Some(exclude_from_sim) = self.exclude_from_sim {
            output.push_str(&format!(
                "{}  (exclude_from_sim {})\n",
                pad,
                if exclude_from_sim { "yes" } else { "no" }
            ));
        }
        write_optional_bool_sexpr(output, indent + 2, "in_bom", self.in_bom);
        write_optional_bool_sexpr(output, indent + 2, "on_board", self.on_board);
        write_optional_bool_sexpr(output, indent + 2, "in_pos_files", self.in_pos_files);
        write_optional_bool_sexpr(
            output,
            indent + 2,
            "duplicate_pin_numbers_are_jumpers",
            self.duplicate_pin_numbers_are_jumpers,
        );
        if !self.jumper_pin_groups.is_empty() {
            output.push_str(&format!("{}  (jumper_pin_groups", pad));
            for group in &self.jumper_pin_groups {
                output.push('\n');
                output.push_str(&format!("{}    (", pad));
                for (index, pin_name) in group.iter().enumerate() {
                    if index > 0 {
                        output.push(' ');
                    }
                    output.push_str(&sexpr_string(pin_name));
                }
                output.push(')');
            }
            output.push_str(&format!("\n{}  )\n", pad));
        }
        write_optional_bool_sexpr(output, indent + 2, "embedded_fonts", self.embedded_fonts);
        if let Some(pin_numbers) = &self.pin_numbers {
            pin_numbers.write_pin_numbers_sexpr(output, indent + 2);
        }
        if let Some(pin_names) = &self.pin_names {
            pin_names.write_pin_names_sexpr(output, indent + 2);
        }
        for property in &self.properties {
            property.write_property_sexpr(output, indent + 2);
        }
        for scope in self.item_scopes() {
            output.push_str(&format!(
                "{}  (symbol {}\n",
                pad,
                sexpr_string(&format!(
                    "{}_{}_{}",
                    self.local_name(),
                    scope.unit,
                    scope.body_style
                ))
            ));
            if let Some(unit_name) = self.unit_names.get(&scope.unit) {
                output.push_str(&format!(
                    "{}    (unit_name {})\n",
                    pad,
                    sexpr_string(unit_name)
                ));
            }
            for graphic in self.graphics.iter().filter(|graphic| {
                graphic.unit == scope.unit && graphic.body_style == scope.body_style
            }) {
                graphic.write_symbol_graphic_sexpr(output, indent + 4);
            }
            for pin in self
                .pins
                .iter()
                .filter(|pin| pin.unit == scope.unit && pin.body_style == scope.body_style)
            {
                pin.write_pin_sexpr(output, indent + 4);
            }
            output.push_str(&format!("{}  )\n", pad));
        }
        output.push_str(&format!("{})\n", pad));
    }

    fn item_scopes(&self) -> Vec<NspSymbolItemScope> {
        let mut scopes = self
            .graphics
            .iter()
            .map(|graphic| NspSymbolItemScope {
                unit: graphic.unit,
                body_style: graphic.body_style,
            })
            .chain(self.pins.iter().map(|pin| NspSymbolItemScope {
                unit: pin.unit,
                body_style: pin.body_style,
            }))
            .chain(self.unit_names.keys().map(|unit| NspSymbolItemScope {
                unit: *unit,
                body_style: 1,
            }))
            .collect::<BTreeSet<_>>();
        if scopes.is_empty() && self.extends.is_none() {
            scopes.insert(NspSymbolItemScope {
                unit: 0,
                body_style: 1,
            });
        }
        scopes.into_iter().collect()
    }
}

pub(crate) trait NspSymbolPropertySource {
    fn property_value(&self, name: &str) -> Option<&str>;
    fn exclude_from_sim_value(&self) -> Option<bool>;
}

impl NspSymbolPropertySource for NspSymbolDef {
    fn property_value(&self, name: &str) -> Option<&str> {
        self.property(name)
    }

    fn exclude_from_sim_value(&self) -> Option<bool> {
        self.exclude_from_sim
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct NspResolvedSymbolDef {
    pub(crate) name: String,
    pub(crate) exclude_from_sim: Option<bool>,
    pub(crate) body_styles: Option<NspSymbolBodyStyles>,
    pub(crate) pin_names: Option<NspPinDisplay>,
    pub(crate) pin_numbers: Option<NspPinDisplay>,
    pub(crate) unit_names: BTreeMap<u32, String>,
    pub(crate) properties: Vec<NspProperty>,
    pub(crate) graphics: Vec<NspSymbolGraphic>,
    pub(crate) pins: Vec<NspPinDef>,
}

impl NspResolvedSymbolDef {
    /// from symbol。
    pub(crate) fn from_symbol(symbol: &NspSymbolDef) -> Self {
        Self {
            name: symbol.name.clone(),
            exclude_from_sim: symbol.exclude_from_sim,
            body_styles: symbol.body_styles.clone(),
            pin_names: symbol.pin_names.clone(),
            pin_numbers: symbol.pin_numbers.clone(),
            unit_names: symbol.unit_names.clone(),
            properties: symbol.properties.clone(),
            graphics: symbol.graphics.clone(),
            pins: symbol.pins.clone(),
        }
    }

    /// property。
    pub(crate) fn property(&self, name: &str) -> Option<&str> {
        self.properties
            .iter()
            .find(|property| property.name == name)
            .map(|property| property.value.as_str())
    }

    /// description。
    pub(crate) fn description(&self) -> Option<&str> {
        self.property("Description")
            .filter(|value| !value.is_empty())
            .or_else(|| {
                self.property("ki_description")
                    .filter(|value| !value.is_empty())
            })
    }

    /// keywords。
    pub(crate) fn keywords(&self) -> Option<&str> {
        self.property("ki_keywords")
            .filter(|value| !value.is_empty())
    }

    /// footprint filters。
    pub(crate) fn footprint_filters(&self) -> Vec<String> {
        self.property("ki_fp_filters")
            .map(parse_footprint_filters)
            .unwrap_or_default()
    }

    /// bounding box。
    pub(crate) fn bounding_box(&self) -> Option<NspBoundingBox> {
        let mut bounds = NspBoundingBoxBuilder::default();
        for graphic in self.scoped_graphics(Some(1), None) {
            graphic.include_in_bounds(&mut bounds);
        }
        for pin in self.scoped_pins(Some(1), None) {
            if let Some(at) = pin.at {
                bounds.include(at.point());
                if let Some(length) = pin.length {
                    bounds.include(pin_body_end(at, length));
                }
            }
        }
        bounds.finish()
    }

    /// indexed units。
    pub(crate) fn indexed_units(&self) -> Vec<NspIndexedSymbolUnit> {
        let mut units = self
            .pins
            .iter()
            .map(|pin| pin.unit)
            .chain(self.graphics.iter().map(|graphic| graphic.unit))
            .chain(self.unit_names.keys().copied())
            .filter(|unit| *unit != 0)
            .collect::<BTreeSet<_>>();
        if units.is_empty() {
            units.insert(1);
        }
        units
            .into_iter()
            .map(|unit| NspIndexedSymbolUnit {
                unit,
                name: self.unit_names.get(&unit).cloned(),
            })
            .collect()
    }

    /// unit count。
    pub(crate) fn unit_count(&self) -> usize {
        self.indexed_units().len()
    }

    /// indexed body styles。
    pub(crate) fn indexed_body_styles(&self) -> Vec<NspIndexedSymbolBodyStyle> {
        let mut body_styles = self
            .pins
            .iter()
            .map(|pin| pin.body_style)
            .chain(self.graphics.iter().map(|graphic| graphic.body_style))
            .filter(|body_style| *body_style != 0)
            .collect::<BTreeSet<_>>();
        if let Some(declared_body_styles) = &self.body_styles {
            body_styles.extend(declared_body_styles.body_style_numbers());
        }

        body_styles
            .into_iter()
            .map(|body_style| NspIndexedSymbolBodyStyle {
                body_style,
                name: self.body_style_name(body_style),
            })
            .collect()
    }

    fn body_style_name(&self, body_style: u32) -> Option<String> {
        match &self.body_styles {
            Some(NspSymbolBodyStyles::Demorgan) => match body_style {
                1 => Some("normal".to_string()),
                2 => Some("demorgan".to_string()),
                _ => None,
            },
            Some(NspSymbolBodyStyles::Names(names)) => {
                names.get(body_style.saturating_sub(1) as usize).cloned()
            }
            None => None,
        }
    }

    /// indexed pins。
    pub(crate) fn indexed_pins(&self) -> Vec<NspIndexedSymbolPin> {
        self.pins
            .iter()
            .map(|pin| NspIndexedSymbolPin {
                number: pin.number().to_string(),
                name: pin.name().to_string(),
                electrical_type: pin.electrical_type.clone(),
                shape: pin.shape.clone(),
                unit: pin.unit,
                body_style: pin.body_style,
                alternates: pin.alternates.clone(),
            })
            .collect()
    }

    /// scoped graphics。
    pub(crate) fn scoped_graphics(
        &self,
        unit: Option<u32>,
        body_style: Option<u32>,
    ) -> impl Iterator<Item = &NspSymbolGraphic> {
        scoped_symbol_items(&self.graphics, unit, body_style, |graphic| {
            (graphic.unit, graphic.body_style)
        })
    }

    /// scoped pins。
    pub(crate) fn scoped_pins(
        &self,
        unit: Option<u32>,
        body_style: Option<u32>,
    ) -> impl Iterator<Item = &NspPinDef> {
        scoped_symbol_items(&self.pins, unit, body_style, |pin| {
            (pin.unit, pin.body_style)
        })
    }
}

impl NspSymbolPropertySource for NspResolvedSymbolDef {
    fn property_value(&self, name: &str) -> Option<&str> {
        self.property(name)
    }

    fn exclude_from_sim_value(&self) -> Option<bool> {
        self.exclude_from_sim
    }
}

/// resolve symbol definition。
pub(crate) fn resolve_symbol_definition(
    symbol: &NspSymbolDef,
    library_symbols: &[NspSymbolDef],
) -> Option<NspResolvedSymbolDef> {
    let mut chain = Vec::new();
    let mut current = symbol;
    let mut visited = BTreeSet::new();

    loop {
        if !visited.insert(current.name.clone()) {
            return Some(NspResolvedSymbolDef::from_symbol(symbol));
        }
        chain.push(current);
        let Some(parent_name) = current.extends.as_deref() else {
            break;
        };
        let Some(parent) = find_symbol_inheritance_parent(current, parent_name, library_symbols)
        else {
            return Some(NspResolvedSymbolDef::from_symbol(symbol));
        };
        current = parent;
    }

    let mut chain = chain.into_iter().rev();
    let root = chain.next()?;
    let mut resolved = NspResolvedSymbolDef::from_symbol(root);
    for derived in chain {
        apply_symbol_inheritance_overrides(&mut resolved, derived);
    }
    resolved.name = symbol.name.clone();
    Some(resolved)
}

/// find symbol inheritance parent。
pub(crate) fn find_symbol_inheritance_parent<'a>(
    symbol: &NspSymbolDef,
    parent_name: &str,
    library_symbols: &'a [NspSymbolDef],
) -> Option<&'a NspSymbolDef> {
    library_symbols
        .iter()
        .find(|candidate| candidate.name == parent_name)
        .or_else(|| {
            symbol
                .name
                .rsplit_once(':')
                .map(|(library, _)| format!("{library}:{parent_name}"))
                .and_then(|qualified_parent| {
                    library_symbols
                        .iter()
                        .find(|candidate| candidate.name == qualified_parent)
                })
        })
        .or_else(|| {
            library_symbols
                .iter()
                .find(|candidate| candidate.local_name() == parent_name)
        })
}

fn apply_symbol_inheritance_overrides(resolved: &mut NspResolvedSymbolDef, derived: &NspSymbolDef) {
    resolved.exclude_from_sim = derived.exclude_from_sim.or(resolved.exclude_from_sim);
    resolved.pin_names = derived
        .pin_names
        .clone()
        .or_else(|| resolved.pin_names.clone());
    resolved.pin_numbers = derived
        .pin_numbers
        .clone()
        .or_else(|| resolved.pin_numbers.clone());
    resolved.body_styles = derived
        .body_styles
        .clone()
        .or_else(|| resolved.body_styles.clone());
    for (unit, name) in &derived.unit_names {
        resolved.unit_names.insert(*unit, name.clone());
    }
    for property in &derived.properties {
        if is_inherited_symbol_browser_property(&property.name)
            && property.value.trim().is_empty()
            && resolved.property(&property.name).is_some()
        {
            continue;
        }
        if is_effective_symbol_property_override(property) {
            upsert_symbol_property(&mut resolved.properties, property.clone());
        }
    }
    if !derived.graphics.is_empty() {
        resolved.graphics.extend(derived.graphics.clone());
    }
    if !derived.pins.is_empty() {
        resolved.pins.extend(derived.pins.clone());
    }
}

fn is_effective_symbol_property_override(property: &NspProperty) -> bool {
    !matches!(property.name.as_str(), "Reference" | "Value") || !property.value.trim().is_empty()
}

fn upsert_symbol_property(properties: &mut Vec<NspProperty>, property: NspProperty) {
    if let Some(existing) = properties
        .iter_mut()
        .find(|existing| existing.name == property.name)
    {
        *existing = property;
    } else {
        properties.push(property);
    }
}

fn is_inherited_symbol_browser_property(name: &str) -> bool {
    matches!(
        name,
        "Description" | "ki_description" | "ki_keywords" | "ki_fp_filters"
    )
}

include!("symbols_parse_impl.rs");
