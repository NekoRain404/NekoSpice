use crate::sexpr::{Sexp, child, list_value};
use std::env;
use std::path::{Path, PathBuf};

pub(crate) fn parse_kicad_footprint_filters(value: &str) -> Vec<String> {
    value
        .split_whitespace()
        .map(unescape_kicad_brace_string)
        .filter(|filter| !filter.is_empty())
        .collect()
}

pub(crate) fn case_insensitive_contains(value: &str, needle: &str) -> bool {
    value
        .to_ascii_lowercase()
        .contains(&needle.to_ascii_lowercase())
}

pub(crate) fn kicad_wildcard_match(pattern: &str, value: &str) -> bool {
    wildcard_match(
        pattern.to_ascii_lowercase().as_bytes(),
        value.to_ascii_lowercase().as_bytes(),
    )
}

fn wildcard_match(pattern: &[u8], value: &[u8]) -> bool {
    let (mut pattern_index, mut value_index) = (0, 0);
    let mut star_index = None;
    let mut star_value_index = 0;

    while value_index < value.len() {
        if pattern_index < pattern.len()
            && (pattern[pattern_index] == b'?' || pattern[pattern_index] == value[value_index])
        {
            pattern_index += 1;
            value_index += 1;
        } else if pattern_index < pattern.len() && pattern[pattern_index] == b'*' {
            star_index = Some(pattern_index);
            pattern_index += 1;
            star_value_index = value_index;
        } else if let Some(star) = star_index {
            pattern_index = star + 1;
            star_value_index += 1;
            value_index = star_value_index;
        } else {
            return false;
        }
    }

    while pattern_index < pattern.len() && pattern[pattern_index] == b'*' {
        pattern_index += 1;
    }

    pattern_index == pattern.len()
}

fn unescape_kicad_brace_string(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut characters = value.chars().peekable();
    while let Some(character) = characters.next() {
        if character != '{' {
            output.push(character);
            continue;
        }

        let mut token = String::new();
        let mut terminated = false;
        for token_character in characters.by_ref() {
            if token_character == '}' {
                terminated = true;
                break;
            }
            token.push(token_character);
        }

        if terminated {
            match token.as_str() {
                "dblquote" => output.push('"'),
                "quote" => output.push('\''),
                "lt" => output.push('<'),
                "gt" => output.push('>'),
                "backslash" => output.push('\\'),
                "slash" => output.push('/'),
                "bar" => output.push('|'),
                "comma" => output.push(','),
                "colon" => output.push(':'),
                "space" => output.push(' '),
                "dollar" => output.push('$'),
                "tab" => output.push('\t'),
                "return" => output.push('\n'),
                "brace" => output.push('{'),
                _ => {
                    output.push('{');
                    output.push_str(&unescape_kicad_brace_string(&token));
                    output.push('}');
                }
            }
        } else {
            output.push('{');
            output.push_str(&unescape_kicad_brace_string(&token));
        }
    }
    output
}

pub(crate) fn resolve_kicad_uri(uri: &str, base_dir: &Path) -> PathBuf {
    let base_dir = normalize_base_dir(base_dir);
    let expanded = expand_kicad_uri(uri, &base_dir);
    let path = PathBuf::from(expanded);
    if path.is_absolute() {
        path
    } else {
        base_dir.join(path)
    }
}

fn normalize_base_dir(base_dir: &Path) -> PathBuf {
    if base_dir.is_absolute() {
        base_dir.to_path_buf()
    } else {
        env::current_dir()
            .map(|cwd| cwd.join(base_dir))
            .unwrap_or_else(|_| base_dir.to_path_buf())
    }
}

fn expand_kicad_uri(uri: &str, base_dir: &Path) -> String {
    let mut expanded = String::new();
    let mut remaining = uri;

    while let Some(start) = remaining.find("${") {
        expanded.push_str(&remaining[..start]);
        let after_start = &remaining[start + 2..];
        let Some(end) = after_start.find('}') else {
            expanded.push_str(&remaining[start..]);
            return expanded;
        };

        let name = &after_start[..end];
        if name == "KIPRJMOD" {
            expanded.push_str(&base_dir.display().to_string());
        } else if let Ok(value) = env::var(name) {
            expanded.push_str(&value);
        } else {
            expanded.push_str("${");
            expanded.push_str(name);
            expanded.push('}');
        }
        remaining = &after_start[end + 1..];
    }

    expanded.push_str(remaining);
    expanded
}

pub(crate) fn parse_kicad_bool_value(value: String) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "yes" | "true" | "1" => Some(true),
        "no" | "false" | "0" => Some(false),
        _ => None,
    }
}

pub(crate) fn parse_optional_bool_child(items: &[Sexp], name: &str) -> Option<bool> {
    child(items, name).map(|node| {
        list_value(node, 1)
            .and_then(parse_kicad_bool_value)
            .unwrap_or(true)
    })
}
