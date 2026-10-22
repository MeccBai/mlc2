pub mod diagnostic;
mod expr;
mod func;
mod generic;
mod glob;
pub mod out;
mod split;
mod stmt;

use chumsky::{input::Input, prelude::*};

use crate::lexer::{SpannedToken, TokenPack};

use self::out::Span;

pub use glob::module_parser;
pub use out::{
    Spanned, TempConstraints as GenericRequirement, TempExpr as Expr, TempFunc as FunctionDecl,
    TempGenericParam as GenericParam, TempLiteralKind as LiteralKind,
    TempMatchPattern as MatchPattern, TempParam as Param, TempPath as Path, TempScope as Scope,
    TempStmt as Statement, TempType as TypeExpr, TempVisibility as Visibility,
};
pub use out::{TempGlobalStmt, TempModule};

pub type ParseError<'tokens> = Rich<'tokens, TokenPack, Span>;

pub fn parse(
    tokens: &[SpannedToken],
    source_len: usize,
) -> (Option<TempModule>, Vec<ParseError<'_>>) {
    let end_span: Span = (source_len..source_len).into();
    let input = tokens.map(end_span, |(token, span)| (token, span));
    let (module, mut errors) = module_parser().parse(input).into_output_errors();
    if let Some(module) = &module {
        for (item, span) in module {
            let attributes = match item {
                TempGlobalStmt::Func(function) if function.body.is_none() => {
                    Some(&function.symbol.attributes)
                }
                TempGlobalStmt::Interface(interface) if interface.body.is_none() => {
                    Some(&interface.symbol.attributes)
                }
                _ => None,
            };
            if attributes
                .is_some_and(|attributes| attributes.iter().any(|attribute| attribute == "c_abi"))
            {
                errors.push(Rich::custom(
                    *span,
                    "C declarations must be imported from .sym; use mlc symbols header.h",
                ));
            }
        }
    }
    (module, errors)
}

#[cfg(test)]
mod tests;
