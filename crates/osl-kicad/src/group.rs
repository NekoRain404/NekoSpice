use crate::sexpr::{Sexp, atom_text, child, child_value, list_items, list_value, sexpr_string};
use crate::util::parse_kicad_bool_value;

#[derive(Debug, Clone, PartialEq)]
pub struct KicadGroup {
    pub name: String,
    pub uuid: Option<String>,
    pub locked: Option<bool>,
    pub members: Vec<String>,
}

impl KicadGroup {
    pub(crate) fn write_group_sexpr(&self, output: &mut String, indent: usize) {
        let pad = " ".repeat(indent);
        output.push_str(&format!("{}(group {}\n", pad, sexpr_string(&self.name)));
        if let Some(uuid) = &self.uuid {
            output.push_str(&format!("{}  (uuid {})\n", pad, sexpr_string(uuid)));
        }
        if self.locked == Some(true) {
            output.push_str(&format!("{}  (locked yes)\n", pad));
        }
        output.push_str(&format!("{}  (members", pad));
        for member in &self.members {
            output.push_str(&format!(" {}", sexpr_string(member)));
        }
        output.push_str(")\n");
        output.push_str(&format!("{})\n", pad));
    }
}

pub(crate) fn parse_group(node: &Sexp) -> Option<KicadGroup> {
    let items = list_items(node);
    Some(KicadGroup {
        name: list_value(node, 1)?,
        uuid: child_value(items, "uuid"),
        locked: child_value(items, "locked").and_then(parse_kicad_bool_value),
        members: child(items, "members")
            .map(|members| {
                list_items(members)
                    .iter()
                    .skip(1)
                    .filter_map(atom_text)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default(),
    })
}
