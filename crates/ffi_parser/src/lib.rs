//! Strict C declaration subset, without preprocessing or host-dependent inference.
mod lexer;
mod parser;
mod types;
mod validate;
use mlc_syntax::parser::out::TempModule;
use std::{fmt, ops::Range};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub span: Range<usize>,
    pub message: String,
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at {:?}", self.message, self.span)
    }
}
impl std::error::Error for Error {}

pub fn parse(source: &str, target: &str) -> Result<TempModule, Error> {
    let model = types::DataModel::new(target).ok_or_else(|| Error {
        span: 0..0,
        message: format!("Unsupported C target: {target}"),
    })?;
    let module = parser::Parser::new(lexer::lex(source)?, model, source.len()).module()?;
    validate::module(&module)?;
    Ok(module)
}

#[cfg(test)]
mod tests;
