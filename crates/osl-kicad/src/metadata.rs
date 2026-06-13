//! Schematic metadata — title block parsing and serialization.

use crate::sexpr::{Sexp, child_value, direct_children, list_items, list_value, sexpr_string};

#[derive(Debug, Clone, PartialEq)]
pub struct KicadTitleBlock {
    pub title: Option<String>,
    pub date: Option<String>,
    pub revision: Option<String>,
    pub company: Option<String>,
    pub comments: Vec<KicadTitleComment>,
}

impl KicadTitleBlock {
    /// write title block sexpr。
    pub(crate) fn write_title_block_sexpr(&self, output: &mut String, indent: usize) {
        let pad = " ".repeat(indent);
        output.push_str(&format!("{}(title_block\n", pad));
        if let Some(title) = &self.title {
            output.push_str(&format!("{}  (title {})\n", pad, sexpr_string(title)));
        }
        if let Some(date) = &self.date {
            output.push_str(&format!("{}  (date {})\n", pad, sexpr_string(date)));
        }
        if let Some(revision) = &self.revision {
            output.push_str(&format!("{}  (rev {})\n", pad, sexpr_string(revision)));
        }
        if let Some(company) = &self.company {
            output.push_str(&format!("{}  (company {})\n", pad, sexpr_string(company)));
        }
        for comment in &self.comments {
            output.push_str(&format!(
                "{}  (comment {} {})\n",
                pad,
                comment.index,
                sexpr_string(&comment.text)
            ));
        }
        output.push_str(&format!("{})\n", pad));
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadTitleComment {
    pub index: u32,
    pub text: String,
}

/// parse title block。
pub(crate) fn parse_title_block(node: &Sexp) -> KicadTitleBlock {
    let items = list_items(node);
    KicadTitleBlock {
        title: child_value(items, "title"),
        date: child_value(items, "date"),
        revision: child_value(items, "rev"),
        company: child_value(items, "company"),
        comments: direct_children(items, "comment")
            .filter_map(parse_title_comment)
            .collect(),
    }
}

fn parse_title_comment(node: &Sexp) -> Option<KicadTitleComment> {
    Some(KicadTitleComment {
        index: list_value(node, 1)?.parse().ok()?,
        text: list_value(node, 2)?,
    })
}
