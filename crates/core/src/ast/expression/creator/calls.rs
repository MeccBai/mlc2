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
        if let TempExpr::Member {
            base,
            name,
            indirect: false,
            ..
        } = &callee.0
        {
            if name == "value" {
                let owner = Self::new(config, (**base).clone(), symbols, context);
                let ty = owner.type_inference(config, symbols);
                if !ty.is_empty()
                    && matches!(
                        symbols.get_type(ty).unqualified(),
                        crate::ast::types::CompileType::Enum(_)
                    )
                {
                    if !args.is_empty() {
                        config.submit_error(
                            CompileError::IllegalUse(IllegalUseError::ArgumentCountMismatch),
                            span,
                        );
                        return Self::Poison(span);
                    }
                    let underlying = match symbols.get_type(ty).unqualified() {
                        crate::ast::types::CompileType::Enum(e) => e.underlying,
                        _ => unreachable!(),
                    };
                    if let Self::InitListE(
                        crate::ast::expression::InitialList::List { values, .. },
                        _,
                    ) = &owner
                    {
                        return values[0].clone().with_span(span);
                    }
                    if let Self::ConstValueE(mut value, _) = owner {
                        value.ty = underlying;
                        return Self::ConstValueE(value, span);
                    }
                    return Self::UnaryExprE(
                        crate::ast::expression::UnaryExpr::Operator {
                            op: crate::ast::expression::Operator::EnumValue,
                            value: Box::new(crate::ast::expression::CompAtom::from_expr(owner)),
                        },
                        span,
                    );
                }
            }
        }
        let indirect = matches!(&callee.0, TempExpr::Member { indirect: true, .. });
        let (owner, mut interface, generic) =
            match Self::search_interface(config, callee, symbols, context) {
                Some((owner, interface, generic)) => (owner, interface, generic),
                None => {
                    config
                        .submit_error(CompileError::Resolve(ResolveError::UnknownInterface), span);
                    return Self::null();
                }
            };
        let symbol = symbols.get_interface(interface, generic).clone();
        let mut params = Vec::new();
        if let Some(receiver_type) = symbol.receiver_type {
            if symbol.mutable && !owner.assignable(config, symbols) && !indirect {
                config.submit_error(
                    CompileError::IllegalUse(IllegalUseError::InvalidAssignment),
                    span,
                );
                return Self::Poison(span);
            }
            let owner_span = owner.span();
            let receiver = if indirect {
                owner
            } else {
                Self::UnaryExprE(
                    crate::ast::expression::UnaryExpr::Operator {
                        op: if symbol.mutable {
                            crate::ast::expression::Operator::MutOf
                        } else {
                            crate::ast::expression::Operator::AddressOf
                        },
                        value: Box::new(crate::ast::expression::CompAtom::from_expr(owner)),
                    },
                    owner_span,
                )
            };
            if !crate::TypeInfer::new(config, symbols).check(
                &receiver,
                receiver_type,
                crate::infer::CheckMode::Argument,
                span,
            ) {
                return Self::Poison(span);
            }
            params.push(receiver);
        }
        params.reserve(args.len());
        for arg in args {
            params.push(Expression::new(config, arg, symbols, context));
        }
        let offset = usize::from(symbol.receiver_type.is_some());
        if generic {
            let Some(index) =
                Self::infer_interface(config, interface, &params[offset..], span, symbols, context)
            else {
                return Self::Poison(span);
            };
            interface = index;
        } else if !crate::TypeInfer::new(config, symbols).check_arguments(
            &symbol.params,
            &params[offset..],
            span,
        ) {
            return Self::Poison(span);
        }

        Self::FuncCallE(
            FuncCall {
                pipeline: false,
                callee: None,
                func: EnumBool::True(interface),
                args: params,
            },
            span,
        )
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
            if let Some((kind, bits, signed, value)) =
                crate::ast::builtins::numeric_constant(&path.segments)
            {
                if !args.is_empty() {
                    config.submit_error(
                        CompileError::IllegalUse(IllegalUseError::ArgumentCountMismatch),
                        span,
                    );
                    return Self::Poison(span);
                }
                let ty = symbols.get_base(kind, bits, signed);
                return Self::ConstValueE(crate::ast::expression::ConstValue { ty, value }, span);
            }
        }
        if let TempExpr::Path(path) = &callee.0 {
            if Self::is_function_builtin(path) {
                return Self::new_function_pointer(
                    config,
                    Vec::new(),
                    args,
                    span,
                    symbols,
                    context,
                );
            }
            if matches!(
                symbols.resolve_path(config, path, context),
                Some(crate::ast::symbols::PathSymbol::Variable(_))
            ) {
                let callee = Self::new(config, callee, symbols, context);
                return Self::new_indirect_call(config, callee, args, span, symbols, context);
            }
        } else {
            let callee = Self::new(config, callee, symbols, context);
            return Self::new_indirect_call(config, callee, args, span, symbols, context);
        }
        if let TempExpr::Path(path) = &callee.0 {
            if path.segments.len() == 1 || path.segments.first().is_some_and(|part| part == "str") {
                if let Some(definition) = crate::ast::builtins::lookup(&path.segments.join("::")) {
                    if definition.type_arguments != 0 {
                        config.submit_error(
                            CompileError::IllegalUse(IllegalUseError::GenericCountMismatch),
                            span,
                        );
                        return Self::Poison(span);
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
        if let TempExpr::Path(path) = &callee.0 {
            use crate::ast::symbols::PathSymbol;
            let resolved = symbols.resolve_path(config, path, context);
            if let Some(PathSymbol::Function { index, generic }) = resolved {
                if let Some(kind) = symbols.get_function(index, generic).builtin() {
                    if generic {
                        config.submit_error(
                            CompileError::IllegalUse(IllegalUseError::GenericCountMismatch),
                            span,
                        );
                        return Self::Poison(span);
                    }
                    return Self::new_builtin_call(
                        config,
                        kind,
                        TypeIndex::empty(),
                        args,
                        span,
                        symbols,
                        context,
                    );
                }
                if generic {
                    return Self::new_inferred_call(config, index, args, span, symbols, context);
                }
            }
            if let Some(PathSymbol::Interface { generic: true, .. }) = resolved {
                // Reuse lookup's visibility checks before instantiating the template.
                let Some(EnumBool::True(index)) =
                    Self::search_generic_function(config, callee, symbols, context)
                else {
                    return Self::Poison(span);
                };
                return Self::new_inferred_interface_call(
                    config, index, args, span, symbols, context,
                );
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
                return Self::Poison(span);
            }
        }
        Self::FuncCallE(
            FuncCall {
                pipeline: false,
                callee: None,
                func: function,
                args,
            },
            span,
        )
    }
}
