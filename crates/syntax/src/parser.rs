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
    module_parser().parse(input).into_output_errors()
}

#[cfg(test)]
mod tests;
