use chumsky::{input::ValueInput, prelude::*};

use crate::lexer::{Span, TokenPack, token::Token};

use super::{
    GenericRequirement, ParseError,
    func::function_parser,
    split::{ident, keyword, operator, path, visibility},
};

pub fn requirement_parser<'tokens, I>()
-> impl Parser<'tokens, I, GenericRequirement, extra::Err<ParseError<'tokens>>> + Clone
where
    I: ValueInput<'tokens, Token = TokenPack, Span = Span>,
{
    let function = visibility()
        .then(function_parser())
        .map(|(visibility, mut function)| {
            function.symbol.visibility = visibility;
            GenericRequirement::Function(function)
        });

    let integer_argument = select! {
        TokenPack::Data { kind: Token::IntLiteral, text } => text,
    }
    .delimited_by(operator(Token::LAngle), operator(Token::RAngle));

    let type_requirement = path()
        .then(integer_argument.or_not())
        .then_ignore(keyword(Token::Semicolon))
        .map(|(path, argument)| GenericRequirement::Type { path, argument });

    choice((function, type_requirement))
        .labelled("generic requirement")
        .boxed()
}

pub fn generic_parser<'tokens, I>()
-> impl Parser<'tokens, I, (String, Vec<GenericRequirement>), extra::Err<ParseError<'tokens>>> + Clone
where
    I: ValueInput<'tokens, Token = TokenPack, Span = Span>,
{
    keyword(Token::Generic)
        .ignore_then(ident())
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
