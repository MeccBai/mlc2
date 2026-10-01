use super::{ReturnStatement, Statement};
use crate::ast::{
    config::Config,
    expression::Expression,
    symbols::{EnumBool, StatementContext, SymbolTable},
};
use crate::error::{CompileError, IllegalUseError};
use crate::parser::out::{Span, Spanned, TempExpr};

impl Statement {
    pub(super) fn create_return(
        config: &mut Config,
        value: Option<Spanned<TempExpr>>,
        symbols: &mut SymbolTable,
        context: Option<&StatementContext>,
        span: Span,
    ) -> Self {
        let expected = match context.map(|context| context.belong()) {
            Some(EnumBool::False(index)) if !index.is_empty() => {
                symbols.functions.get(*index).ret_type
            }
            Some(EnumBool::True(index)) if !index.is_empty() => {
                symbols.interfaces.get(*index).ret_type
            }
            _ => {
                config.submit_error(
                    CompileError::IllegalUse(IllegalUseError::ReturnOutsideFunction),
                    span,
                );
                return Self::Poison;
            }
        };
        let value_span = value.as_ref().map_or(span, |value| value.1);
        let value = value.map(|value| Expression::new(config, value, symbols, context));
        if config.is_poisoned() {
            return Self::Poison;
        }
        match (expected, &value) {
            (Some(_), None) => config.submit_error(
                CompileError::IllegalUse(IllegalUseError::ReturnValueRequired),
                span,
            ),
            (None, Some(_)) => config.submit_error(
                CompileError::IllegalUse(IllegalUseError::UnexpectedReturnValue),
                value_span,
            ),
            (Some(expected), Some(value)) if !value.type_check(&expected, config, symbols) => {
                let found = value.type_inference(config, symbols);
                let found = if found.is_empty() {
                    "void".into()
                } else {
                    found.format(&symbols.types)
                };
                config.submit_error(
                    CompileError::IllegalUse(IllegalUseError::TypeMismatched {
                        expected: expected.format(&symbols.types),
                        found,
                    }),
                    value_span,
                );
            }
            _ => {}
        }
        Self::ReturnBlock(ReturnStatement { value })
    }
}
