use super::*;
use crate::ast::{
    symbols::{PathSymbol, StatementContext},
    types::FunctionType,
};
use crate::parser::out::{Span, TempPath, TempType};

impl Expression {
    pub(super) fn new_member_callable(
        config: &mut Config,
        callee: Spanned<TempExpr>,
        args: Vec<Spanned<TempExpr>>,
        span: Span,
        symbols: &mut dyn Resolution,
        context: Option<&StatementContext>,
    ) -> Self {
        let TempExpr::Member {
            base,
            indirect,
            name,
            ..
        } = &callee.0
        else {
            unreachable!()
        };
        let owner = Self::new(config, (**base).clone(), symbols, context);
        if config.is_poisoned() {
            return Self::Poison;
        }
        let mut ty = owner.type_inference(config, symbols);
        if *indirect && !ty.is_empty() && ty.is_ref(symbols) {
            ty = ty.deref(symbols).unwrap();
        }
        let has_field = !ty.is_empty()
            && matches!(symbols.get_type(ty).unqualified(), CompileType::Unit(unit) if unit.get_member(name).is_some());
        if has_field {
            let callee = Self::new(config, callee, symbols, context);
            Self::new_indirect_call(config, callee, args, span, symbols, context)
        } else {
            Self::new_interface_call(config, callee, args, span, symbols, context)
        }
    }
    pub(super) fn new_function_pointer(
        config: &mut Config,
        generics: Vec<Spanned<TempType>>,
        mut args: Vec<Spanned<TempExpr>>,
        span: Span,
        symbols: &mut dyn Resolution,
        context: Option<&StatementContext>,
    ) -> Self {
        if args.len() != 1 {
            config.submit_error(
                CompileError::IllegalUse(IllegalUseError::ArgumentCountMismatch),
                span,
            );
            return Self::Poison;
        }
        let (TempExpr::Path(path), arg_span) = args.remove(0) else {
            config.submit_error(
                CompileError::IllegalUse(IllegalUseError::SymbolNotCallable),
                span,
            );
            return Self::Poison;
        };
        let Some(PathSymbol::Function { mut index, generic }) =
            symbols.resolve_path(config, &path, context)
        else {
            config.submit_error(
                CompileError::IllegalUse(IllegalUseError::SymbolNotCallable),
                arg_span,
            );
            return Self::Poison;
        };
        let symbol = symbols.get_function(index, generic).clone();
        if symbol.builtin().is_some() {
            config.submit_error(
                CompileError::IllegalUse(IllegalUseError::UnsupportedSymbolValue),
                arg_span,
            );
            return Self::Poison;
        }
        if symbol.generics.len() != generics.len() {
            config.submit_error(
                CompileError::IllegalUse(IllegalUseError::GenericCountMismatch),
                span,
            );
            return Self::Poison;
        }
        if generic {
            let types = generics
                .into_iter()
                .map(|ty| {
                    resolve_type(
                        config,
                        ty,
                        symbols,
                        context.map(|ctx| ctx as &dyn crate::ast::types::TypeContext),
                    )
                })
                .collect::<Option<Vec<_>>>();
            let Some(types) = types else {
                return Self::Poison;
            };
            if types.iter().any(|ty| ty.is_generic(symbols)) {
                config.submit_error(
                    CompileError::IllegalUse(IllegalUseError::UnsupportedGenericCall),
                    span,
                );
                return Self::Poison;
            }
            let bindings = symbol
                .generics
                .iter()
                .zip(types)
                .map(|(name, ty)| (symbol.generic_map[name], ty))
                .collect();
            index = index.instantiation(
                config,
                &bindings,
                symbols,
                context.and_then(|ctx| ctx.instantiation_actives()),
                span,
            );
            if config.is_poisoned() {
                return Self::Poison;
            }
        }
        let signature = FunctionType::new(index, symbols.get_function_regular(index));
        let name = signature.format(symbols);
        let ty = symbols
            .local_mut()
            .types
            .insert(name, CompileType::Function(signature));
        Self::UnaryExprE(UnaryExpr::Function {
            function: index,
            ty,
        })
    }

    pub(super) fn new_indirect_call(
        config: &mut Config,
        callee: Expression,
        args: Vec<Spanned<TempExpr>>,
        span: Span,
        symbols: &mut dyn Resolution,
        context: Option<&StatementContext>,
    ) -> Self {
        if callee.is_poisoned() {
            return Self::Poison;
        }
        let ty = callee.type_inference(config, symbols);
        let Some(signature) = (!ty.is_empty())
            .then(|| symbols.get_type(ty).unqualified())
            .and_then(|ty| match ty {
                CompileType::Function(signature) => Some(signature.signature),
                _ => None,
            })
        else {
            config.submit_error(
                CompileError::IllegalUse(IllegalUseError::SymbolNotCallable),
                span,
            );
            return Self::Poison;
        };
        let args = args
            .into_iter()
            .map(|arg| Self::new(config, arg, symbols, context))
            .collect::<Vec<_>>();
        if !Self::check_function_args(config, signature, &args, symbols, span) {
            return Self::Poison;
        }
        Self::FuncCallE(FuncCall {
            callee: Some(Box::new(callee)),
            func: EnumBool::False(signature),
            args,
        })
    }

    pub(super) fn is_function_builtin(path: &TempPath) -> bool {
        path.segments == ["std", "function"]
    }
}
