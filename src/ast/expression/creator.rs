#[cfg(test)]
mod call_tests;
mod generic_call;
mod lookup;
#[cfg(test)]
mod reference_tests;

use std::collections::HashMap;
use std::rc::Rc;

use super::operators::Operator;
use super::{
    CompAtom, Composite, ConstValue, Expression, FuncCall, InitialList, MemberAccess, UnaryExpr,
};
use crate::ast::symbol_name::SymbolName;
use crate::ast::types::CompileType;
use crate::ast::{
    EnumBool, SymbolTable, TypeIndex, config::Config, expression::Expression::InitListE,
    statement::Variable, types::resolve_type,
};
use crate::error::{CompileError, IllegalUseError, ResolveError};

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
            Call { callee, args } => match callee {
                TempCallee::GenericPath {
                    path,
                    args: generic_args,
                } => {
                    Self::new_generic_call(config, path, generic_args, args, span, symbols, context)
                }
                TempCallee::Expr(callee) => {
                    if matches!(callee.0, Member { .. }) {
                        let (owner, interface) =
                            match Self::search_interface(config, *callee, symbols, context) {
                                Some((owner, interface)) => (owner, interface),
                                None => {
                                    config.submit_error(
                                        CompileError::Resolve(ResolveError::UnknownInterface),
                                        span,
                                    );
                                    return Self::null();
                                }
                            };
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
                        let function =
                            match Self::search_function(config, *callee, symbols, context) {
                                Some(f) => f,
                                None => {
                                    config.submit_error(
                                        CompileError::Resolve(ResolveError::UnknownFunction),
                                        span,
                                    );
                                    return Self::null();
                                }
                            };
                        Self::FuncCallE(FuncCall {
                            func: function,
                            args: args
                                .into_iter()
                                .map(|arg| Expression::new(config, arg, symbols, context))
                                .collect::<Vec<_>>(),
                        })
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
    use crate::error::ResolveError::UnknownFunction;
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

    #[test]
    fn unknown_generic_callee_reports_unknown_function() {
        let (mut config, mut symbols) = setup();
        let span = (0..8).into();
        let callee = TempCallee::GenericPath {
            path: crate::parser::out::TempPath {
                segments: vec!["identity".into()],
            },
            args: vec![(
                crate::parser::out::TempType::Path(crate::parser::out::TempPath {
                    segments: vec!["i32".into()],
                }),
                span,
            )],
        };
        Expression::new(
            &mut config,
            (
                Call {
                    callee,
                    args: Vec::new(),
                },
                span,
            ),
            &mut symbols,
            None,
        );
        assert!(config.error_handle().errors.contains(&ErrorInfo::new(
            CompileError::Resolve(UnknownFunction),
            span,
        )));
    }

    #[test]
    fn registered_generic_function_is_found_in_generic_table() {
        let (mut config, mut symbols) = setup();
        let span = (0..8).into();
        let requirement = symbols.generics.requires.insert(
            "any".into(),
            crate::ast::generic::GenericRequire::empty("any".into()),
        );
        let index = symbols.generics.functions.insert(
            "identity".into(),
            crate::ast::function::FuncSymbol {
                name: "identity".into(),
                params: Vec::new(),
                ret_type: None,
                generics: vec!["T".into()],
                generic_map: HashMap::from([("T".into(), requirement)]),
                attributes: Vec::new(),
                exported: false,
            },
        );
        let result = Expression::search_generic_function(
            &mut config,
            (
                Path(crate::parser::out::TempPath {
                    segments: vec!["identity".into()],
                }),
                span,
            ),
            &mut symbols,
            None,
        );
        assert_eq!(result, Some(EnumBool::False(index)));
        assert!(config.error_handle().errors.is_empty());
    }
}
