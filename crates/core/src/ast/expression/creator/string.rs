/// Decode once, before type inference counts the actual UTF-8 bytes.
pub(super) fn decode(text: &str) -> Option<String> {
    let mut chars = text.strip_prefix('"')?.strip_suffix('"')?.chars();
    let mut value = String::new();
    while let Some(character) = chars.next() {
        value.push(if character == '\\' {
            match chars.next()? {
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                '0' => '\0',
                '\\' => '\\',
                '"' => '"',
                _ => return None,
            }
        } else {
            character
        });
    }
    Some(value)
}

#[cfg(test)]
mod tests {
    use super::decode;
    #[test]
    fn quotes_escapes_and_utf8() {
        assert_eq!(
            decode(r#""中\n\t\r\0\\\"""#).as_deref(),
            Some("中\n\t\r\0\\\"")
        );
        assert_eq!(decode("\"\""), Some(String::new()));
        assert_eq!(decode(r#""\q""#), None);
    }
}
