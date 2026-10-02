use crate::ast::{arena::FuncIndex, config::Config, expression::Expression, symbols::SymbolTable};
use crate::diagnostic::error::{CompileError, IllegalUseError};
use crate::parser::out::Span;

impl Expression {
    /// Validate the fixed prefix of a concrete function signature. The final
    /// `...` marker is not a parameter type and permits any number of extra args.
    pub(super) fn check_function_args(
        config: &mut Config,
        function: FuncIndex,
        args: &[Expression],
        symbols: &mut SymbolTable,
        span: Span,
    ) -> bool {
        if config.is_poisoned() || args.iter().any(Expression::is_poisoned) {
            return false;
        }
        let params = symbols.functions.get(function).params.clone();
        let variadic = params.last().is_some_and(|(_, name)| name == "...");
        let fixed = params.len() - usize::from(variadic);
        if args.len() < fixed || (!variadic && args.len() != fixed) {
            config.submit_error(
                CompileError::IllegalUse(IllegalUseError::ArgumentCountMismatch),
                span,
            );
            return false;
        }
        for (argument, (expected, _)) in args.iter().zip(params.iter().take(fixed)) {
            let found = argument.type_inference(config, symbols);
            if config.is_poisoned() {
                return false;
            }
            if found.is_empty() || !expected.type_check(false, &found, &symbols.types) {
                let found = if found.is_empty() {
                    "void".to_owned()
                } else {
                    found.format(&symbols.types)
                };
                config.submit_error(
                    CompileError::IllegalUse(IllegalUseError::TypeMismatched {
                        expected: expected.format(&symbols.types),
                        found,
                    }),
                    span,
                );
                return false;
            }
        }
        true
    }
}
