use crate::Error;
use std::ops::Range;

#[derive(Debug)]
pub(super) struct Token {
    pub text: String,
    pub span: Range<usize>,
}

pub(super) fn lex(source: &str) -> Result<Vec<Token>, Error> {
    let bytes = source.as_bytes();
    let mut tokens = vec![];
    let mut cursor = 0;
    while cursor < bytes.len() {
        let start = cursor;
        if bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
            continue;
        }
        if source[cursor..].starts_with("//") {
            cursor = source[cursor..]
                .find(['\r', '\n'])
                .map_or(bytes.len(), |n| cursor + n);
            continue;
        }
        if source[cursor..].starts_with("/*") {
            cursor = source[cursor + 2..]
                .find("*/")
                .map(|n| cursor + n + 4)
                .ok_or_else(|| Error {
                    span: start..bytes.len(),
                    message: "Unterminated block comment".into(),
                })?;
            continue;
        }
        if bytes[cursor] == b'#' {
            return Err(Error {
                span: start..start + 1,
                message: "Preprocessing is not supported, including #include and header guards"
                    .into(),
            });
        }
        if source[cursor..].starts_with("...") {
            cursor += 3;
        } else if bytes[cursor].is_ascii_alphabetic() || bytes[cursor] == b'_' {
            cursor += 1;
            while cursor < bytes.len()
                && (bytes[cursor].is_ascii_alphanumeric() || bytes[cursor] == b'_')
            {
                cursor += 1;
            }
        } else if bytes[cursor].is_ascii_digit() {
            cursor += 1;
            while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
                cursor += 1;
            }
        } else if b"{}()[];,*".contains(&bytes[cursor]) {
            cursor += 1;
        } else {
            let end = start + source[start..].chars().next().unwrap().len_utf8();
            return Err(Error {
                span: start..end,
                message: "Unsupported C declaration token".into(),
            });
        }
        tokens.push(Token {
            text: source[start..cursor].into(),
            span: start..cursor,
        });
    }
    Ok(tokens)
}
