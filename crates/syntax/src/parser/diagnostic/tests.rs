use super::*;
use crate::lexer::{TokenPack, token::Token};

fn errors(source: &str) -> Vec<String> {
    let lexed = crate::lexer::tokenize(source).unwrap();
    let (_, errors) = crate::parser::parse(&lexed.tokens, source.len());
    assert!(!errors.is_empty());
    errors.iter().map(message).collect()
}

#[test]
fn keyword_and_operator_display_use_source_spelling() {
    for (token, spelling) in [
        (Token::ColonColon, "::"),
        (Token::RBrace, "}"),
        (Token::LAngle, "<"),
        (Token::Public, "pub"),
        (Token::Variable, "var"),
        (Token::AttributeStart, "#["),
        (Token::AttributeEnd, "]#"),
        (Token::VarList, "..."),
    ] {
        assert_eq!(TokenPack::KeyWord(token).to_string(), spelling);
    }
}

#[test]
fn enum_separator_error_preserves_useful_alternatives() {
    let diagnostic = errors("enum Color { Red Green }").join("\n");
    assert!(diagnostic.contains("`Green`"), "{diagnostic}");
    assert!(diagnostic.contains("`,`"), "{diagnostic}");
    assert!(diagnostic.contains("`}`"), "{diagnostic}");
    assert!(!diagnostic.contains("Comma"));
    assert!(!diagnostic.contains("RBrace"));
}

#[test]
fn long_expression_expectations_are_bounded_and_do_not_leak_token_names() {
    for diagnostic in errors("func main() { var c = Color:Red; }") {
        assert!(diagnostic.contains("`:`"), "{diagnostic}");
        assert!(diagnostic.len() < 220, "{diagnostic}");
        assert!(!diagnostic.contains("ColonColon"));
        assert!(!diagnostic.contains("Semicolon"));
        assert!(!diagnostic.contains("LogicalAnd"));
    }
}

#[test]
fn eof_and_lexical_errors_are_readable() {
    assert!(errors("enum Color {").join("\n").contains("end of input"));
    let error = crate::lexer::tokenize("\u{1b}").unwrap_err();
    assert_eq!(error.message(), "Unrecognized token `\\u{1b}`");
}
