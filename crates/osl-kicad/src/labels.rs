use crate::coordinates::{KicadAt, parse_at};
use crate::property::{KicadProperty, parse_property};
use crate::sexpr::{
    Sexp, child, child_value, direct_children, format_number, list_items, list_value,
    sexpr_atom_or_string, sexpr_string,
};
use crate::style::{KicadTextEffects, parse_text_effects, write_inline_text_effects};
use crate::util::parse_optional_bool_child;

#[derive(Debug, Clone, PartialEq)]
pub struct KicadLabel {
    pub text: String,
    pub kind: KicadLabelKind,
    pub at: Option<KicadAt>,
    pub uuid: Option<String>,
    pub shape: Option<String>,
    pub fields_autoplaced: Option<bool>,
    pub effects: Option<KicadTextEffects>,
    pub properties: Vec<KicadProperty>,
}

impl KicadLabel {
    /// write label sexpr。
    pub(crate) fn write_label_sexpr(&self, output: &mut String, indent: usize) {
        let pad = " ".repeat(indent);
        output.push_str(&format!(
            "{}({} {}",
            pad,
            self.kind.sexpr_name(),
            sexpr_string(&self.text)
        ));
        if let Some(shape) = &self.shape {
            output.push_str(&format!(" (shape {})", sexpr_atom_or_string(shape)));
        }
        if let Some(at) = self.at {
            output.push_str(&format!(
                " (at {} {} {})",
                format_number(at.x),
                format_number(at.y),
                format_number(at.rotation)
            ));
        }
        if let Some(fields_autoplaced) = self.fields_autoplaced {
            output.push_str(&format!(
                " (fields_autoplaced {})",
                if fields_autoplaced { "yes" } else { "no" }
            ));
        }
        write_inline_text_effects(output, self.effects.as_ref());
        if let Some(uuid) = &self.uuid {
            output.push_str(&format!(" (uuid {})", sexpr_string(uuid)));
        }
        if self.properties.is_empty() {
            output.push_str(")\n");
        } else {
            output.push('\n');
            for property in &self.properties {
                property.write_property_sexpr(output, indent + 2);
            }
            output.push_str(&format!("{})\n", pad));
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadDirectiveLabel {
    pub text: String,
    pub length: Option<f64>,
    pub shape: Option<String>,
    pub at: Option<KicadAt>,
    pub fields_autoplaced: Option<bool>,
    pub effects: Option<KicadTextEffects>,
    pub uuid: Option<String>,
    pub properties: Vec<KicadProperty>,
}

impl KicadDirectiveLabel {
    /// property。
    pub fn property(&self, name: &str) -> Option<&str> {
        self.properties
            .iter()
            .find(|property| property.name == name)
            .map(|property| property.value.as_str())
    }

    /// display text。
    pub fn display_text(&self) -> &str {
        ["Netclass", "Net Class", "Component Class"]
            .into_iter()
            .find_map(|name| self.property(name).filter(|value| !value.is_empty()))
            .unwrap_or(&self.text)
    }

    /// write directive label sexpr。
    pub(crate) fn write_directive_label_sexpr(&self, output: &mut String, indent: usize) {
        let pad = " ".repeat(indent);
        output.push_str(&format!(
            "{}(netclass_flag {}",
            pad,
            sexpr_string(&self.text)
        ));
        if let Some(length) = self.length {
            output.push_str(&format!(" (length {})", format_number(length)));
        }
        if let Some(shape) = &self.shape {
            output.push_str(&format!(" (shape {})", sexpr_atom_or_string(shape)));
        }
        if let Some(at) = self.at {
            output.push_str(&format!(
                " (at {} {} {})",
                format_number(at.x),
                format_number(at.y),
                format_number(at.rotation)
            ));
        }
        if let Some(fields_autoplaced) = self.fields_autoplaced {
            output.push_str(&format!(
                " (fields_autoplaced {})",
                if fields_autoplaced { "yes" } else { "no" }
            ));
        }
        write_inline_text_effects(output, self.effects.as_ref());
        if let Some(uuid) = &self.uuid {
            output.push_str(&format!(" (uuid {})", sexpr_string(uuid)));
        }
        if self.properties.is_empty() {
            output.push_str(")\n");
        } else {
            output.push('\n');
            for property in &self.properties {
                property.write_property_sexpr(output, indent + 2);
            }
            output.push_str(&format!("{})\n", pad));
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KicadLabelKind {
    Local,
    Global,
    Hierarchical,
}

impl KicadLabelKind {
    /// as str。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Global => "global",
            Self::Hierarchical => "hierarchical",
        }
    }

    /// sexpr name。
    pub(crate) fn sexpr_name(self) -> &'static str {
        match self {
            Self::Local => "label",
            Self::Global => "global_label",
            Self::Hierarchical => "hierarchical_label",
        }
    }
}

/// parse label。
pub(crate) fn parse_label(node: &Sexp, kind: KicadLabelKind) -> Option<KicadLabel> {
    let items = list_items(node);
    Some(KicadLabel {
        text: list_value(node, 1)?,
        kind,
        at: child(items, "at").and_then(parse_at),
        uuid: child_value(items, "uuid"),
        shape: child_value(items, "shape"),
        fields_autoplaced: parse_optional_bool_child(items, "fields_autoplaced"),
        effects: child(items, "effects").map(parse_text_effects),
        properties: direct_children(items, "property")
            .filter_map(parse_property)
            .collect(),
    })
}

/// parse directive label。
pub(crate) fn parse_directive_label(node: &Sexp) -> Option<KicadDirectiveLabel> {
    let items = list_items(node);
    Some(KicadDirectiveLabel {
        text: list_value(node, 1).unwrap_or_default(),
        length: child_value(items, "length").and_then(|value| value.parse().ok()),
        shape: child_value(items, "shape"),
        at: child(items, "at").and_then(parse_at),
        fields_autoplaced: parse_optional_bool_child(items, "fields_autoplaced"),
        effects: child(items, "effects").map(parse_text_effects),
        uuid: child_value(items, "uuid"),
        properties: direct_children(items, "property")
            .filter_map(parse_property)
            .collect(),
    })
}
