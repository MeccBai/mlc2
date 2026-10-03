use crate::ast::symbols::Resolution;
mod builtins;
mod calls;
mod function_args;
mod generic_call;
mod lookup;
mod string;

use std::collections::HashMap;
use std::rc::Rc;

use super::operators::Operator;
use super::{
    Access, CompAtom, Composite, ConstValue, Expression, FuncCall, InitialList, UnaryExpr,
};
use crate::ast::symbol_name::SymbolName;
use crate::ast::types::CompileType;
use crate::ast::types::ListType;
use crate::ast::{
    EnumBool, SymbolTable, TypeIndex, config::Config, expression::Expression::InitListE,
    statement::Variable, types::resolve_type,
};
use crate::diagnostic::error::{CompileError, IllegalUseError, ResolveError};

use crate::parser::out::{
    Spanned, TempCallee, TempExpr,
    TempExpr::{Array, Binary, Call, Group, Init, Literal, Member, Path, Unary},
    TempLiteralKind,
    TempLiteralKind::{Boolean, Float, Integer, Null},
};

impl Expression {
    pub fn null() -> Self {
        Expression::ConstValueE(ConstValue::null())
    }

    pub fn new(
        config: &mut Config,
        temp_expr: Spanned<TempExpr>,
        symbols: &mut dyn Resolution,
        context: Option<&crate::ast::symbols::StatementContext>,
    ) -> Self {
        if config.is_poisoned() {
            return Self::Poison;
        }
        let expression = Self::create(config, temp_expr, symbols, context);
        if config.is_poisoned() {
            Self::Poison
        } else {
            expression
        }
    }

    fn create(
        config: &mut Config,
        temp_expr: Spanned<TempExpr>,
        symbols: &mut dyn Resolution,
        context: Option<&crate::ast::symbols::StatementContext>,
    ) -> Self {
        let (expr, span) = temp_expr;

        match expr {
            Literal { kind, text } => match kind {
                Integer | Float | Boolean | Null => {
                    Self::ConstValueE(ConstValue::new((kind, text), symbols))
                }
                TempLiteralKind::String => match string::decode(&text) {
                    Some(value) => Self::InitListE(InitialList::from_string(value)),
                    None => {
                        config.submit_error(
                            CompileError::IllegalUse(IllegalUseError::InvalidStringLiteral),
                            span,
                        );
                        Self::Poison
                    }
                },
            },
            Path(path) => {
                if path.segments.len() > 1 {
                    return match symbols.enum_value(config, &path) {
                        Ok(value) => Self::ConstValueE(ConstValue {
                            value: value.value.to_string(),
                            ty: value.enum_type,
                        }),
                        Err(error) => {
                            config.submit_error(CompileError::Resolve(error), span);
                            Self::Poison
                        }
                    };
                }
                use crate::ast::symbols::PathSymbol;
                let error = match symbols.resolve_path(config, &path, context) {
                    Some(PathSymbol::Variable(variable)) => {
                        variable.read_count.set(variable.read_count.get() + 1);
                        return Self::VarValueE(variable);
                    }
                    Some(PathSymbol::EnumValue(_)) => {
                        CompileError::IllegalUse(IllegalUseError::EnumValueRequiresPrefix)
                    }
                    Some(PathSymbol::GenericParameter(_)) => {
                        CompileError::IllegalUse(IllegalUseError::TypeUsedAsValue)
                    }
                    Some(PathSymbol::Interface { .. } | PathSymbol::Function { .. }) => {
                        CompileError::IllegalUse(IllegalUseError::UnsupportedSymbolValue)
                    }
                    None => CompileError::Resolve(ResolveError::UnknownVariable),
                };
                config.submit_error(error, span);
                Self::Poison
            }
            Unary { op, value } => {
                let expr = Self::new(config, *value, symbols, context);
                if expr.is_poisoned() {
                    return Self::Poison;
                }
                if op == Operator::MutOf && !expr.assignable(config, symbols) {
                    config.submit_error(
                        CompileError::IllegalUse(IllegalUseError::InvalidAssignment),
                        span,
                    );
                    return Self::null();
                }
                if op == Operator::Dereference {
                    let ty = expr.type_inference(config, symbols);
                    if !ty.is_empty() && !ty.is_ref(symbols) {
                        config.submit_error(
                            CompileError::IllegalUse(IllegalUseError::InvalidDereference),
                            span,
                        );
                        return Self::null();
                    }
                }
                let value = CompAtom::from_expr(expr);

                Expression::UnaryExprE(UnaryExpr::Operator {
                    op,
                    value: Box::new(value),
                })
            }
            Binary {
                mut operands,
                operators,
            } => {
                if operators == [Operator::Index] && operands.len() == 2 {
                    let index = Self::new(config, operands.pop().unwrap(), symbols, context);
                    let base = Self::new(config, operands.pop().unwrap(), symbols, context);
                    let base_type = base.type_inference(config, symbols);
                    let index_type = index.type_inference(config, symbols);
                    if (!base_type.is_empty()
                        && !matches!(
                            symbols.get_type(base_type).unqualified(),
                            CompileType::List(_)
                        ))
                        || (!index_type.is_empty() && !index_type.is_integer(symbols))
                    {
                        config.submit_error(
                            CompileError::IllegalUse(IllegalUseError::InvalidIndexAccess),
                            span,
                        );
                    }
                    return Self::UnaryExprE(UnaryExpr::Access(Access::Index {
                        base: Box::new(base),
                        index: Box::new(index),
                    }));
                }
                let expressions = operands
                    .into_iter()
                    .map(|operand| {
                        let expr = Self::new(config, operand, symbols, context);
                        CompAtom::from_expr(expr)
                    })
                    .collect::<Vec<_>>();

                if config.is_poisoned() {
                    return Self::Poison;
                }
                expressions.iter().for_each(|expr| {
                    let ty = expr.type_inference(config, symbols);
                    if !ty.is_empty() && ty.is_ref(symbols) {
                        config.submit_error(
                            CompileError::IllegalUse(IllegalUseError::CannotInferenceType),
                            span,
                        );
                    }
                });

                Expression::CompositeE(Composite {
                    members: expressions,
                    operators,
                })
            }
            Group(inner) => Expression::new(config, *inner, symbols, context),
            Call { callee, args } => match callee {
                TempCallee::GenericPath {
                    path,
                    args: generic_args,
                } => {
                    Self::new_generic_call(config, path, generic_args, args, span, symbols, context)
                }
                TempCallee::Expr(callee) => {
                    if matches!(callee.0, Member { .. }) {
                        Self::new_interface_call(config, *callee, args, span, symbols, context)
                    } else {
                        Self::new_function_call(config, *callee, args, span, symbols, context)
                    }
                }
            },
            Member {
                base,
                indirect,
                name,
                name_span,
            } => {
                let owner = Expression::new(config, *base, symbols, context);
                if owner.is_poisoned() {
                    return Self::Poison;
                }
                let receiver = owner.is_receiver(context);
                if receiver && !indirect {
                    config.submit_error(
                        CompileError::IllegalUse(IllegalUseError::MemberAccessViolation),
                        name_span,
                    );
                    return Self::Poison;
                }
                let mut owner_type = owner.type_inference(config, symbols);
                if !owner_type.is_empty() && indirect && !receiver {
                    if !owner_type.is_ref(symbols) {
                        config.submit_error(
                            CompileError::IllegalUse(IllegalUseError::MemberAccessViolation),
                            span,
                        );
                        return Self::null();
                    }
                    owner_type = owner_type.deref(symbols).unwrap();
                }
                if !owner_type.is_empty()
                    && !matches!(
                        symbols.get_type(owner_type).unqualified(),
                        CompileType::Unit(unit) if unit.get_member(&name).is_some_and(|member| receiver || member.public)
                    )
                {
                    config.submit_error(
                        CompileError::IllegalUse(IllegalUseError::MemberAccessViolation),
                        name_span,
                    );
                    return Self::null();
                }
                Self::UnaryExprE(UnaryExpr::Access(Access::Member {
                    base: Box::new(CompAtom::from_expr(owner)),
                    indirect,
                    name,
                }))
            }
            Init { target, values } => {
                let values = values
                    .into_iter()
                    .map(|value| {
                        let expr = Expression::new(config, value, symbols, context);
                        expr
                    })
                    .collect::<Vec<_>>();

                let ty = match target {
                    Some(t) => resolve_type(
                        config,
                        t,
                        symbols,
                        context.map(|context| context as &dyn crate::ast::types::TypeContext),
                    ),
                    None => None,
                };

                InitListE(InitialList::List {
                    onwer: ty,
                    values: values,
                })
            }
            Array(values) => {
                let values = values
                    .into_iter()
                    .map(|value| {
                        let expr = Expression::new(config, value, symbols, context);
                        expr
                    })
                    .collect::<Vec<_>>();

                let ty = if let Some(first) = values.first() {
                    let element_type = first
                        .type_inference(config, symbols)
                        .into(crate::ast::types::ValueType::Flex, symbols);
                    if element_type.is_empty() {
                        TypeIndex::empty()
                    } else {
                        let list = ListType::new(element_type, values.len());
                        let name = list.format(symbols);
                        symbols
                            .local_mut()
                            .types
                            .insert(name, CompileType::List(list))
                    }
                } else {
                    config.submit_error(CompileError::Resolve(ResolveError::MissingType), span);
                    TypeIndex::empty()
                };

                InitListE(InitialList::Array {
                    ty: ty,
                    values: values,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests;
