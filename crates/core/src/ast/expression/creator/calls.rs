use crate::ast::symbols::Resolution;
use crate::ast::{
    TypeIndex,
    config::Config,
    expression::{Expression, FuncCall},
    symbols::{EnumBool, StatementContext, SymbolTable},
};
use crate::diagnostic::error::{CompileError, IllegalUseError, ResolveError};
use crate::parser::out::{Span, Spanned, TempExpr};

impl Expression {
    pub(super) fn new_interface_call(
        config: &mut Config,
        callee: Spanned<TempExpr>,
        args: Vec<Spanned<TempExpr>>,
        span: Span,
        symbols: &mut dyn Resolution,
        context: Option<&StatementContext>,
    ) -> Self {
        let (owner, interface) = match Self::search_interface(config, callee, symbols, context) {
            Some((owner, interface)) => (owner, interface),
            None => {
                config.submit_error(CompileError::Resolve(ResolveError::UnknownInterface), span);
                return Self::null();
            }
        };
        let mut params = vec![owner];
        let mut params_types = Vec::<TypeIndex>::new();
        params.reserve(args.len() + 1);
        args.into_iter().for_each(|arg| {
            let expr = Expression::new(config, arg, symbols, context);
            params_types.push(expr.type_inference(config, symbols));
            params.push(expr);
        });

        if params.iter().any(|param| param.is_poisoned()) {
            return Self::Poison;
        }

        let interface_params = &symbols.get_interface_regular(interface).params;

        if params_types.len() != interface_params.len() {
            config.submit_error(
                CompileError::IllegalUse(IllegalUseError::ArgumentCountMismatch),
                span,
            );
            return Self::Poison;
        }

        let check_result = params_types.iter().zip(interface_params.iter()).all(
            |(param_type, interface_param)| {
                let check_result = interface_param.0.type_check(false, param_type, symbols);
                if !check_result {
                    config.submit_error(
                        CompileError::IllegalUse(IllegalUseError::TypeMismatched {
                            expected: interface_param.0.format(symbols),
                            found: param_type.format(symbols),
                        }),
                        span,
                    );
                }
                check_result
            },
        );

        if !check_result {
            return Self::Poison;
        }

        Self::FuncCallE(FuncCall {
            func: EnumBool::True(interface),
            args: params,
        })
    }

    pub(super) fn new_function_call(
        config: &mut Config,
        callee: Spanned<TempExpr>,
        args: Vec<Spanned<TempExpr>>,
        span: Span,
        symbols: &mut dyn Resolution,
        context: Option<&StatementContext>,
    ) -> Self {
        if let TempExpr::Path(path) = &callee.0 {
            if path.segments.len() == 1 {
                if let Some(definition) = crate::ast::builtins::lookup(&path.segments[0]) {
                    if definition.type_arguments != 0 {
                        config.submit_error(
                            CompileError::IllegalUse(IllegalUseError::GenericCountMismatch),
                            span,
                        );
                        return Self::Poison;
                    }
                    return Self::new_builtin_call(
                        config,
                        definition.kind,
                        TypeIndex::empty(),
                        args,
                        span,
                        symbols,
                        context,
                    );
                }
            }
        }
        let function = match Self::search_function(config, callee, symbols, context) {
            Some(f) => f,
            None => {
                config.submit_error(CompileError::Resolve(ResolveError::UnknownFunction), span);
                return Self::null();
            }
        };
        let args = args
            .into_iter()
            .map(|arg| Expression::new(config, arg, symbols, context))
            .collect::<Vec<_>>();
        if let EnumBool::False(index) = function {
            if !Self::check_function_args(config, index, &args, symbols, span) {
                return Self::Poison;
            }
        }
        Self::FuncCallE(FuncCall {
            func: function,
            args,
        })
    }
}
