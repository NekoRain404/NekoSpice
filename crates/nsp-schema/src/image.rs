//! schema image element parsing and serialization.

use crate::coordinates::parse_image_at;
use crate::sexpr::{Sexp, atom_text, child, child_value, format_number, list_items, sexpr_string};
use crate::util::parse_bool_value;
use crate::{NspBoundingBox, NspPoint, NspSize};

#[derive(Debug, Clone, PartialEq)]
pub struct NspImage {
    pub at: Option<NspPoint>,
    pub scale: f64,
    pub data_base64: String,
    pub uuid: Option<String>,
    pub locked: Option<bool>,
}

impl NspImage {
    /// image size mm。
    pub fn image_size_mm(&self) -> Option<NspSize> {
        png_size_from_base64(&self.data_base64).map(|(width_px, height_px)| {
            let scale = if self.scale.is_finite() && self.scale > 0.0 {
                self.scale
            } else {
                1.0
            };
            NspSize {
                width: f64::from(width_px) / 300.0 * 25.4 * scale,
                height: f64::from(height_px) / 300.0 * 25.4 * scale,
            }
        })
    }

    /// bounding box。
    pub fn bounding_box(&self) -> Option<NspBoundingBox> {
        let at = self.at?;
        let size = self.image_size_mm()?;
        Some(NspBoundingBox {
            min: NspPoint {
                x: at.x - size.width / 2.0,
                y: at.y - size.height / 2.0,
            },
            max: NspPoint {
                x: at.x + size.width / 2.0,
                y: at.y + size.height / 2.0,
            },
        })
    }

    /// mime type。
    pub fn mime_type(&self) -> &'static str {
        if base64_starts_with(&self.data_base64, b"\x89PNG\r\n\x1a\n") {
            "image/png"
        } else if base64_starts_with(&self.data_base64, b"\xff\xd8\xff") {
            "image/jpeg"
        } else {
            "application/octet-stream"
        }
    }

    /// write image sexpr。
    pub(crate) fn write_image_sexpr(&self, output: &mut String, indent: usize) {
        let pad = " ".repeat(indent);
        output.push_str(&format!("{}(image", pad));
        if let Some(at) = self.at {
            output.push_str(&format!(
                " (at {} {})",
                format_number(at.x),
                format_number(at.y)
            ));
        }
        if self.scale != 1.0 {
            output.push_str(&format!(" (scale {})", format_number(self.scale)));
        }
        if let Some(uuid) = &self.uuid {
            output.push_str(&format!(" (uuid {})", sexpr_string(uuid)));
        }
        if self.locked == Some(true) {
            output.push_str(" (locked yes)");
        }
        output.push('\n');
        write_base64_data_sexpr(output, &self.data_base64, indent + 2);
        output.push_str(&format!("{})\n", pad));
    }
}

/// parse image。
pub(crate) fn parse_image(node: &Sexp) -> Option<NspImage> {
    let items = list_items(node);
    Some(NspImage {
        at: child(items, "at").and_then(parse_image_at),
        scale: child_value(items, "scale")
            .and_then(|value| value.parse().ok())
            .filter(|scale: &f64| scale.is_finite() && *scale > 0.0)
            .unwrap_or(1.0),
        data_base64: child(items, "data").map(parse_data_chunks)?,
        uuid: child_value(items, "uuid"),
        locked: child_value(items, "locked").and_then(parse_bool_value),
    })
}

fn parse_data_chunks(node: &Sexp) -> String {
    list_items(node)
        .iter()
        .skip(1)
        .filter_map(atom_text)
        .collect::<String>()
}

fn write_base64_data_sexpr(output: &mut String, data: &str, indent: usize) {
    let pad = " ".repeat(indent);
    output.push_str(&format!("{}(data", pad));
    let mut wrote_chunk = false;
    for chunk in data.as_bytes().chunks(76) {
        wrote_chunk = true;
        output.push_str(&format!(
            "\n{}  {}",
            pad,
            sexpr_string(std::str::from_utf8(chunk).unwrap_or_default())
        ));
    }
    if wrote_chunk {
        output.push('\n');
        output.push_str(&pad);
    }
    output.push_str(")\n");
}

fn png_size_from_base64(data: &str) -> Option<(u32, u32)> {
    let header = decode_base64_prefix(data, 24)?;
    if header.len() < 24 || &header[0..8] != b"\x89PNG\r\n\x1a\n" || &header[12..16] != b"IHDR" {
        return None;
    }
    let width = u32::from_be_bytes([header[16], header[17], header[18], header[19]]);
    let height = u32::from_be_bytes([header[20], header[21], header[22], header[23]]);
    (width > 0 && height > 0).then_some((width, height))
}

fn base64_starts_with(data: &str, prefix: &[u8]) -> bool {
    decode_base64_prefix(data, prefix.len())
        .map(|decoded| decoded.starts_with(prefix))
        .unwrap_or(false)
}

fn decode_base64_prefix(data: &str, wanted_len: usize) -> Option<Vec<u8>> {
    let mut decoded = Vec::with_capacity(wanted_len);
    let mut buffer = [0_u8; 4];
    let mut buffer_len = 0;

    for byte in data.bytes().filter(|byte| !byte.is_ascii_whitespace()) {
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' => 64,
            _ => return None,
        };
        buffer[buffer_len] = value;
        buffer_len += 1;

        if buffer_len == 4 {
            decoded.push((buffer[0] << 2) | (buffer[1] >> 4));
            if buffer[2] != 64 {
                decoded.push((buffer[1] << 4) | (buffer[2] >> 2));
            }
            if buffer[3] != 64 {
                decoded.push((buffer[2] << 6) | buffer[3]);
            }
            if decoded.len() >= wanted_len {
                decoded.truncate(wanted_len);
                return Some(decoded);
            }
            if buffer[2] == 64 || buffer[3] == 64 {
                break;
            }
            buffer_len = 0;
        }
    }

    (decoded.len() >= wanted_len).then_some(decoded)
}
