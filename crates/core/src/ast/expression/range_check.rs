//! AST adapter for the shared range/precision checker.
use super::Expression;
use crate::ast::{TypeIndex, config::Config, symbols::Resolution};
use crate::parser::out::Span;
impl Expression {
    pub(crate) fn check_constant_range(
        &self,
        target: TypeIndex,
        config: &mut Config,
        symbols: &dyn Resolution,
        span: Span,
    ) {
        crate::TypeInfer::read_only(config, symbols).check_constant_range(self, target, span)
    }
}
