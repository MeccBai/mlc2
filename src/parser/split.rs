use chumsky::{input::ValueInput, prelude::*};

use crate::lexer::{Span, TokenPack, token::Token};

use super::{GenericParam, ParseError, Path, Spanned, TypeExpr, Visibility};

pub fn keyword<'tokens, I>(
    token: Token,
) -> impl Parser<'tokens, I, (), extra::Err<ParseError<'tokens>>> + Clone
where
    I: ValueInput<'tokens, Token = TokenPack, Span = Span>,
{
    just(TokenPack::KeyWord(token)).ignored()
}

pub fn operator<'tokens, I>(
    token: Token,
) -> impl Parser<'tokens, I, (), extra::Err<ParseError<'tokens>>> + Clone
where
    I: ValueInput<'tokens, Token = TokenPack, Span = Span>,
{
    just(TokenPack::Operator(token)).ignored()
}

pub fn ident<'tokens, I>()
-> impl Parser<'tokens, I, String, extra::Err<ParseError<'tokens>>> + Clone
where
    I: ValueInput<'tokens, Token = TokenPack, Span = Span>,
{
    select! { TokenPack::Ident(name) => name }.labelled("identifier")
}

pub fn path<'tokens, I>() -> impl Parser<'tokens, I, Path, extra::Err<ParseError<'tokens>>> + Clone
where
    I: ValueInput<'tokens, Token = TokenPack, Span = Span>,
{
    let segment = choice((ident(), keyword(Token::Generic).to("generic".to_owned())));

    segment
        .separated_by(operator(Token::ColonColon))
        .at_least(1)
        .collect::<Vec<_>>()
        .map(|segments| Path { segments })
        .labelled("path")
}

pub fn visibility<'tokens, I>()
-> impl Parser<'tokens, I, Visibility, extra::Err<ParseError<'tokens>>> + Clone
where
    I: ValueInput<'tokens, Token = TokenPack, Span = Span>,
{
    choice((
        keyword(Token::Public).to(Visibility::Public),
        keyword(Token::Export).to(Visibility::Export),
        keyword(Token::Api).to(Visibility::Api),
    ))
    .or_not()
    .map(Option::unwrap_or_default)
}

pub fn attributes<'tokens, I>()
-> impl Parser<'tokens, I, Vec<String>, extra::Err<ParseError<'tokens>>> + Clone
where
    I: ValueInput<'tokens, Token = TokenPack, Span = Span>,
{
    ident()
        .separated_by(operator(Token::Comma))
        .allow_trailing()
        .collect::<Vec<_>>()
        .delimited_by(keyword(Token::AttributeStart), keyword(Token::AttributeEnd))
        .repeated()
        .collect::<Vec<_>>()
        .map(|groups| groups.into_iter().flatten().collect())
}

pub fn generic_params<'tokens, I>()
-> impl Parser<'tokens, I, Vec<GenericParam>, extra::Err<ParseError<'tokens>>> + Clone
where
    I: ValueInput<'tokens, Token = TokenPack, Span = Span>,
{
    ident()
        .then(operator(Token::Colon).ignore_then(path()).or_not())
        .map(|(name, constraint)| GenericParam { name, constraint })
        .separated_by(operator(Token::Comma))
        .at_least(1)
        .allow_trailing()
        .collect::<Vec<_>>()
        .delimited_by(operator(Token::LAngle), operator(Token::RAngle))
        .or_not()
        .map(Option::unwrap_or_default)
}

pub fn type_parser<'tokens, I>()
-> impl Parser<'tokens, I, Spanned<TypeExpr>, extra::Err<ParseError<'tokens>>> + Clone
where
    I: ValueInput<'tokens, Token = TokenPack, Span = Span>,
{
    recursive(|ty| {
        let generic_args = ty
            .clone()
            .separated_by(operator(Token::Comma))
            .at_least(1)
            .allow_trailing()
            .collect::<Vec<_>>()
            .delimited_by(operator(Token::LAngle), operator(Token::RAngle));

        let named = path()
            .then(generic_args.or_not())
            .map(|(base, args)| match args {
                Some(args) => TypeExpr::Generic { base, args },
                None => TypeExpr::Path(base),
            });

        let reference = operator(Token::Dereference)
            .ignore_then(ty.clone())
            .map(|inner| TypeExpr::Reference(Box::new(inner)));

        choice((reference, named))
            .map_with(|ty, extra| (ty, extra.span()))
            .labelled("type")
    })
    .boxed()
}
