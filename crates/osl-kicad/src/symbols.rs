use crate::coordinates::{KicadAt, parse_at};
use crate::geometry::{KicadBoundingBox, KicadBoundingBoxBuilder, pin_body_end};
use crate::graphics::{KicadSymbolGraphic, parse_symbol_graphic};
use crate::instances::{
    KicadProjectInstance, parse_project_instances, write_project_instances_sexpr,
};
use crate::library_index::{
    KicadIndexedSymbolBodyStyle, KicadIndexedSymbolPin, KicadIndexedSymbolUnit,
};
use crate::pins::{
    KicadPinDef, KicadPinDisplay, KicadSymbolPinRef, compare_pin_numbers, parse_pin_def,
    parse_pin_display, parse_symbol_pin_ref,
};
use crate::property::{KicadProperty, parse_property};
use crate::sexpr::{
    Sexp, atom_text, child, child_value, direct_children, format_number, head, list_items,
    list_value, sexpr_atom_or_string, sexpr_string,
};
use crate::style::write_optional_bool_sexpr;
use crate::symbol_library::KicadSymbolLibrary;
use crate::transform::{normalize_symbol_mirror, transform_local_at};
use crate::util::{
    parse_kicad_bool_value, parse_kicad_footprint_filters, parse_optional_bool_child,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq)]
pub struct KicadSymbolInstance {
    pub lib_id: String,
    pub at: Option<KicadAt>,
    pub mirror: Option<String>,
    pub unit: Option<u32>,
    pub body_style: Option<u32>,
    pub uuid: Option<String>,
    pub exclude_from_sim: Option<bool>,
    pub in_bom: Option<bool>,
    pub on_board: Option<bool>,
    pub dnp: Option<bool>,
    pub fields_autoplaced: Option<bool>,
    pub properties: Vec<KicadProperty>,
    pub pins: Vec<KicadSymbolPinRef>,
    pub instances: Vec<KicadProjectInstance>,
}

impl KicadSymbolInstance {
    pub fn property(&self, name: &str) -> Option<&str> {
        self.properties
            .iter()
            .find(|property| property.name == name)
            .map(|property| property.value.as_str())
    }

    pub fn reference(&self) -> Option<&str> {
        self.property("Reference")
    }

    pub fn value(&self) -> Option<&str> {
        self.property("Value")
    }

    fn inherited_property<'a>(
        &'a self,
        definition: Option<&'a impl KicadSymbolPropertySource>,
        name: &str,
    ) -> Option<&'a str> {
        self.property(name)
            .or_else(|| definition.and_then(|definition| definition.property_value(name)))
    }

    pub(crate) fn sim_enabled(
        &self,
        definition: Option<&impl KicadSymbolPropertySource>,
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
            .and_then(parse_kicad_enable_value)
    }

    pub(crate) fn sim_device(
        &self,
        definition: Option<&impl KicadSymbolPropertySource>,
    ) -> Option<String> {
        self.inherited_property(definition, "Sim.Device")
            .or_else(|| self.inherited_property(definition, "Spice_Primitive"))
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    }

    pub(crate) fn sim_model_value(
        &self,
        definition: Option<&impl KicadSymbolPropertySource>,
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

    pub(crate) fn sim_params_value(
        &self,
        definition: Option<&impl KicadSymbolPropertySource>,
    ) -> Option<String> {
        self.inherited_property(definition, "Sim.Params")
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(strip_kicad_sim_model_params)
            .filter(|value| !value.is_empty())
    }

    pub(crate) fn sim_library<'a>(
        &'a self,
        definition: Option<&'a impl KicadSymbolPropertySource>,
    ) -> Option<&'a str> {
        self.inherited_property(definition, "Sim.Library")
            .or_else(|| self.inherited_property(definition, "Spice_Lib_File"))
    }

    pub(crate) fn sim_pins<'a>(
        &'a self,
        definition: Option<&'a impl KicadSymbolPropertySource>,
    ) -> Option<&'a str> {
        self.inherited_property(definition, "Sim.Pins")
            .or_else(|| self.inherited_property(definition, "Spice_Node_Sequence"))
    }

    pub(crate) fn has_explicit_sim_model(
        &self,
        definition: Option<&impl KicadSymbolPropertySource>,
    ) -> bool {
        self.inherited_property(definition, "Sim.Device").is_some()
            || self.inherited_property(definition, "Sim.Params").is_some()
            || self.inherited_property(definition, "Sim.Name").is_some()
            || self
                .inherited_property(definition, "Spice_Primitive")
                .is_some()
            || self.inherited_property(definition, "Spice_Model").is_some()
    }

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
pub struct KicadSymbolDef {
    pub name: String,
    pub extends: Option<String>,
    pub power: Option<KicadSymbolPower>,
    pub body_styles: Option<KicadSymbolBodyStyles>,
    pub exclude_from_sim: Option<bool>,
    pub in_bom: Option<bool>,
    pub on_board: Option<bool>,
    pub in_pos_files: Option<bool>,
    pub duplicate_pin_numbers_are_jumpers: Option<bool>,
    pub jumper_pin_groups: Vec<Vec<String>>,
    pub embedded_fonts: Option<bool>,
    pub pin_names: Option<KicadPinDisplay>,
    pub pin_numbers: Option<KicadPinDisplay>,
    pub unit_names: BTreeMap<u32, String>,
    pub properties: Vec<KicadProperty>,
    pub graphics: Vec<KicadSymbolGraphic>,
    pub pins: Vec<KicadPinDef>,
}

impl KicadSymbolDef {
    pub fn property(&self, name: &str) -> Option<&str> {
        self.properties
            .iter()
            .find(|property| property.name == name)
            .map(|property| property.value.as_str())
    }

    pub fn description(&self) -> Option<&str> {
        self.property("Description")
            .filter(|value| !value.is_empty())
            .or_else(|| {
                self.property("ki_description")
                    .filter(|value| !value.is_empty())
            })
    }

    pub fn keywords(&self) -> Option<&str> {
        self.property("ki_keywords")
            .filter(|value| !value.is_empty())
    }

    pub fn footprint_filters(&self) -> Vec<String> {
        self.property("ki_fp_filters")
            .map(parse_kicad_footprint_filters)
            .unwrap_or_default()
    }

    pub fn bounding_box(&self) -> Option<KicadBoundingBox> {
        let mut bounds = KicadBoundingBoxBuilder::default();
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

    pub fn local_name(&self) -> &str {
        self.name
            .rsplit_once(':')
            .map(|(_, local_name)| local_name)
            .unwrap_or(&self.name)
    }

    pub(crate) fn write_symbol_sexpr(&self, output: &mut String, indent: usize) {
        let pad = " ".repeat(indent);
        output.push_str(&format!("{}(symbol {}\n", pad, sexpr_string(&self.name)));
        if let Some(extends) = &self.extends {
            output.push_str(&format!("{}  (extends {})\n", pad, sexpr_string(extends)));
        }
        if let Some(power) = self.power {
            match power {
                KicadSymbolPower::Bare => output.push_str(&format!("{}  (power)\n", pad)),
                KicadSymbolPower::Global => output.push_str(&format!("{}  (power global)\n", pad)),
                KicadSymbolPower::Local => output.push_str(&format!("{}  (power local)\n", pad)),
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

    fn item_scopes(&self) -> Vec<KicadSymbolItemScope> {
        let mut scopes = self
            .graphics
            .iter()
            .map(|graphic| KicadSymbolItemScope {
                unit: graphic.unit,
                body_style: graphic.body_style,
            })
            .chain(self.pins.iter().map(|pin| KicadSymbolItemScope {
                unit: pin.unit,
                body_style: pin.body_style,
            }))
            .chain(self.unit_names.keys().map(|unit| KicadSymbolItemScope {
                unit: *unit,
                body_style: 1,
            }))
            .collect::<BTreeSet<_>>();
        if scopes.is_empty() && self.extends.is_none() {
            scopes.insert(KicadSymbolItemScope {
                unit: 0,
                body_style: 1,
            });
        }
        scopes.into_iter().collect()
    }
}

pub(crate) trait KicadSymbolPropertySource {
    fn property_value(&self, name: &str) -> Option<&str>;
    fn exclude_from_sim_value(&self) -> Option<bool>;
}

impl KicadSymbolPropertySource for KicadSymbolDef {
    fn property_value(&self, name: &str) -> Option<&str> {
        self.property(name)
    }

    fn exclude_from_sim_value(&self) -> Option<bool> {
        self.exclude_from_sim
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct KicadResolvedSymbolDef {
    pub(crate) name: String,
    pub(crate) exclude_from_sim: Option<bool>,
    pub(crate) body_styles: Option<KicadSymbolBodyStyles>,
    pub(crate) pin_names: Option<KicadPinDisplay>,
    pub(crate) pin_numbers: Option<KicadPinDisplay>,
    pub(crate) unit_names: BTreeMap<u32, String>,
    pub(crate) properties: Vec<KicadProperty>,
    pub(crate) graphics: Vec<KicadSymbolGraphic>,
    pub(crate) pins: Vec<KicadPinDef>,
}

impl KicadResolvedSymbolDef {
    pub(crate) fn from_symbol(symbol: &KicadSymbolDef) -> Self {
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

    pub(crate) fn property(&self, name: &str) -> Option<&str> {
        self.properties
            .iter()
            .find(|property| property.name == name)
            .map(|property| property.value.as_str())
    }

    pub(crate) fn description(&self) -> Option<&str> {
        self.property("Description")
            .filter(|value| !value.is_empty())
            .or_else(|| {
                self.property("ki_description")
                    .filter(|value| !value.is_empty())
            })
    }

    pub(crate) fn keywords(&self) -> Option<&str> {
        self.property("ki_keywords")
            .filter(|value| !value.is_empty())
    }

    pub(crate) fn footprint_filters(&self) -> Vec<String> {
        self.property("ki_fp_filters")
            .map(parse_kicad_footprint_filters)
            .unwrap_or_default()
    }

    pub(crate) fn bounding_box(&self) -> Option<KicadBoundingBox> {
        let mut bounds = KicadBoundingBoxBuilder::default();
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

    pub(crate) fn indexed_units(&self) -> Vec<KicadIndexedSymbolUnit> {
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
            .map(|unit| KicadIndexedSymbolUnit {
                unit,
                name: self.unit_names.get(&unit).cloned(),
            })
            .collect()
    }

    pub(crate) fn unit_count(&self) -> usize {
        self.indexed_units().len()
    }

    pub(crate) fn indexed_body_styles(&self) -> Vec<KicadIndexedSymbolBodyStyle> {
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
            .map(|body_style| KicadIndexedSymbolBodyStyle {
                body_style,
                name: self.body_style_name(body_style),
            })
            .collect()
    }

    fn body_style_name(&self, body_style: u32) -> Option<String> {
        match &self.body_styles {
            Some(KicadSymbolBodyStyles::Demorgan) => match body_style {
                1 => Some("normal".to_string()),
                2 => Some("demorgan".to_string()),
                _ => None,
            },
            Some(KicadSymbolBodyStyles::Names(names)) => {
                names.get(body_style.saturating_sub(1) as usize).cloned()
            }
            None => None,
        }
    }

    pub(crate) fn indexed_pins(&self) -> Vec<KicadIndexedSymbolPin> {
        self.pins
            .iter()
            .map(|pin| KicadIndexedSymbolPin {
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

    pub(crate) fn scoped_graphics(
        &self,
        unit: Option<u32>,
        body_style: Option<u32>,
    ) -> impl Iterator<Item = &KicadSymbolGraphic> {
        scoped_symbol_items(&self.graphics, unit, body_style, |graphic| {
            (graphic.unit, graphic.body_style)
        })
    }

    pub(crate) fn scoped_pins(
        &self,
        unit: Option<u32>,
        body_style: Option<u32>,
    ) -> impl Iterator<Item = &KicadPinDef> {
        scoped_symbol_items(&self.pins, unit, body_style, |pin| {
            (pin.unit, pin.body_style)
        })
    }
}

impl KicadSymbolPropertySource for KicadResolvedSymbolDef {
    fn property_value(&self, name: &str) -> Option<&str> {
        self.property(name)
    }

    fn exclude_from_sim_value(&self) -> Option<bool> {
        self.exclude_from_sim
    }
}

pub(crate) fn resolve_symbol_definition(
    symbol: &KicadSymbolDef,
    library_symbols: &[KicadSymbolDef],
) -> Option<KicadResolvedSymbolDef> {
    let mut chain = Vec::new();
    let mut current = symbol;
    let mut visited = BTreeSet::new();

    loop {
        if !visited.insert(current.name.clone()) {
            return Some(KicadResolvedSymbolDef::from_symbol(symbol));
        }
        chain.push(current);
        let Some(parent_name) = current.extends.as_deref() else {
            break;
        };
        let Some(parent) = find_symbol_inheritance_parent(current, parent_name, library_symbols)
        else {
            return Some(KicadResolvedSymbolDef::from_symbol(symbol));
        };
        current = parent;
    }

    let mut chain = chain.into_iter().rev();
    let root = chain.next()?;
    let mut resolved = KicadResolvedSymbolDef::from_symbol(root);
    for derived in chain {
        apply_symbol_inheritance_overrides(&mut resolved, derived);
    }
    resolved.name = symbol.name.clone();
    Some(resolved)
}

pub(crate) fn find_symbol_inheritance_parent<'a>(
    symbol: &KicadSymbolDef,
    parent_name: &str,
    library_symbols: &'a [KicadSymbolDef],
) -> Option<&'a KicadSymbolDef> {
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

fn apply_symbol_inheritance_overrides(
    resolved: &mut KicadResolvedSymbolDef,
    derived: &KicadSymbolDef,
) {
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

fn is_effective_symbol_property_override(property: &KicadProperty) -> bool {
    !matches!(property.name.as_str(), "Reference" | "Value") || !property.value.trim().is_empty()
}

fn upsert_symbol_property(properties: &mut Vec<KicadProperty>, property: KicadProperty) {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KicadSymbolPower {
    Bare,
    Global,
    Local,
}

impl KicadSymbolPower {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Bare => "bare",
            Self::Global => "global",
            Self::Local => "local",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KicadSymbolBodyStyles {
    Demorgan,
    Names(Vec<String>),
}

impl KicadSymbolBodyStyles {
    fn body_style_numbers(&self) -> Vec<u32> {
        match self {
            Self::Demorgan => vec![1, 2],
            Self::Names(names) => (1..=names.len() as u32).collect(),
        }
    }

    fn write_body_styles_sexpr(&self, output: &mut String, indent: usize) {
        let pad = " ".repeat(indent);
        output.push_str(&format!("{}(body_styles", pad));
        match self {
            Self::Demorgan => output.push_str(" demorgan"),
            Self::Names(names) => {
                for name in names {
                    output.push(' ');
                    output.push_str(&sexpr_atom_or_string(name));
                }
            }
        }
        output.push_str(")\n");
    }
}

pub(crate) fn parse_symbol_instance(node: &Sexp) -> Option<KicadSymbolInstance> {
    let items = list_items(node);
    Some(KicadSymbolInstance {
        lib_id: child_value(items, "lib_id")?,
        at: child(items, "at").and_then(parse_at),
        mirror: child(items, "mirror").and_then(parse_symbol_mirror),
        unit: child_value(items, "unit").and_then(|value| value.parse().ok()),
        body_style: child_value(items, "body_style")
            .or_else(|| child_value(items, "convert"))
            .and_then(|value| value.parse().ok()),
        uuid: child_value(items, "uuid"),
        exclude_from_sim: child_value(items, "exclude_from_sim").and_then(parse_kicad_bool_value),
        in_bom: child_value(items, "in_bom").and_then(parse_kicad_bool_value),
        on_board: child_value(items, "on_board").and_then(parse_kicad_bool_value),
        dnp: child_value(items, "dnp").and_then(parse_kicad_bool_value),
        fields_autoplaced: parse_optional_bool_child(items, "fields_autoplaced"),
        properties: direct_children(items, "property")
            .filter_map(parse_property)
            .collect(),
        pins: direct_children(items, "pin")
            .filter_map(parse_symbol_pin_ref)
            .collect(),
        instances: child(items, "instances")
            .map(parse_project_instances)
            .unwrap_or_default(),
    })
}

fn parse_symbol_mirror(node: &Sexp) -> Option<String> {
    let mirror = list_items(node)
        .iter()
        .skip(1)
        .filter_map(atom_text)
        .collect::<Vec<_>>()
        .join(" ");
    normalize_symbol_mirror(&mirror).ok().flatten()
}

pub(crate) fn parse_symbol_def(node: &Sexp) -> Option<KicadSymbolDef> {
    let items = list_items(node);
    Some(KicadSymbolDef {
        name: list_value(node, 1)?,
        extends: child_value(items, "extends"),
        power: child(items, "power").map(parse_symbol_power),
        body_styles: child(items, "body_styles").and_then(parse_symbol_body_styles),
        exclude_from_sim: child_value(items, "exclude_from_sim").and_then(parse_kicad_bool_value),
        in_bom: child_value(items, "in_bom").and_then(parse_kicad_bool_value),
        on_board: child_value(items, "on_board").and_then(parse_kicad_bool_value),
        in_pos_files: child_value(items, "in_pos_files").and_then(parse_kicad_bool_value),
        duplicate_pin_numbers_are_jumpers: child_value(items, "duplicate_pin_numbers_are_jumpers")
            .and_then(parse_kicad_bool_value),
        jumper_pin_groups: child(items, "jumper_pin_groups")
            .map(parse_jumper_pin_groups)
            .unwrap_or_default(),
        embedded_fonts: child_value(items, "embedded_fonts").and_then(parse_kicad_bool_value),
        pin_names: child(items, "pin_names").map(parse_pin_display),
        pin_numbers: child(items, "pin_numbers").map(parse_pin_display),
        unit_names: collect_symbol_unit_names(node),
        properties: direct_children(items, "property")
            .filter_map(parse_property)
            .collect(),
        graphics: collect_graphics(node),
        pins: collect_pin_defs(node),
    })
}

fn collect_symbol_unit_names(node: &Sexp) -> BTreeMap<u32, String> {
    let mut unit_names = BTreeMap::new();
    collect_symbol_unit_names_into(node, &mut unit_names);
    unit_names
}

fn collect_symbol_unit_names_into(node: &Sexp, unit_names: &mut BTreeMap<u32, String>) {
    if let Some(scope) = child_symbol_item_scope(node)
        && scope.unit != 0
        && let Some(unit_name) = child_value(list_items(node), "unit_name")
    {
        unit_names.insert(scope.unit, unit_name);
    }
    for child in list_items(node) {
        if matches!(child, Sexp::List(_)) {
            collect_symbol_unit_names_into(child, unit_names);
        }
    }
}

fn parse_symbol_power(node: &Sexp) -> KicadSymbolPower {
    match list_value(node, 1)
        .as_deref()
        .map(str::trim)
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("global") => KicadSymbolPower::Global,
        Some("local") => KicadSymbolPower::Local,
        _ => KicadSymbolPower::Bare,
    }
}

fn parse_symbol_body_styles(node: &Sexp) -> Option<KicadSymbolBodyStyles> {
    let names = list_items(node)
        .iter()
        .skip(1)
        .filter_map(atom_text)
        .map(str::to_string)
        .collect::<Vec<_>>();
    if names.iter().any(|name| name == "demorgan") {
        Some(KicadSymbolBodyStyles::Demorgan)
    } else if names.is_empty() {
        None
    } else {
        Some(KicadSymbolBodyStyles::Names(names))
    }
}

fn parse_jumper_pin_groups(node: &Sexp) -> Vec<Vec<String>> {
    list_items(node)
        .iter()
        .skip(1)
        .filter_map(|group| {
            let pins = list_items(group)
                .iter()
                .filter_map(atom_text)
                .map(str::to_string)
                .collect::<Vec<_>>();
            (!pins.is_empty()).then_some(pins)
        })
        .collect()
}

fn collect_pin_defs(node: &Sexp) -> Vec<KicadPinDef> {
    let mut pins = Vec::new();
    collect_pin_defs_into(node, KicadSymbolItemScope::default(), &mut pins);
    pins
}

fn collect_pin_defs_into(node: &Sexp, scope: KicadSymbolItemScope, pins: &mut Vec<KicadPinDef>) {
    if head(node) == Some("pin")
        && let Some(mut pin) = parse_pin_def(node)
    {
        pin.unit = scope.unit;
        pin.body_style = scope.body_style;
        pins.push(pin);
    }
    for child in list_items(node) {
        if matches!(child, Sexp::List(_)) {
            let child_scope = child_symbol_item_scope(child).unwrap_or(scope);
            collect_pin_defs_into(child, child_scope, pins);
        }
    }
}

fn collect_graphics(node: &Sexp) -> Vec<KicadSymbolGraphic> {
    let mut graphics = Vec::new();
    collect_graphics_into(node, KicadSymbolItemScope::default(), &mut graphics);
    graphics
}

fn collect_graphics_into(
    node: &Sexp,
    scope: KicadSymbolItemScope,
    graphics: &mut Vec<KicadSymbolGraphic>,
) {
    if let Some(graphic) = parse_symbol_graphic(node) {
        graphics.push(KicadSymbolGraphic {
            unit: scope.unit,
            body_style: scope.body_style,
            ..graphic
        });
    }
    for child in list_items(node) {
        if matches!(child, Sexp::List(_)) {
            let child_scope = child_symbol_item_scope(child).unwrap_or(scope);
            collect_graphics_into(child, child_scope, graphics);
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
struct KicadSymbolItemScope {
    unit: u32,
    body_style: u32,
}

fn child_symbol_item_scope(node: &Sexp) -> Option<KicadSymbolItemScope> {
    if head(node) != Some("symbol") {
        return None;
    }
    parse_symbol_item_scope(list_value(node, 1)?.as_str())
}

fn parse_symbol_item_scope(name: &str) -> Option<KicadSymbolItemScope> {
    let (_, body_style) = name.rsplit_once('_')?;
    let (base, unit) = name[..name.len() - body_style.len() - 1].rsplit_once('_')?;
    if base.is_empty() {
        return None;
    }
    Some(KicadSymbolItemScope {
        unit: unit.parse().ok()?,
        body_style: body_style.parse().ok()?,
    })
}

pub(crate) fn library_symbol_definition_for_lib_id(
    library: &KicadSymbolLibrary,
    library_name: &str,
    lib_id: &str,
) -> Option<KicadSymbolDef> {
    if let Some(symbol) = library.symbol(lib_id) {
        return Some(symbol.clone());
    }

    let (requested_library, requested_name) = lib_id.split_once(':')?;
    if requested_library != library_name {
        return None;
    }

    library
        .symbols
        .iter()
        .find(|symbol| symbol.name == requested_name || symbol.local_name() == requested_name)
        .cloned()
        .map(|mut symbol| {
            qualify_library_symbol_name(&mut symbol, library_name);
            symbol
        })
}

pub(crate) fn qualify_library_symbol_name(symbol: &mut KicadSymbolDef, library_name: &str) {
    if !symbol.name.contains(':') {
        symbol.name = format!("{library_name}:{}", symbol.name);
    }
}

pub(crate) fn symbol_ordered_pins<'a>(
    symbol: &'a KicadSymbolInstance,
    definition: &'a KicadResolvedSymbolDef,
) -> Vec<&'a KicadPinDef> {
    let scoped_pins = definition
        .scoped_pins(symbol.unit, symbol.body_style)
        .collect::<Vec<_>>();
    let mut by_number = scoped_pins
        .iter()
        .copied()
        .map(|pin| (pin.number(), pin))
        .collect::<BTreeMap<_, _>>();
    let by_name = scoped_pins
        .iter()
        .copied()
        .map(|pin| (pin.name(), pin))
        .collect::<BTreeMap<_, _>>();
    let mut ordered = Vec::new();

    for pin_number in symbol_sim_pin_order(symbol, definition) {
        if let Some(pin) = by_number.remove(pin_number.as_str()) {
            ordered.push(pin);
        } else if let Some(pin) = by_name.get(pin_number.as_str()) {
            ordered.push(*pin);
        }
    }

    if ordered.is_empty() {
        ordered = scoped_pins;
        ordered.sort_by(compare_pin_numbers);
    }

    ordered
}

fn scoped_symbol_pins<'a>(
    definition: &'a KicadSymbolDef,
    unit: Option<u32>,
    body_style: Option<u32>,
) -> impl Iterator<Item = &'a KicadPinDef> + 'a {
    let unit = unit.unwrap_or(1);
    let body_style = body_style.unwrap_or(1);
    definition
        .pins
        .iter()
        .filter(move |pin| symbol_item_scope_matches(pin.unit, pin.body_style, unit, body_style))
}

fn scoped_definition_graphics<'a>(
    definition: &'a KicadSymbolDef,
    unit: Option<u32>,
    body_style: Option<u32>,
) -> impl Iterator<Item = &'a KicadSymbolGraphic> + 'a {
    let unit = unit.unwrap_or(1);
    let body_style = body_style.unwrap_or(1);
    definition.graphics.iter().filter(move |graphic| {
        symbol_item_scope_matches(graphic.unit, graphic.body_style, unit, body_style)
    })
}

fn scoped_symbol_items<'a, T>(
    items: &'a [T],
    unit: Option<u32>,
    body_style: Option<u32>,
    scope: impl Fn(&T) -> (u32, u32) + 'a,
) -> impl Iterator<Item = &'a T> + 'a {
    let unit = unit.unwrap_or(1);
    let body_style = body_style.unwrap_or(1);
    items.iter().filter(move |item| {
        let (item_unit, item_body_style) = scope(item);
        symbol_item_scope_matches(item_unit, item_body_style, unit, body_style)
    })
}

pub(crate) fn symbol_item_scope_matches(
    item_unit: u32,
    item_body_style: u32,
    selected_unit: u32,
    selected_body_style: u32,
) -> bool {
    (item_unit == 0 || item_unit == selected_unit)
        && (item_body_style == 0 || item_body_style == selected_body_style)
}

pub(crate) fn symbol_sim_pin_order(
    symbol: &KicadSymbolInstance,
    definition: &KicadResolvedSymbolDef,
) -> Vec<String> {
    let Some(pins) = symbol.sim_pins(Some(definition)) else {
        return Vec::new();
    };
    parse_sim_pin_order(pins)
}

fn parse_sim_pin_order(value: &str) -> Vec<String> {
    value
        .split(|character: char| character.is_ascii_whitespace() || character == ',')
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .filter_map(|token| {
            let symbol_pin = token.split_once('=').map(|(left, _)| left).unwrap_or(token);
            let symbol_pin = symbol_pin.trim();
            (!symbol_pin.is_empty()).then(|| symbol_pin.to_string())
        })
        .collect()
}

fn parse_kicad_enable_value(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "y" | "yes" | "true" | "1" | "on" => Some(true),
        "n" | "no" | "false" | "0" | "off" => Some(false),
        _ => None,
    }
}

fn strip_kicad_sim_model_params(value: &str) -> String {
    split_spice_tokens(value)
        .into_iter()
        .filter(|token| {
            token
                .split_once('=')
                .map(|(name, _)| {
                    !matches!(name.trim().to_ascii_lowercase().as_str(), "model" | "lib")
                })
                .unwrap_or(true)
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn extract_named_sim_param(value: &str, name: &str) -> Option<String> {
    for token in split_spice_tokens(value) {
        let Some((left, right)) = token.split_once('=') else {
            continue;
        };
        if left.trim().eq_ignore_ascii_case(name) {
            return Some(unquote_spice_token(right.trim()).to_string());
        }
    }
    None
}

fn split_spice_tokens(value: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut escaped = false;

    for character in value.chars() {
        if escaped {
            current.push(character);
            escaped = false;
            continue;
        }
        if character == '\\' {
            current.push(character);
            escaped = true;
            continue;
        }
        if character == '"' {
            current.push(character);
            in_quotes = !in_quotes;
            continue;
        }
        if character.is_ascii_whitespace() && !in_quotes {
            if !current.is_empty() {
                tokens.push(current.clone());
                current.clear();
            }
        } else {
            current.push(character);
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

fn unquote_spice_token(value: &str) -> &str {
    value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .unwrap_or(value)
}

pub(crate) fn symbol_instance_properties(
    definition: &KicadSymbolDef,
    reference: &str,
    value: &str,
    symbol_at: KicadAt,
) -> Vec<KicadProperty> {
    let mut properties = definition
        .properties
        .iter()
        .map(|property| KicadProperty {
            name: property.name.clone(),
            value: match property.name.as_str() {
                "Reference" => reference.to_string(),
                "Value" => value.to_string(),
                _ => property.value.clone(),
            },
            id: property.id,
            at: property
                .at
                .map(|property_at| transform_local_at(property_at, symbol_at, None)),
            hide: property.hide,
            show_name: property.show_name,
            do_not_autoplace: property.do_not_autoplace,
            effects: property.effects.clone(),
        })
        .collect::<Vec<_>>();

    if !properties
        .iter()
        .any(|property| property.name == "Reference")
    {
        properties.push(KicadProperty {
            name: "Reference".to_string(),
            value: reference.to_string(),
            id: None,
            at: Some(KicadAt {
                x: symbol_at.x,
                y: symbol_at.y - 2.54,
                rotation: symbol_at.rotation,
            }),
            hide: None,
            show_name: None,
            do_not_autoplace: None,
            effects: None,
        });
    }
    if !properties.iter().any(|property| property.name == "Value") {
        properties.push(KicadProperty {
            name: "Value".to_string(),
            value: value.to_string(),
            id: None,
            at: Some(KicadAt {
                x: symbol_at.x,
                y: symbol_at.y + 2.54,
                rotation: symbol_at.rotation,
            }),
            hide: None,
            show_name: None,
            do_not_autoplace: None,
            effects: None,
        });
    }

    properties
}
