pub mod token;

use crate::lexer::token::Token;
pub use logos::{Logos, Span};
use std::fmt::Formatter;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenPack {
    Keyword(Token),
    Operator(Token),
    Ident(String),
    Eof,
}

impl TokenPack {
    pub fn is_keyword(&self) -> bool {
        matches!(self, TokenPack::Keyword(_))
    }

    pub fn is_ident(&self) -> bool {
        matches!(self, TokenPack::Ident(_))
    }

    pub fn not_in_generic(&self) -> bool {
        matches!(self, TokenPack::Keyword(Token::Generic))
    }
}

impl std::fmt::Display for TokenPack {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenPack::Keyword(token) => write!(f, "{:?}", token),
            TokenPack::Ident(ident) => write!(f, "Ident({})", ident),
            TokenPack::Operator(token) => write!(f, "Operator({:?})", token),
            TokenPack::Eof => write!(f, "EOF"),
        }
    }
}
#[derive(Debug)]
pub struct TokenError {
    pub span: Span,
    pub context: String,
}

pub fn tokenize(source: &str) -> Result<Vec<TokenPack>, TokenError> {
    Token::lexer(source)
        .spanned()
        .map(|(token, span)| match token {
            Ok(token) => Ok(match token {
                token::Token::Ident => TokenPack::Ident(source[span.clone()].to_string()),
                _ => {
                    if token.is_operator() {
                        TokenPack::Operator(token)
                    }
                    else {
                        TokenPack::Keyword(token)
                    }
                }
            }),
            Err(_) => Err(TokenError {
                span: span.clone(),
                context: format!("{}", &source[span.clone()]),
            }),
        })
        .collect()
}
