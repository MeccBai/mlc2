use crate::ast::{arena::FuncIndex, config::Config, expression::Expression, symbols::Resolution};
use crate::parser::out::Span;
impl Expression {
    /// Validate the fixed prefix of a concrete function signature. The final
    /// `...` marker is not a parameter type and permits any number of extra args.
    pub(super) fn check_function_args(
        config: &mut Config,
        function: FuncIndex,
        args: &[Expression],
        symbols: &mut dyn Resolution,
        span: Span,
    ) -> bool {
        let params = symbols.get_function_regular(function).params.clone();
        crate::TypeInfer::new(config, symbols).check_arguments(&params, args, span)
    }
}
