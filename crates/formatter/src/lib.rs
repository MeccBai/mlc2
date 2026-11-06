//! Fixed-style, syntax-only formatting with import ordering. No symbol resolution.
mod error;
mod hints;
mod imports;
mod layout;
mod render;
mod separation;

pub use error::{FormatError, FormatIssue};
use mlc_syntax::{lexer, parser};

/// Format a complete translation unit. Invalid input never produces partial output.
pub fn format(source: &str) -> Result<String, FormatError> {
    let lexed = lexer::tokenize(source).map_err(|error| {
        FormatError::Lexical(FormatIssue {
            span: error.span.clone(),
            message: error.message(),
        })
    })?;
    let (module, errors) = parser::parse(&lexed.tokens, source.len());
    if !errors.is_empty() || module.is_none() {
        return Err(FormatError::Syntax(
            errors
                .iter()
                .map(|error| FormatIssue {
                    span: error.span().into_range(),
                    message: parser::diagnostic::message(error),
                })
                .collect(),
        ));
    }
    let module = module.ok_or_else(|| FormatError::Syntax(vec![]))?;
    if let Some(sorted) = imports::sorted(source, &lexed, &module) {
        return format(&sorted);
    }
    let output = render::render(source, &lexed, &module);
    // After import ordering, rendering may change whitespace only. Guard against merging
    // operators, swallowing code into a line comment, or dropping punctuation.
    let checked = lexer::tokenize(&output).map_err(|_| FormatError::InvalidOutput)?;
    if !lexed
        .tokens
        .iter()
        .map(|(token, _)| token)
        .eq(checked.tokens.iter().map(|(token, _)| token))
        || !lexed
            .comments
            .iter()
            .map(|comment| &comment.text)
            .eq(checked.comments.iter().map(|comment| &comment.text))
    {
        return Err(FormatError::InvalidOutput);
    }
    Ok(output)
}

#[cfg(test)]
mod tests;
