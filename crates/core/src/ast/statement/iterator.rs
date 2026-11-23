use super::{ForStatement, Statement, Variable};
use crate::ast::{
    EnumBool, TypeIndex,
    config::Config,
    expression::{CompAtom, Expression, FuncCall, UnaryExpr, operators::Operator},
    generic::InterfaceRequire,
    symbols::{Resolution, StatementContext, SupperScopeType},
    types::{CompileType, base_type::DataType},
};
use crate::diagnostic::error::{CompileError, IllegalUseError};
use crate::parser::out::{Span, Spanned, TempExpr, TempForBinding, TempScope};
use std::{cell::Cell, rc::Rc};

impl Statement {
    pub(super) fn create_for(
        config: &mut Config,
        binding: Spanned<String>,
        mode: TempForBinding,
        iterator: Spanned<TempExpr>,
        scope: TempScope,
        symbols: &mut dyn Resolution,
        context: Option<&StatementContext>,
    ) -> Self {
        let span = iterator.1;
        let iterator = Expression::new(config, iterator, symbols, context);
        if config.is_poisoned() {
            return Self::Poison;
        }
        let ty = iterator.type_inference(config, symbols);
        let Some((owner, borrowed)) = iterator_owner(ty, symbols) else {
            return invalid_iterator(config, mode, "a mutable iterator is required", span);
        };
        let iterator = Rc::new(Variable {
            constant: false,
            read_count: Cell::new(3),
            declaration_span: span,
            name: format!("$for.{}.{}", binding.0, binding.1.start),
            var_type: ty.with_immutability(false, symbols),
            init_val: Box::new(iterator),
        });
        let mut calls = Vec::new();
        let mut methods = Vec::new();
        for (name, mutable_self) in [
            ("get_clone", false),
            ("has_next", false),
            ("to_next", true),
            ("get_ref", false),
        ] {
            let Some((index, generic)) = Expression::associated_candidate(owner, name, symbols)
            else {
                let reason = if name == "get_clone" {
                    "missing get_clone() -> T".into()
                } else {
                    format!("{name}: missing iterator method")
                };
                return invalid_iterator(config, mode, &reason, span);
            };
            let receiver = if borrowed {
                Expression::VarValueE(iterator.clone(), span)
            } else {
                Expression::UnaryExprE(
                    UnaryExpr::Operator {
                        op: if mutable_self {
                            Operator::MutOf
                        } else {
                            Operator::AddressOf
                        },
                        value: Box::new(CompAtom::VarValueA(iterator.clone(), span)),
                    },
                    span,
                )
            };
            let symbol = symbols.get_interface(index, generic);
            if !symbol.has_self || !symbol.params.is_empty() {
                return invalid_iterator(config, mode, &format!("{name}: invalid signature"), span);
            }
            let index = if generic {
                let Some(instance) = Expression::infer_interface(
                    config,
                    index,
                    std::slice::from_ref(&receiver),
                    span,
                    symbols,
                    context,
                ) else {
                    return Self::Poison;
                };
                instance
            } else {
                index
            };
            methods.push(index);
            calls.push(Expression::FuncCallE(
                FuncCall {
                    pipeline: false,
                    callee: None,
                    func: EnumBool::True(index),
                    args: vec![receiver],
                },
                span,
            ));
        }
        let [clone, condition, advance, borrow]: [Expression; 4] =
            calls.try_into().expect("four iterator methods");
        let element = clone.type_inference(config, symbols);
        if element.is_empty() {
            return invalid_iterator(config, mode, "missing get_clone() -> T", span);
        }
        let mutable = mode == TempForBinding::MutRef;
        let reference = element.make_ref(symbols, mutable);
        let mutable_reference = element.make_ref(symbols, true);
        let boolean = symbols.get_base(DataType::Boolean, 8, true);
        let returned_ref = borrow.type_inference(config, symbols);
        let required_ref = if !mutable && returned_ref == mutable_reference {
            mutable_reference
        } else {
            reference
        };
        for ((name, returned, mutable_self), index) in [
            ("get_clone", Some(element), false),
            ("has_next", Some(boolean), false),
            ("to_next", None, true),
            ("get_ref", Some(required_ref), false),
        ]
        .into_iter()
        .zip(methods)
        {
            let requirement = InterfaceRequire {
                name: name.into(),
                ret_type: returned,
                params: vec![],
                has_self: true,
                mutable: mutable_self,
                owner: false,
            };
            if let Some(reason) = requirement.signature_failure(index, name, symbols) {
                return invalid_iterator(config, mode, &format!("{name}: {reason}"), span);
            }
        }
        let (value, element_type) = if mode == TempForBinding::Clone {
            (clone, element)
        } else {
            (borrow, reference)
        };
        let element = Rc::new(Variable {
            constant: false,
            read_count: Default::default(),
            declaration_span: binding.1,
            name: binding.0,
            var_type: element_type,
            init_val: Box::new(value),
        });
        let mut child = super::control::child_context(context, SupperScopeType::For);
        if !child.declare(config, element.clone(), binding.1) {
            return Self::Poison;
        }
        let mut stmts = vec![Statement::VariableDecl(element)];
        stmts.extend(Self::parse_scope(config, scope, symbols, &mut child));
        Self::ForBlock(ForStatement {
            init: Some(Box::new(Statement::VariableDecl(iterator))),
            condition: Box::new(condition),
            execution: Some(Box::new(Statement::Expression(advance))),
            stmts,
        })
    }
}

fn iterator_owner(ty: TypeIndex, symbols: &dyn Resolution) -> Option<(TypeIndex, bool)> {
    if ty.is_empty() {
        return None;
    }
    match symbols.get_type(ty).unqualified() {
        CompileType::Ref(reference) if reference.level == 1 && reference.mut_base => {
            Some((reference.base, true))
        }
        CompileType::Ref(_) => None,
        _ => Some((ty, false)),
    }
}

fn invalid_iterator(
    config: &mut Config,
    mode: TempForBinding,
    reason: &str,
    span: Span,
) -> Statement {
    let requirement = if mode == TempForBinding::MutRef {
        "IteratorMut<T>"
    } else {
        "Iterator<T> or IteratorMut<T>"
    };
    config.submit_error(
        CompileError::IllegalUse(IllegalUseError::InvalidIterator {
            requirement,
            reason: reason.into(),
        }),
        span,
    );
    Statement::Poison
}
