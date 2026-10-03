pub mod token;

use chumsky::span::SimpleSpan;
use std::{fmt, ops::Range};

use crate::lexer::token::Token;
pub use logos::Logos;

pub type Span = SimpleSpan<usize>;
pub type SpannedToken = (TokenPack, Span);

#[derive(Debug, Clone, PartialEq)]
pub struct Comment {
    pub text: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Lexed {
    pub tokens: Vec<SpannedToken>,
    pub comments: Vec<Comment>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenPack {
    KeyWord(Token),
    Operator(Token),
    Ident(String),
    Data { kind: Token, text: String },
}

impl fmt::Display for TokenPack {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::KeyWord(token) | Self::Operator(token) => write!(f, "{token}"),
            Self::Ident(ident) => write!(f, "{ident}"),
            Self::Data { text, .. } => write!(f, "{text}"),
        }
    }
}

#[derive(Debug)]
pub struct TokenError {
    pub span: Range<usize>,
    pub context: String,
}

impl TokenError {
    pub fn message(&self) -> String {
        format!("Unrecognized token `{}`", self.context.escape_debug())
    }
}

pub fn tokenize(source: &str) -> Result<Lexed, TokenError> {
    let mut tokens = Vec::new();
    let mut comments = Vec::new();

    for (token, range) in Token::lexer(source).spanned() {
        let token = token.map_err(|_| TokenError {
            span: range.clone(),
            context: source[range.clone()].to_owned(),
        })?;

        if token.is_comment() {
            comments.push(Comment {
                text: source[range.clone()].to_owned(),
                span: range.into(),
            });
            continue;
        }

        let packed = match token {
            Token::Ident => TokenPack::Ident(source[range.clone()].to_owned()),
            token if token.is_operator() => TokenPack::Operator(token),
            token if token.is_data() => TokenPack::Data {
                kind: token,
                text: source[range.clone()].to_owned(),
            },
            token => TokenPack::KeyWord(token),
        };

        tokens.push((packed, range.into()));
    }

    Ok(Lexed { tokens, comments })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_line_endings_preserve_original_offsets() {
        let source = "func main() {\r\n    // comment\r\n    var text = \"hello\";\r\n}\r\n";
        let lexed = tokenize(source).unwrap();
        let variable = lexed
            .tokens
            .iter()
            .find(|(token, _)| matches!(token, TokenPack::KeyWord(Token::Variable)))
            .unwrap();
        assert_eq!(variable.1.start, source.find("var").unwrap());
        assert_eq!(&source[variable.1.into_range()], "var");
        let (module, errors) = crate::parser::parse(&lexed.tokens, source.len());
        assert!(errors.is_empty(), "{errors:?}");
        assert!(module.is_some());
        assert_eq!(lexed.comments.len(), 1);
        assert_eq!(lexed.comments[0].text, "// comment");
    }

    #[test]
    fn retains_comment_text_and_source_span_outside_the_parser_stream() {
        let source = "var a = 1; // keep me\n/* and me */ var b = 2;";
        let lexed = tokenize(source).unwrap();

        assert_eq!(lexed.comments.len(), 2);
        assert_eq!(lexed.comments[0].text, "// keep me");
        assert_eq!(lexed.comments[1].text, "/* and me */");
        for comment in &lexed.comments {
            assert_eq!(&source[comment.span.into_range()], comment.text);
        }
        assert!(lexed.tokens.iter().all(|(token, _)| !matches!(
            token,
            TokenPack::KeyWord(Token::SingleLineComment | Token::MultiLineComment)
        )));
    }
}
