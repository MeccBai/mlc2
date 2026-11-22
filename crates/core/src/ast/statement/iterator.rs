use super::{ForStatement, Statement, Variable};
use crate::ast::{
    EnumBool, TypeIndex,
    config::Config,
    expression::{CompAtom, Expression, FuncCall, UnaryExpr, operators::Operator},
    generic::InterfaceRequire,
    symbol_name::SymbolName,
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
        let owner_name = symbols.get_type(owner).unqualified().format(symbols);
        let Some(element) = symbols
            .associated_interface(owner, &SymbolName::callable(Some(&owner_name), "get_clone"))
            .and_then(|index| symbols.get_interface_regular(index).ret_type)
        else {
            return invalid_iterator(config, mode, "missing get_clone() -> T", span);
        };

        let mutable = mode == TempForBinding::MutRef;
        let reference = element.make_ref(symbols, mutable);
        let boolean = symbols.get_base(DataType::Boolean, 8, true);
        let iterator = Rc::new(Variable {
            constant: false,
            read_count: Cell::new(3),
            declaration_span: span,
            name: format!("$for.{}.{}", binding.0, binding.1.start),
            var_type: ty.with_immutability(false, symbols),
            init_val: Box::new(iterator),
        });
        let mut calls = Vec::new();
        for (name, returned, mutable_self) in [
            ("has_next", Some(boolean), false),
            ("to_next", None, true),
            ("get_ref", Some(reference), false),
            ("get_clone", Some(element), false),
        ] {
            let requirement = InterfaceRequire {
                name: name.into(),
                ret_type: returned,
                params: vec![],
                has_self: true,
                mutable: mutable_self,
                owner: false,
            };
            if let Some(reason) = requirement.failure(owner, symbols) {
                return invalid_iterator(config, mode, &format!("{name}: {reason}"), span);
            }
            let index = symbols
                .associated_interface(owner, &SymbolName::callable(Some(&owner_name), name))
                .expect("validated iterator interface");
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
        let [condition, advance, borrow, clone]: [Expression; 4] =
            calls.try_into().expect("four iterator methods");
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
