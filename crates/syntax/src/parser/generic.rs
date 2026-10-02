use chumsky::{input::ValueInput, prelude::*};

use crate::lexer::{TokenPack, token::Token};

use super::{
    GenericRequirement, ParseError,
    func::interface_symbol_parser,
    out::{Span, Spanned},
    split::{keyword, operator, path, spanned_ident, visibility},
};

pub fn requirement_parser<'tokens, I>()
-> impl Parser<'tokens, I, Spanned<GenericRequirement>, extra::Err<ParseError<'tokens>>> + Clone
where
    I: ValueInput<'tokens, Token = TokenPack, Span = Span>,
{
    let interface = visibility()
        .then(interface_symbol_parser(false))
        .then_ignore(keyword(Token::Semicolon))
        .map(|(visibility, mut symbol)| {
            symbol.visibility = visibility;
            GenericRequirement::Interface(symbol)
        });

    let integer_argument = select! {
        TokenPack::Data { kind: Token::IntLiteral, text } => text,
    }
    .delimited_by(operator(Token::LAngle), operator(Token::RAngle));

    let type_requirement = path()
        .then(integer_argument.or_not())
        .then_ignore(keyword(Token::Semicolon))
        .map(|(path, argument)| GenericRequirement::Type { path, argument });

    choice((interface, type_requirement))
        .map_with(|requirement, extra| (requirement, extra.span()))
        .labelled("generic requirement")
        .boxed()
}

pub fn generic_parser<'tokens, I>() -> impl Parser<
    'tokens,
    I,
    (Spanned<String>, Vec<Spanned<GenericRequirement>>),
    extra::Err<ParseError<'tokens>>,
> + Clone
where
    I: ValueInput<'tokens, Token = TokenPack, Span = Span>,
{
    keyword(Token::Generic)
        .ignore_then(spanned_ident())
        .then(
            requirement_parser()
                .repeated()
                .collect::<Vec<_>>()
                .delimited_by(keyword(Token::LBrace), keyword(Token::RBrace)),
        )
        .then_ignore(keyword(Token::Semicolon).or_not())
        .labelled("generic declaration")
        .boxed()
}
