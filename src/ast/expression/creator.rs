use std::collections::HashMap;
use std::rc::Rc;

use super::operators::Operator;
use super::{
    CompAtom, Composite, ConstValue, Expression, FuncCall, InitialList, MemberAccess, UnaryExpr,
};
use crate::ast::symbol_name::SymbolName;
use crate::ast::types::CompileType;
use crate::ast::{
    EnumBool, FuncIndex, SymbolTable, TypeIndex, arena::InterfaceIndex, config::Config,
    expression::Expression::InitListE, statement::Variable, types::resolve_type,
};
use crate::error::{CompileError, IllegalUseError, ResolveError, ice::ice};
use crate::parser::out::{
    Spanned, TempExpr,
    TempExpr::{Array, Binary, Call, Group, Init, Literal, Member, Path, Unary},
    TempLiteralKind,
    TempLiteralKind::{Boolean, Float, Integer, Null},
};

impl Expression {
    pub fn null() -> Self {
        Expression::ConstValueE(ConstValue::null())
    }

    pub fn search_variable(
        name: &String,
        symbols: &SymbolTable,
        context: &Option<&HashMap<String, Rc<Variable>>>,
    ) -> Option<Rc<Variable>> {
        if let Some(ctx) = context {
            match ctx.get(name) {
                Some(var) => return Some(Rc::clone(var)),
                None => {}
            }
        }

        if let Some(var) = symbols.globals.get(name) {
            return Some(Rc::clone(var));
        }

        None
    }

    pub fn search_interface(
        config: &mut Config,
        temp_expr: Spanned<TempExpr>,
        symbols: &mut SymbolTable,
        context: Option<&HashMap<String, Rc<Variable>>>,
    ) -> (Expression, InterfaceIndex) {
        let (expr, span) = temp_expr;
        match expr {
            Member {
                base,
                indirect,
                name,
                ..
            } => {
                let owner = Expression::new(config, *base, symbols, context);
                let owner_type = owner.type_inference(config, symbols);
                let owner_type = if indirect && !owner_type.is_empty() {
                    if !owner_type.is_ref(&symbols.types) {
                        config.submit_error(
                            CompileError::IllegalUse(IllegalUseError::MemberAccessViolation),
                            span,
                        );
                        return (owner, InterfaceIndex::empty());
                    }
                    owner_type.deref(&mut symbols.types).unwrap()
                } else {
                    owner_type
                };
                if owner_type.is_empty() {
                    return (owner, InterfaceIndex::empty());
                }
                let interface_name = SymbolName::member(&owner_type.format(&symbols.types), &name);
                (
                    owner,
                    symbols
                        .interfaces
                        .get_by_name(&interface_name)
                        .unwrap_or_else(|| {
                            config.submit_error(
                                CompileError::Resolve(ResolveError::UnknownInterface),
                                span,
                            );
                            InterfaceIndex::empty()
                        }),
                )
            }
            _ => ice("Interface search is not implemented yet."),
        }
    }

    pub fn search_function(
        config: &mut Config,
        temp_expr: Spanned<TempExpr>,
        symbols: &mut SymbolTable,
        context: Option<&HashMap<String, Rc<Variable>>>,
    ) -> EnumBool<InterfaceIndex, FuncIndex> {
        let (expr, span) = temp_expr;
        match expr {
            Path(path) => {
                let name = SymbolName::path(&path.segments);
                let interface = symbols.interfaces.get_by_name(&name);
                match interface {
                    Some(interface_index) => EnumBool::True(interface_index),
                    None => {
                        let func = symbols.functions.get_by_name(&name);
                        match func {
                            Some(func_index) => EnumBool::False(func_index),
                            None => {
                                config.submit_error(
                                    CompileError::Resolve(ResolveError::UnknownFunction),
                                    span,
                                );
                                EnumBool::False(FuncIndex::empty())
                            }
                        }
                    }
                }
            }
            _ => ice("Interface search is not implemented yet."),
        }
    }

    pub fn new(
        config: &mut Config,
        temp_expr: Spanned<TempExpr>,
        symbols: &mut SymbolTable,
        context: Option<&HashMap<String, Rc<Variable>>>,
    ) -> Self {
        let (expr, span) = temp_expr;

        match expr {
            Literal { kind, text } => match kind {
                Integer | Float | Boolean | Null => {
                    Self::ConstValueE(ConstValue::new((kind, text), symbols))
                }
                TempLiteralKind::String => Self::InitListE(InitialList::from_string(text)),
            },
            Path(path) => {
                let var_name = SymbolName::path(&path.segments);
                match Self::search_variable(&var_name, symbols, &context) {
                    Some(var) => Self::VarValueE(var),
                    None => {
                        config.submit_error(
                            CompileError::Resolve(ResolveError::UnknownVariable),
                            span,
                        );
                        Self::VarValueE(Variable::empty())
                    }
                }
            }
            Unary { op, value } => {
                let expr = Self::new(config, *value, symbols, context);
                if op == Operator::Dereference {
                    let ty = expr.type_inference(config, symbols);
                    if !ty.is_empty() && !ty.is_ref(&symbols.types) {
                        config.submit_error(
                            CompileError::IllegalUse(IllegalUseError::InvalidDereference),
                            span,
                        );
                        return Self::null();
                    }
                }
                Expression::UnaryExprE(UnaryExpr {
                    op,
                    value: Box::new(CompAtom::from_expr(expr)),
                })
            }
            Binary {
                operands,
                operators,
            } => {
                let expressions = operands
                    .into_iter()
                    .map(|operand| {
                        let expr = Self::new(config, operand, symbols, context);
                        CompAtom::from_expr(expr)
                    })
                    .collect::<Vec<_>>();

                Expression::CompositeE(Composite {
                    members: expressions,
                    operators,
                })
            }
            Group(inner) => Expression::new(config, *inner, symbols, context),
            Call { callee, args } => {
                if let Member { .. } = callee.0 {
                    let (owner, interface) =
                        Self::search_interface(config, *callee, symbols, context);
                    let mut params = vec![owner];
                    params.reserve(args.len() + 1);
                    args.into_iter().for_each(|arg| {
                        params.push(Expression::new(config, arg, symbols, context));
                    });
                    Self::FuncCallE(FuncCall {
                        func: EnumBool::True(interface),
                        args: params,
                    })
                } else {
                    let function = Self::search_function(config, *callee, symbols, context);
                    Self::FuncCallE(FuncCall {
                        func: function,
                        args: args
                            .into_iter()
                            .map(|arg| Expression::new(config, arg, symbols, context))
                            .collect::<Vec<_>>(),
                    })
                }
            }
            Member {
                base,
                indirect,
                name,
                name_span,
            } => {
                let owner = Expression::new(config, *base, symbols, context);
                let mut owner_type = owner.type_inference(config, symbols);
                if !owner_type.is_empty() && indirect {
                    if !owner_type.is_ref(&symbols.types) {
                        config.submit_error(
                            CompileError::IllegalUse(IllegalUseError::MemberAccessViolation),
                            span,
                        );
                        return Self::null();
                    }
                    owner_type = owner_type.deref(&mut symbols.types).unwrap();
                }
                if !owner_type.is_empty()
                    && !matches!(
                        symbols.types.get(owner_type),
                        CompileType::Unit(unit) if unit.get_member(&name).is_some()
                    )
                {
                    config.submit_error(
                        CompileError::IllegalUse(IllegalUseError::MemberAccessViolation),
                        name_span,
                    );
                    return Self::null();
                }
                Self::MemberAccessE(MemberAccess {
                    base: Box::new(CompAtom::from_expr(owner)),
                    indirect,
                    name,
                })
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
                    Some(t) => resolve_type(config, t, symbols),
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
                    first.type_inference(config, symbols)
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
mod tests {
    use super::*;
    use crate::error::{ErrorHandle, ErrorInfo};
    use crate::parser::out::TempLiteralKind;

    fn setup() -> (Config, SymbolTable) {
        let config = Config::new(
            Vec::new(),
            "test".into(),
            "test".into(),
            ErrorHandle::new("test".into()),
        );
        (config, SymbolTable::new())
    }

    #[test]
    fn empty_array_reports_missing_type() {
        let (mut config, mut symbols) = setup();
        let span = (0..2).into();
        Expression::new(&mut config, (Array(Vec::new()), span), &mut symbols, None);
        assert!(config.error_handle().errors.contains(&ErrorInfo::new(
            CompileError::Resolve(ResolveError::MissingType),
            span,
        )));
    }

    #[test]
    fn dereferencing_non_reference_reports_error() {
        let (mut config, mut symbols) = setup();
        let span = (0..2).into();
        let value = (
            Literal {
                kind: TempLiteralKind::Integer,
                text: "1".into(),
            },
            (1..2).into(),
        );
        Expression::new(
            &mut config,
            (
                Unary {
                    op: Operator::Dereference,
                    value: Box::new(value),
                },
                span,
            ),
            &mut symbols,
            None,
        );
        assert!(config.error_handle().errors.contains(&ErrorInfo::new(
            CompileError::IllegalUse(IllegalUseError::InvalidDereference),
            span,
        )));
    }
}
