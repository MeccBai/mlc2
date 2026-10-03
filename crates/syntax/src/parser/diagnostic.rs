//! Render structured parser errors without Debug token names or unbounded lists.
use super::ParseError;
use chumsky::error::{RichPattern, RichReason};

const MAX_EXPECTED: usize = 3;

fn quote(text: &str) -> String {
    format!("`{}`", text.escape_debug())
}

pub fn message(error: &ParseError<'_>) -> String {
    let RichReason::ExpectedFound { expected, .. } = error.reason() else {
        let RichReason::Custom(message) = error.reason() else {
            unreachable!()
        };
        return format!("Syntax error: {message}");
    };
    let found = error
        .found()
        .map(|token| quote(&token.to_string()))
        .unwrap_or_else(|| "end of input".into());
    let mut alternatives: Vec<_> = expected
        .iter()
        .map(|pattern| match pattern {
            RichPattern::Token(token) => quote(&token.to_string()),
            RichPattern::Identifier(name) => quote(name),
            RichPattern::Label(label) => label.to_string(),
            RichPattern::EndOfInput => "end of input".into(),
            RichPattern::Any => "a token".into(),
            RichPattern::SomethingElse => "a different token".into(),
            _ => "valid syntax".into(),
        })
        .collect();
    alternatives.sort();
    alternatives.dedup();
    if alternatives.is_empty() {
        return format!("Syntax error: unexpected {found}");
    }
    let expected = if alternatives.len() > MAX_EXPECTED {
        // Do not pretend the omitted alternatives are invalid or infer a rule
        // from a partial parse. The full structured error remains available.
        format!(
            "{} or another valid continuation",
            alternatives[..MAX_EXPECTED].join(", ")
        )
    } else if alternatives.len() == 1 {
        alternatives.remove(0)
    } else {
        let last = alternatives.pop().unwrap();
        format!("{} or {last}", alternatives.join(", "))
    };
    format!("Syntax error: found {found}; expected {expected}")
}

#[cfg(test)]
mod tests;
