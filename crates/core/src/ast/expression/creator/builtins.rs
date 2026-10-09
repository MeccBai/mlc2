use crate::ast::{
    TypeIndex,
    attribute::FuncAttibute,
    builtins::Builtin,
    config::Config,
    expression::{Expression, FuncCall, InitialList},
    function::FuncSymbol,
    symbols::{EnumBool, Resolution, StatementContext},
    types::{CompileType, ValueType, base_type::DataType},
};
use crate::diagnostic::error::{CompileError, IllegalUseError};
use crate::parser::out::{Span, Spanned, TempExpr};
use std::collections::{HashMap, HashSet};

impl Expression {
    pub(super) fn new_builtin_call(
        config: &mut Config,
        kind: Builtin,
        target: TypeIndex,
        args: Vec<Spanned<TempExpr>>,
        span: Span,
        symbols: &mut dyn Resolution,
        context: Option<&StatementContext>,
    ) -> Self {
        let definition = crate::ast::builtins::FUNCTIONS
            .iter()
            .find(|d| d.kind == kind)
            .unwrap();
        if args.len() != definition.arguments {
            config.submit_error(
                CompileError::IllegalUse(IllegalUseError::ArgumentCountMismatch),
                span,
            );
            return Self::Poison;
        }
        let target = if kind == Builtin::ConstCStr {
            symbols.get_base(DataType::Integer, 8, true)
        } else {
            target
        };
        let args: Vec<_> = args
            .into_iter()
            .map(|arg| Expression::new(config, arg, symbols, context).const_fold(config, symbols))
            .collect();
        if config.is_poisoned() {
            return Self::Poison;
        }
        let source = args[0].type_inference(config, symbols);
        if source.is_empty() {
            return Self::invalid_builtin(config, definition.name, span);
        }
        let valid = match kind {
            Builtin::ConstCStr => {
                matches!(&args[0], Expression::InitListE(InitialList::String { .. }))
            }
            Builtin::Alloc => source.is_integer(symbols),
            Builtin::Cast => matches!(
                (
                    symbols.get_type(source).unqualified(),
                    symbols.get_type(target).unqualified()
                ),
                (CompileType::Base(_), CompileType::Base(_))
                    | (CompileType::Ref(_), CompileType::Ref(_))
            ),
            Builtin::Dealloc => match symbols.get_type(source).unqualified() {
                CompileType::Ref(reference) => {
                    reference.ownership
                        && reference.level == 1
                        && target.type_check(false, &reference.base, symbols)
                }
                _ => false,
            },
        };
        if !valid {
            return Self::invalid_builtin(config, definition.name, span);
        }
        if kind == Builtin::Alloc {
            if target.has_res(symbols) {
                config.submit_error(
                    CompileError::IllegalUse(IllegalUseError::ResourceAggregateUnsupported),
                    span,
                );
                return Self::Poison;
            }
            let size_ty = symbols.get_base(DataType::Integer, 64, false);
            args[0].check_constant_range(size_ty, config, symbols, span);
            if config.is_poisoned() {
                return Self::Poison;
            }
        }
        let result = match kind {
            Builtin::ConstCStr => Some(
                symbols
                    .get_base(DataType::Integer, 8, true)
                    .make_ref(symbols, false),
            ),
            Builtin::Alloc => {
                let reference = crate::ast::types::RefType::new(target, 1, true).owned();
                let name = reference.format(symbols);
                Some(
                    symbols
                        .local_mut()
                        .types
                        .insert(name, CompileType::Ref(reference)),
                )
            }
            Builtin::Cast => Some(target.into(ValueType::Flex, symbols)),
            Builtin::Dealloc => None,
        };
        let name = crate::ast::symbol_name::SymbolName::builtin(
            definition.name,
            &target.format(symbols),
            &source.format(symbols),
        );
        let index = symbols.local_mut().functions.insert(
            name.clone(),
            FuncSymbol {
                name,
                params: vec![(source, "input".into())],
                ret_type: result,
                generics: Vec::new(),
                generic_map: HashMap::new(),
                attributes: HashSet::from([FuncAttibute::Builtin(kind)]),
                exported: false,
            },
        );
        Self::FuncCallE(FuncCall {
            callee: None,
            func: EnumBool::False(index),
            args,
        })
    }

    fn invalid_builtin(config: &mut Config, name: &str, span: Span) -> Self {
        config.submit_error(
            CompileError::IllegalUse(IllegalUseError::InvalidBuiltinArgument { name: name.into() }),
            span,
        );
        Self::Poison
    }
}
