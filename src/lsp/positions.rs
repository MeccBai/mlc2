use serde_json::{Value, json};
use std::path::{Path, PathBuf};
pub fn path(uri: &str) -> Option<PathBuf> {
    let raw = uri.strip_prefix("file://")?;
    let raw = if cfg!(windows) {
        raw.strip_prefix('/').unwrap_or(raw)
    } else {
        raw
    };
    let mut bytes = vec![];
    let mut i = 0;
    while i < raw.len() {
        if raw.as_bytes()[i] == b'%' {
            bytes.push(u8::from_str_radix(raw.get(i + 1..i + 3)?, 16).ok()?);
            i += 3;
        } else {
            bytes.push(raw.as_bytes()[i]);
            i += 1;
        }
    }
    Some(PathBuf::from(String::from_utf8(bytes).ok()?))
}
pub fn uri(path: &Path) -> String {
    let text = path.to_string_lossy().replace('\\', "/");
    let text = text.strip_prefix("//?/").unwrap_or(&text);
    let mut result = if text.starts_with('/') {
        "file://".to_string()
    } else {
        "file:///".to_string()
    };
    for b in text.bytes() {
        if b.is_ascii_alphanumeric() || b"/:._-~".contains(&b) {
            result.push(b as char);
        } else {
            result.push_str(&format!("%{b:02X}"));
        }
    }
    result
}
pub fn position(text: &str, byte: usize) -> Value {
    let mut end = byte.min(text.len());
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    let prefix = &text[..end];
    let start = prefix.rfind('\n').map_or(0, |n| n + 1);
    json!({"line":prefix.bytes().filter(|b|*b==b'\n').count(),"character":text[start..end].encode_utf16().count()})
}
pub fn offset(text: &str, p: &Value) -> Option<usize> {
    let line = p["line"].as_u64()? as usize;
    let character = p["character"].as_u64()? as usize;
    let start = if line == 0 {
        0
    } else {
        text.match_indices('\n').nth(line - 1)?.0 + 1
    };
    let mut units = 0;
    for (i, c) in text[start..].char_indices() {
        if units == character {
            return Some(start + i);
        }
        if c == '\n' || c == '\r' {
            return None;
        }
        units += c.len_utf16();
        if units > character {
            return None;
        }
    }
    (units == character).then_some(text.len())
}
pub fn range(text: &str, start: usize, end: usize) -> Value {
    json!({"start":position(text,start),"end":position(text,end)})
}
