use crate::coordinates::{
    KicadPoint, KicadSize, parse_at, parse_points, parse_size, write_points_sexpr,
};
use crate::sexpr::{
    Sexp, atom_text, child, child_value, format_number, head, list_items, list_value, sexpr_string,
    write_sexpr_inline,
};
use crate::style::{KicadColor, KicadStroke, parse_color, parse_stroke, write_inline_stroke};

#[derive(Debug, Clone, PartialEq)]
pub struct KicadWire {
    pub points: Vec<KicadPoint>,
    pub stroke: Option<KicadStroke>,
    pub uuid: Option<String>,
}

impl KicadWire {
    /// write wire sexpr。
    pub(crate) fn write_wire_sexpr(&self, output: &mut String, indent: usize) {
        let pad = " ".repeat(indent);
        output.push_str(&format!("{}(wire", pad));
        write_points_sexpr(output, &self.points);
        write_inline_stroke(output, self.stroke.as_ref(), 0.0);
        if let Some(uuid) = &self.uuid {
            output.push_str(&format!(" (uuid {})", sexpr_string(uuid)));
        }
        output.push_str(")\n");
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadBusAlias {
    pub name: String,
    pub members: Vec<String>,
}

impl KicadBusAlias {
    /// write bus alias sexpr。
    pub(crate) fn write_bus_alias_sexpr(&self, output: &mut String, indent: usize) {
        let pad = " ".repeat(indent);
        let members = self
            .members
            .iter()
            .map(|member| sexpr_string(member))
            .collect::<Vec<_>>()
            .join(" ");
        output.push_str(&format!(
            "{}(bus_alias {} (members {}))\n",
            pad,
            sexpr_string(&self.name),
            members
        ));
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadBus {
    pub points: Vec<KicadPoint>,
    pub stroke: Option<KicadStroke>,
    pub uuid: Option<String>,
}

impl KicadBus {
    /// write bus sexpr。
    pub(crate) fn write_bus_sexpr(&self, output: &mut String, indent: usize) {
        let pad = " ".repeat(indent);
        output.push_str(&format!("{}(bus", pad));
        write_points_sexpr(output, &self.points);
        write_inline_stroke(output, self.stroke.as_ref(), 0.0);
        if let Some(uuid) = &self.uuid {
            output.push_str(&format!(" (uuid {})", sexpr_string(uuid)));
        }
        output.push_str(")\n");
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadBusEntry {
    pub at: KicadPoint,
    pub size: KicadSize,
    pub stroke: Option<KicadStroke>,
    pub uuid: Option<String>,
}

impl KicadBusEntry {
    /// end。
    pub fn end(&self) -> KicadPoint {
        KicadPoint {
            x: self.at.x + self.size.width,
            y: self.at.y + self.size.height,
        }
    }

    /// write bus entry sexpr。
    pub(crate) fn write_bus_entry_sexpr(&self, output: &mut String, indent: usize) {
        let pad = " ".repeat(indent);
        output.push_str(&format!(
            "{}(bus_entry\n{}  (at {} {})\n{}  (size {} {})\n",
            pad,
            pad,
            format_number(self.at.x),
            format_number(self.at.y),
            pad,
            format_number(self.size.width),
            format_number(self.size.height)
        ));
        output.push_str(&format!("{}  ", pad));
        write_inline_stroke(output, self.stroke.as_ref(), 0.0);
        output.push('\n');
        if let Some(uuid) = &self.uuid {
            output.push_str(&format!("{}  (uuid {})\n", pad, sexpr_string(uuid)));
        }
        output.push_str(&format!("{})\n", pad));
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadNetChain {
    pub name: String,
    pub from: Option<KicadNetChainEndpoint>,
    pub to: Option<KicadNetChainEndpoint>,
    pub net_class: Option<String>,
    pub color: Option<KicadColor>,
    pub member_nets: Vec<String>,
    pub extra: Vec<Sexp>,
}

impl KicadNetChain {
    /// write net chain sexpr。
    pub(crate) fn write_net_chain_sexpr(&self, output: &mut String, indent: usize) {
        let pad = " ".repeat(indent);
        output.push_str(&format!("{}(net_chain {}", pad, sexpr_string(&self.name)));
        if let Some(from) = &self.from {
            output.push_str(&format!(
                " (from {} {})",
                sexpr_string(&from.reference),
                sexpr_string(&from.pin)
            ));
        }
        if let Some(to) = &self.to {
            output.push_str(&format!(
                " (to {} {})",
                sexpr_string(&to.reference),
                sexpr_string(&to.pin)
            ));
        }
        if let Some(net_class) = &self.net_class {
            output.push_str(&format!(" (net_class {})", sexpr_string(net_class)));
        }
        if let Some(color) = self.color {
            color.write_inline_color_sexpr(output);
        }
        if !self.member_nets.is_empty() {
            output.push_str(" (nets");
            for net in &self.member_nets {
                output.push_str(&format!(" {}", sexpr_string(net)));
            }
            output.push(')');
        }
        for item in &self.extra {
            output.push(' ');
            write_sexpr_inline(output, item);
        }
        output.push_str(")\n");
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadNetChainEndpoint {
    pub reference: String,
    pub pin: String,
}

/// parse wire。
pub(crate) fn parse_wire(node: &Sexp) -> KicadWire {
    let items = list_items(node);
    KicadWire {
        points: child(items, "pts").map(parse_points).unwrap_or_default(),
        stroke: child(items, "stroke").map(parse_stroke),
        uuid: child_value(items, "uuid"),
    }
}

/// parse bus alias。
pub(crate) fn parse_bus_alias(node: &Sexp) -> Option<KicadBusAlias> {
    let items = list_items(node);
    Some(KicadBusAlias {
        name: list_value(node, 1)?,
        members: child(items, "members")
            .map(|members| {
                list_items(members)
                    .iter()
                    .skip(1)
                    .filter_map(atom_text)
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default(),
    })
}

/// parse bus。
pub(crate) fn parse_bus(node: &Sexp) -> KicadBus {
    let items = list_items(node);
    KicadBus {
        points: child(items, "pts").map(parse_points).unwrap_or_default(),
        stroke: child(items, "stroke").map(parse_stroke),
        uuid: child_value(items, "uuid"),
    }
}

/// parse bus entry。
pub(crate) fn parse_bus_entry(node: &Sexp) -> Option<KicadBusEntry> {
    let items = list_items(node);
    let at = child(items, "at").and_then(parse_at)?;
    Some(KicadBusEntry {
        at: KicadPoint { x: at.x, y: at.y },
        size: child(items, "size").and_then(parse_size)?,
        stroke: child(items, "stroke").map(parse_stroke),
        uuid: child_value(items, "uuid"),
    })
}

/// parse net chain。
pub(crate) fn parse_net_chain(node: &Sexp) -> Option<KicadNetChain> {
    let items = list_items(node);
    let known_heads = ["from", "to", "net_class", "color", "nets"];
    Some(KicadNetChain {
        name: list_value(node, 1)?,
        from: child(items, "from").and_then(parse_net_chain_endpoint),
        to: child(items, "to").and_then(parse_net_chain_endpoint),
        net_class: child_value(items, "net_class"),
        color: child(items, "color").and_then(parse_color),
        member_nets: child(items, "nets")
            .map(|nets| {
                list_items(nets)
                    .iter()
                    .skip(1)
                    .filter_map(atom_text)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default(),
        extra: list_items(node)
            .iter()
            .skip(2)
            .filter(|item| {
                matches!(item, Sexp::List(_))
                    && head(item).is_none_or(|head| !known_heads.contains(&head))
            })
            .cloned()
            .collect(),
    })
}

fn parse_net_chain_endpoint(node: &Sexp) -> Option<KicadNetChainEndpoint> {
    Some(KicadNetChainEndpoint {
        reference: list_value(node, 1)?,
        pin: list_value(node, 2)?,
    })
}
