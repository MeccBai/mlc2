use chumsky::{input::ValueInput, prelude::*};

use crate::lexer::{Span, TokenPack, token::Token};

use super::{
    FunctionDecl, Param, ParseError,
    out::TempFuncSymbol,
    split::{generic_params, ident, keyword, operator, path, type_parser},
    stmt::scope_parser,
};

pub fn parameter_parser<'tokens, I>()
-> impl Parser<'tokens, I, Param, extra::Err<ParseError<'tokens>>> + Clone
where
    I: ValueInput<'tokens, Token = TokenPack, Span = Span>,
{
    let immutable_self = just(TokenPack::Ident("self".to_owned())).to(Param {
        name: "self".to_owned(),
        ty: None,
        is_self: true,
        mutable: false,
    });
    let mutable_self = keyword(Token::Mut)
        .ignore_then(just(TokenPack::Ident("self".to_owned())))
        .to(Param {
            name: "self".to_owned(),
            ty: None,
            is_self: true,
            mutable: true,
        });

    let typed_param = ident()
        .then_ignore(operator(Token::Colon))
        .then(type_parser())
        .map(|(name, ty)| Param {
            name,
            ty: Some(ty),
            is_self: false,
            mutable: false,
        });

    choice((mutable_self, immutable_self, typed_param))
        .labelled("function parameter")
        .boxed()
}

pub fn function_parser<'tokens, I>()
-> impl Parser<'tokens, I, FunctionDecl, extra::Err<ParseError<'tokens>>> + Clone
where
    I: ValueInput<'tokens, Token = TokenPack, Span = Span>,
{
    let owner = path()
        .then_ignore(operator(Token::ColonColon))
        .then_ignore(keyword(Token::Function))
        .map(Some)
        .or(keyword(Token::Function).to(None));

    let params = parameter_parser()
        .separated_by(operator(Token::Comma))
        .allow_trailing()
        .collect::<Vec<_>>()
        .delimited_by(keyword(Token::LParen), keyword(Token::RParen));

    let return_type = operator(Token::Arrow).ignore_then(type_parser()).or_not();

    let body = scope_parser()
        .map(Some)
        .or(keyword(Token::Semicolon).to(None));

    owner
        .then(generic_params())
        .then(ident())
        .then(params)
        .then(return_type)
        .then(body)
        .map(
            |(((((owner, generics), name), params), return_type), body)| FunctionDecl {
                symbol: TempFuncSymbol {
                    visibility: Default::default(),
                    owner,
                    name,
                    generics,
                    params,
                    return_type,
                    attributes: Vec::new(),
                },
                body,
            },
        )
        .labelled("function")
        .boxed()
}
