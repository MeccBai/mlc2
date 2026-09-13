use chumsky::{input::ValueInput, prelude::*};

use crate::ast::ImportModule;
use crate::lexer::{Span, TokenPack, token::Token};

use super::{
    ParseError, Spanned,
    func::function_parser,
    generic::generic_parser,
    out::{
        TempConstraints, TempEnum, TempFunc, TempGeneric, TempGenericParam, TempGlobalStmt,
        TempModule, TempPath, TempType, TempUnit, TempUnitMember, TempUsing, TempVar,
        TempVisibility,
    },
    split::{attributes, generic_params, ident, keyword, operator, path, type_parser, visibility},
};

enum RawItem {
    Import(TempPath),
    Function(TempFunc),
    Unit {
        name: String,
        generics: Vec<TempGenericParam>,
        members: Vec<TempUnitMember>,
    },
    Using {
        name: String,
        ty: Spanned<TempType>,
    },
    Generic {
        name: String,
        requirements: Vec<TempConstraints>,
    },
    Enum {
        name: String,
        variants: Vec<String>,
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
        .then(ident())
        .then_ignore(operator(Token::Colon))
        .then(type_parser())
        .then_ignore(keyword(Token::Semicolon))
        .map(|((public, name), ty)| TempUnitMember { name, ty, public });
    let unit = keyword(Token::Unit)
        .ignore_then(ident())
        .then(generic_params())
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
        });

    let using = keyword(Token::Using)
        .ignore_then(ident())
        .then_ignore(operator(Token::Assign))
        .then(type_parser())
        .then_ignore(keyword(Token::Semicolon))
        .map(|(name, ty)| RawItem::Using { name, ty });

    let generic =
        generic_parser().map(|(name, requirements)| RawItem::Generic { name, requirements });

    let function = function_parser().map(RawItem::Function);

    let enum_ = keyword(Token::Enum)
        .ignore_then(ident())
        .then(
            ident()
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
            keyword(Token::Variable).to(false),
            keyword(Token::Constant).to(true),
        )))
        .then(ident())
        .then(operator(Token::Colon).ignore_then(type_parser()).or_not())
        .then(
            operator(Token::Assign)
                .ignore_then(super::expr::expression_parser())
                .or_not(),
        )
        .then_ignore(keyword(Token::Semicolon))
        .map(|(((constant, name), ty), initializer)| {
            TempGlobalStmt::Variable(TempVar {
                name,
                ty,
                initializer,
                constant,
                generics: Vec::new(),
            })
        })
        .map_with(|item, extra| (item, extra.span()));

    let declaration = attributes()
        .then(visibility())
        .then(choice((import, unit, using, generic, enum_, function)))
        .map(|((attributes, visibility), raw)| match raw {
            RawItem::Import(path) => TempGlobalStmt::Import(ImportModule::new(
                path.segments,
                matches!(visibility, TempVisibility::Export | TempVisibility::Api),
            )),
            RawItem::Function(mut function) => {
                function.visibility = visibility;
                function.attributes = attributes;
                TempGlobalStmt::Func(function)
            }
            RawItem::Unit {
                name,
                generics,
                members,
            } => TempGlobalStmt::Unit(TempUnit {
                visibility,
                name,
                generics,
                members,
                attributes,
            }),
            RawItem::Using { name, ty } => TempGlobalStmt::Using(TempUsing {
                visibility,
                name,
                target: ty,
            }),
            RawItem::Generic { name, requirements } => TempGlobalStmt::Generic(TempGeneric {
                visibility,
                name,
                requirements,
                attributes,
            }),
            RawItem::Enum { name, variants } => TempGlobalStmt::Enum(TempEnum {
                visibility,
                name,
                variants,
                attributes,
            }),
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
