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
            return Self::Poison(span);
        }
        // Type-only intrinsic: resolve layout now and leave no callable symbol in the AST.
        if kind == Builtin::Size {
            if target.is_empty() || symbols.get_type(target).is_generic(symbols) {
                return Self::invalid_builtin(config, definition.name, span);
            }
            let value = target.size(symbols).to_string();
            let ty = symbols.get_base(DataType::Integer, config.ptr_size * 8, false);
            return Self::ConstValueE(crate::ast::expression::ConstValue { ty, value }, span);
        }
        if let Some(op) = kind.bit_operator() {
            use crate::ast::expression::{CompAtom, Composite, UnaryExpr};
            let values: Vec<_> = args
                .into_iter()
                .map(|arg| Self::new(config, arg, symbols, context).const_fold(config, symbols))
                .collect();
            if config.is_poisoned() {
                return Self::Poison(span);
            }
            let mut bits = None;
            for value in &values {
                let ty = value.type_inference(config, symbols);
                if ty.is_empty() {
                    return Self::invalid_builtin(config, definition.name, span);
                }
                let CompileType::Base(base) = symbols.get_type(ty).unqualified() else {
                    return Self::invalid_builtin(config, definition.name, span);
                };
                if bits.is_some_and(|bits| bits != base.bits()) {
                    return Self::invalid_builtin(config, definition.name, span);
                }
                bits = Some(base.bits());
            }
            let mut members: Vec<_> = values
                .into_iter()
                .map(|value| {
                    CompAtom::UnaryExprA(
                        UnaryExpr::Operator {
                            op: crate::ast::expression::operators::Operator::BitPattern,
                            value: Box::new(CompAtom::from_expr(value)),
                        },
                        span,
                    )
                })
                .collect();
            return if kind == Builtin::BitNot {
                Self::UnaryExprE(
                    UnaryExpr::Operator {
                        op,
                        value: Box::new(members.remove(0)),
                    },
                    span,
                )
            } else {
                Self::CompositeE(
                    Composite {
                        members,
                        operators: vec![op],
                    },
                    span,
                )
            };
        }
        if kind == Builtin::Todo {
            let ty = symbols.get_base(DataType::Integer, 32, true);
            let source = std::fs::read_to_string(config.current_file()).unwrap_or_default();
            let lines = crate::lexer::LineTable::new(&source);
            let position = lines.span_start(span);
            let location = position
                .map(|p| format!("{}:{}:{}", config.current_file(), p.line, p.column))
                .unwrap_or_else(|| config.current_file().clone());
            let index = symbols.local_mut().functions.insert(
                format!(
                    "__mlc_todo_{:?}_{}",
                    context.map(|c| c.belong()),
                    span.start
                ),
                FuncSymbol {
                    constant: false,
                    span,
                    name: format!("__mlc_todo_{}", span.start),
                    params: vec![],
                    ret_type: Some(ty),
                    generics: vec![],
                    generic_map: HashMap::new(),
                    attributes: HashSet::from([FuncAttibute::Builtin(kind)]),
                    exported: false,
                },
            );
            return Self::FuncCallE(
                FuncCall {
                    pipeline: false,
                    callee: None,
                    func: EnumBool::False(index),
                    args: vec![Self::InitListE(
                        InitialList::from_string(format!("{location}: not implemented (todo)")),
                        span,
                    )],
                },
                span,
            );
        }
        let target = if kind == Builtin::ConstCStr {
            symbols.get_base(DataType::Integer, 8, true)
        } else if kind == Builtin::StrLen {
            symbols.get_base(DataType::Integer, 64, false)
        } else {
            target
        };
        let args: Vec<_> = args
            .into_iter()
            .map(|arg| Expression::new(config, arg, symbols, context).const_fold(config, symbols))
            .collect();
        if config.is_poisoned() {
            return Self::Poison(span);
        }
        let source = args[0].type_inference(config, symbols);
        if source.is_empty() {
            return Self::invalid_builtin(config, definition.name, span);
        }
        let valid = match kind {
            Builtin::Todo
            | Builtin::Size
            | Builtin::BitAnd
            | Builtin::BitXor
            | Builtin::BitOr
            | Builtin::BitNot
            | Builtin::BitShl
            | Builtin::BitShr => unreachable!(),
            Builtin::ConstCStr => {
                matches!(symbols.get_type(source).unqualified(), CompileType::Str)
            }
            Builtin::StrLen => matches!(symbols.get_type(source).unqualified(), CompileType::Str),
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
                return Self::Poison(span);
            }
            let size_ty = symbols.get_base(DataType::Integer, config.ptr_size * 8, false);
            args[0].check_constant_range(size_ty, config, symbols, span);
            if config.is_poisoned() {
                return Self::Poison(span);
            }
        }
        let result = match kind {
            Builtin::Todo
            | Builtin::Size
            | Builtin::BitAnd
            | Builtin::BitXor
            | Builtin::BitOr
            | Builtin::BitNot
            | Builtin::BitShl
            | Builtin::BitShr => unreachable!(),
            Builtin::StrLen => Some(symbols.get_base(DataType::Integer, 64, false)),
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
            Builtin::Cast => Some(target.with_immutability(false, symbols)),
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
                constant: false,
                span: span,
                name,
                params: vec![(source, "input".into())],
                ret_type: result,
                generics: Vec::new(),
                generic_map: HashMap::new(),
                attributes: HashSet::from([FuncAttibute::Builtin(kind)]),
                exported: false,
            },
        );
        Self::FuncCallE(
            FuncCall {
                pipeline: false,
                callee: None,
                func: EnumBool::False(index),
                args,
            },
            span,
        )
    }

    fn invalid_builtin(config: &mut Config, name: &str, span: Span) -> Self {
        config.submit_error(
            CompileError::IllegalUse(IllegalUseError::InvalidBuiltinArgument { name: name.into() }),
            span,
        );
        Self::Poison(span)
    }
}
