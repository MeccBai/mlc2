//! Build calls using the shared inference session and normal instantiation.
use crate::ast::{
    arena::FuncIndex,
    config::Config,
    expression::{Expression, FuncCall},
    symbols::{EnumBool, Resolution, StatementContext},
};
use crate::diagnostic::error::{CompileError, IllegalUseError};
use crate::parser::out::{Span, Spanned, TempExpr};
impl Expression {
    pub(super) fn new_inferred_call(
        config: &mut Config,
        template: FuncIndex,
        args: Vec<Spanned<TempExpr>>,
        span: Span,
        symbols: &mut dyn Resolution,
        context: Option<&StatementContext>,
    ) -> Self {
        let symbol = symbols.get_function(template, true).clone();
        let variadic = symbol.params.last().is_some_and(|(_, name)| name == "...");
        let fixed = symbol.params.len() - usize::from(variadic);
        if args.len() < fixed || (!variadic && args.len() != fixed) {
            config.submit_error(
                CompileError::IllegalUse(IllegalUseError::ArgumentCountMismatch),
                span,
            );
            return Self::Poison;
        }
        let args: Vec<_> = args
            .into_iter()
            .map(|arg| Self::new(config, arg, symbols, context))
            .collect();
        if config.is_poisoned() {
            return Self::Poison;
        }
        let Some(bindings) =
            crate::TypeInfer::new(config, symbols).generic_arguments(&symbol, &args, span)
        else {
            return Self::Poison;
        };
        let index = template.instantiation(
            config,
            &bindings,
            symbols,
            context.and_then(|ctx| ctx.instantiation_actives()),
            span,
        );
        if config.is_poisoned() || !Self::check_function_args(config, index, &args, symbols, span) {
            return Self::Poison;
        }
        Self::FuncCallE(FuncCall {
            callee: None,
            func: EnumBool::False(index),
            args,
        })
    }
}
