use chumsky::{input::ValueInput, prelude::*};

use crate::language::ImportModule;
use crate::language::ValueType;
use crate::lexer::{TokenPack, token::Token};

use super::{
    ParseError, Spanned,
    func::{function_parser, interface_parser},
    generic::generic_parser,
    out::{
        Span, TempConstraints, TempEnum, TempFunc, TempGeneric, TempGenericParam, TempGlobalStmt,
        TempInterface, TempModule, TempPath, TempType, TempUnit, TempUnitMember, TempUsing,
        TempVar, TempVisibility,
    },
    split::{
        attributes, generic_params, keyword, operator, path, spanned_ident, type_parser, visibility,
    },
};

enum RawItem {
    Import(TempPath),
    Function(TempFunc),
    Interface(TempInterface),
    Unit {
        name: Spanned<String>,
        generics: Vec<TempGenericParam>,
        members: Vec<TempUnitMember>,
        is_union: bool,
    },
    Using {
        name: Spanned<String>,
        ty: Spanned<TempType>,
    },
    Generic {
        name: Spanned<String>,
        requirements: Vec<Spanned<TempConstraints>>,
    },
    Enum {
        name: Spanned<String>,
        variants: Vec<Spanned<String>>,
    },
}

pub fn item_parser<'tokens, I>()
-> impl Parser<'tokens, I, Spanned<TempGlobalStmt>, extra::Err<ParseError<'tokens>>> + Clone
where
    I: ValueInput<'tokens, Token = TokenPack, Span = Span>,
{
    let import = keyword(Token::Import)
        .ignore_then(path())
        .then_ignore(keyword(Token::Semicolon))
        .map(RawItem::Import);

    let member = keyword(Token::Public)
        .or_not()
        .map(|public| public.is_some())
        .then(spanned_ident())
        .then_ignore(operator(Token::Colon))
        .then(type_parser())
        .then_ignore(keyword(Token::Semicolon))
        .map(|((public, (name, name_span)), ty)| TempUnitMember {
            name,
            name_span,
            ty,
            public,
        });
    let aggregate_name = generic_params()
        .then(spanned_ident())
        .then(generic_params())
        .try_map(|((prefix, name), suffix), span| {
            if !prefix.is_empty() && !suffix.is_empty() {
                return Err(Rich::custom(
                    span,
                    "Declare generic parameters either before or after the name, not both",
                ));
            }
            Ok((name, if prefix.is_empty() { suffix } else { prefix }))
        });
    let unit = keyword(Token::Unit)
        .ignore_then(aggregate_name.clone())
        .then(
            member
                .repeated()
                .collect::<Vec<_>>()
                .delimited_by(keyword(Token::LBrace), keyword(Token::RBrace)),
        )
        .then_ignore(keyword(Token::Semicolon).or_not())
        .map(|((name, generics), members)| RawItem::Unit {
            name,
            generics,
            members,
            is_union: false,
        });

    let union = keyword(Token::Union)
        .ignore_then(aggregate_name)
        .then(
            type_parser()
                .separated_by(operator(Token::Comma))
                .at_least(1)
                .allow_trailing()
                .collect::<Vec<_>>()
                .delimited_by(keyword(Token::LeftBracket), keyword(Token::RightBracket)),
        )
        .then_ignore(keyword(Token::Semicolon))
        .map(|((name, generics), candidates)| RawItem::Unit {
            name,
            generics,
            is_union: true,
            members: candidates
                .into_iter()
                .enumerate()
                .map(|(index, ty)| TempUnitMember {
                    name: format!("$variant{index}"),
                    name_span: ty.1,
                    ty,
                    public: false,
                })
                .collect(),
        });

    let using = keyword(Token::Using)
        .ignore_then(spanned_ident())
        .then_ignore(operator(Token::Assign))
        .then(type_parser())
        .then_ignore(keyword(Token::Semicolon))
        .map(|(name, ty)| RawItem::Using { name, ty });

    let generic =
        generic_parser().map(|(name, requirements)| RawItem::Generic { name, requirements });

    let function = function_parser().map(RawItem::Function);
    let interface = interface_parser().map(RawItem::Interface);

    let enum_ = keyword(Token::Enum)
        .ignore_then(spanned_ident())
        .then(
            spanned_ident()
                .separated_by(operator(Token::Comma))
                .at_least(1)
                .allow_trailing()
                .collect::<Vec<_>>()
                .delimited_by(keyword(Token::LBrace), keyword(Token::RBrace)),
        )
        .then_ignore(keyword(Token::Semicolon).or_not())
        .map(|(name, variants)| RawItem::Enum { name, variants });

    let global_variable = keyword(Token::Global)
        .ignore_then(choice((
            keyword(Token::Variable).to(ValueType::Flex),
            keyword(Token::Value).to(ValueType::Final),
            keyword(Token::Constant).to(ValueType::Constant),
        )))
        .then(spanned_ident())
        .then(operator(Token::Colon).ignore_then(type_parser()).or_not())
        .then(operator(Token::Assign).ignore_then(super::expr::expression_parser()))
        .then_ignore(keyword(Token::Semicolon))
        .map(|(((value_type, (name, name_span)), ty), initializer)| {
            TempGlobalStmt::Variable(TempVar {
                name,
                name_span,
                ty,
                initializer,
                value_type,
            })
        })
        .map_with(|item, extra| (item, extra.span()));

    let declaration = attributes()
        .then(visibility().map_with(|visibility, extra| (visibility, extra.span())))
        .then(choice((
            import, union, unit, using, generic, enum_, interface, function,
        )))
        .try_map(|((attributes, (visibility, visibility_span)), raw), _| {
            if !matches!(raw, RawItem::Interface(_))
                && matches!(visibility, TempVisibility::Public | TempVisibility::Api)
            {
                return Err(Rich::custom(
                    visibility_span,
                    "pub and api are only valid for interfaces",
                ));
            }
            Ok(match raw {
                RawItem::Import(path) => TempGlobalStmt::Import(ImportModule::new(
                    path.segments,
                    matches!(visibility, TempVisibility::Export),
                )),
                RawItem::Function(mut function) => {
                    function.symbol.visibility = visibility;
                    function.symbol.attributes = attributes;
                    TempGlobalStmt::Func(function)
                }
                RawItem::Interface(mut interface) => {
                    interface.symbol.visibility = visibility;
                    interface.symbol.attributes = attributes;
                    TempGlobalStmt::Interface(interface)
                }
                RawItem::Unit {
                    name: (name, name_span),
                    generics,
                    members,
                    is_union,
                } => TempGlobalStmt::Unit(TempUnit {
                    is_union,
                    visibility,
                    name,
                    name_span,
                    generics,
                    members,
                    attributes,
                }),
                RawItem::Using {
                    name: (name, name_span),
                    ty,
                } => TempGlobalStmt::Using(TempUsing {
                    visibility,
                    name,
                    name_span,
                    target: ty,
                }),
                RawItem::Generic {
                    name: (name, name_span),
                    requirements,
                } => TempGlobalStmt::Generic(TempGeneric {
                    visibility,
                    name,
                    name_span,
                    requirements,
                    attributes,
                }),
                RawItem::Enum {
                    name: (name, name_span),
                    variants,
                } => TempGlobalStmt::Enum(TempEnum {
                    visibility,
                    name,
                    name_span,
                    variants,
                    attributes,
                }),
            })
        })
        .map_with(|item, extra| (item, extra.span()))
        .labelled("module declaration");

    choice((global_variable, declaration))
        .labelled("module item")
        .boxed()
}

pub fn module_parser<'tokens, I>()
-> impl Parser<'tokens, I, TempModule, extra::Err<ParseError<'tokens>>> + Clone
where
    I: ValueInput<'tokens, Token = TokenPack, Span = Span>,
{
    item_parser()
        .repeated()
        .collect::<Vec<_>>()
        .then_ignore(end())
        .boxed()
}
