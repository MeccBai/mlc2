use chumsky::{input::ValueInput, prelude::*};

use crate::lexer::{TokenPack, token::Token};

use super::{
    FunctionDecl, Param, ParseError,
    out::{Span, TempFuncSymbol, TempInterface, TempInterfaceSymbol},
    split::{generic_params, keyword, operator, path, spanned_ident, type_parser},
    stmt::scope_parser,
};

#[derive(Clone)]
enum ParsedParam {
    Receiver { mutable: bool },
    Typed(Param),
    Variadic(Span),
}

fn parameter_parser<'tokens, I>()
-> impl Parser<'tokens, I, ParsedParam, extra::Err<ParseError<'tokens>>> + Clone
where
    I: ValueInput<'tokens, Token = TokenPack, Span = Span>,
{
    let immutable_self =
        just(TokenPack::Ident("self".to_owned())).to(ParsedParam::Receiver { mutable: false });
    let mutable_self = keyword(Token::Mut)
        .ignore_then(just(TokenPack::Ident("self".to_owned())))
        .to(ParsedParam::Receiver { mutable: true });

    let typed_param = spanned_ident()
        .then_ignore(operator(Token::Colon))
        .then(type_parser())
        .map(|((name, name_span), ty)| {
            ParsedParam::Typed(Param {
                name,
                name_span,
                ty: Some(ty),
            })
        });

    let variadic = keyword(Token::VarList).map_with(|_, extra| ParsedParam::Variadic(extra.span()));

    choice((mutable_self, immutable_self, typed_param, variadic))
        .labelled("function parameter")
        .boxed()
}

fn signature_parser<'tokens, I>() -> impl Parser<
    'tokens,
    I,
    (
        Vec<super::GenericParam>,
        super::Spanned<String>,
        Vec<ParsedParam>,
        Option<super::Spanned<super::TypeExpr>>,
    ),
    extra::Err<ParseError<'tokens>>,
> + Clone
where
    I: ValueInput<'tokens, Token = TokenPack, Span = Span>,
{
    let params = parameter_parser()
        .separated_by(operator(Token::Comma))
        .allow_trailing()
        .collect::<Vec<_>>()
        .delimited_by(keyword(Token::LParen), keyword(Token::RParen));

    generic_params()
        .then(spanned_ident())
        .then(params)
        .then(operator(Token::Arrow).ignore_then(type_parser()).or_not())
        .try_map(|(((generics, name), params), return_type), _| {
            if let Some(ParsedParam::Variadic(span)) = params
                .iter()
                .rev()
                .skip(1)
                .find(|param| matches!(param, ParsedParam::Variadic(_)))
            {
                return Err(Rich::custom(*span, "... must be the last parameter"));
            }
            Ok((generics, name, params, return_type))
        })
        .boxed()
}

fn body_parser<'tokens, I>()
-> impl Parser<'tokens, I, Option<super::Scope>, extra::Err<ParseError<'tokens>>> + Clone
where
    I: ValueInput<'tokens, Token = TokenPack, Span = Span>,
{
    scope_parser()
        .map(Some)
        .or(keyword(Token::Semicolon).to(None))
        .boxed()
}

pub fn function_parser<'tokens, I>()
-> impl Parser<'tokens, I, FunctionDecl, extra::Err<ParseError<'tokens>>> + Clone
where
    I: ValueInput<'tokens, Token = TokenPack, Span = Span>,
{
    keyword(Token::Function)
        .ignore_then(signature_parser())
        .then(body_parser())
        .try_map(
            |((generics, (name, name_span), params, return_type), body), span| {
                let params = std::iter::Iterator::collect::<Result<Vec<_>, _>>(
                    params.into_iter().map(|param| match param {
                        ParsedParam::Typed(param) => Ok(param),
                        ParsedParam::Variadic(name_span) => Ok(Param {
                            name: "...".to_owned(),
                            name_span,
                            ty: None,
                        }),
                        ParsedParam::Receiver { .. } => {
                            Err(Rich::custom(span, "self is only valid in an interface"))
                        }
                    }),
                )?;
                Ok(FunctionDecl {
                    symbol: TempFuncSymbol {
                        visibility: Default::default(),
                        name,
                        name_span,
                        generics,
                        params,
                        return_type,
                        attributes: Vec::new(),
                    },
                    body,
                })
            },
        )
        .labelled("function")
        .boxed()
}

pub fn interface_symbol_parser<'tokens, I>(
    require_owner: bool,
) -> impl Parser<'tokens, I, TempInterfaceSymbol, extra::Err<ParseError<'tokens>>> + Clone
where
    I: ValueInput<'tokens, Token = TokenPack, Span = Span>,
{
    let owner = path()
        .map_with(|path, extra| (path, extra.span()))
        .then_ignore(operator(Token::ColonColon))
        .map(Some);
    let owner = if require_owner {
        owner.boxed()
    } else {
        owner.or_not().map(Option::flatten).boxed()
    };

    owner
        .then_ignore(keyword(Token::Function))
        .then(signature_parser())
        .try_map(
            |(owner, (generics, (name, name_span), mut params, return_type)), span| {
                let receiver = match params.first() {
                    Some(ParsedParam::Receiver { mutable }) => Some(*mutable),
                    _ => None,
                };
                let has_self = receiver.is_some();
                let mutable = receiver.unwrap_or(false);
                if has_self {
                    params.remove(0);
                }
                let params = std::iter::Iterator::collect::<Result<Vec<_>, _>>(
                    params.into_iter().map(|param| match param {
                        ParsedParam::Typed(param) => Ok(param),
                        ParsedParam::Variadic(name_span) => Ok(Param {
                            name: "...".to_owned(),
                            name_span,
                            ty: None,
                        }),
                        ParsedParam::Receiver { .. } => Err(Rich::custom(
                            span,
                            "self must be the first interface parameter",
                        )),
                    }),
                )?;
                Ok(TempInterfaceSymbol {
                    visibility: Default::default(),
                    owner,
                    has_self,
                    mutable,
                    name,
                    name_span,
                    generics,
                    params,
                    return_type,
                    attributes: Vec::new(),
                })
            },
        )
        .labelled("interface symbol")
        .boxed()
}

pub fn interface_parser<'tokens, I>()
-> impl Parser<'tokens, I, TempInterface, extra::Err<ParseError<'tokens>>> + Clone
where
    I: ValueInput<'tokens, Token = TokenPack, Span = Span>,
{
    interface_symbol_parser(true)
        .then(body_parser())
        .map(|(symbol, body)| TempInterface { symbol, body })
        .labelled("interface")
        .boxed()
}
