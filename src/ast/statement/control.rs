use std::rc::Rc;

use super::{
    Assignment, ForStatement, IfStatement, MatchPattern, MatchStatement, Statement, Variable,
    WhileStatement,
};
use crate::ast::{
    TypeIndex,
    arena::FuncIndex,
    config::Config,
    expression::{CompAtom, Composite, ConstValue, Expression, operators::Operator},
    symbols::{EnumBool, StatementContext, SymbolTable},
    types::ValueType,
};
use crate::error::{CompileError, IllegalUseError};
use crate::parser::out::{Span, Spanned, TempExpr, TempMatchPattern, TempScope};

fn child_context(
    context: Option<&StatementContext>,
    kind: crate::ast::symbols::SupperScopeType,
) -> StatementContext {
    let mut child = context
        .cloned()
        .unwrap_or_else(|| StatementContext::new(EnumBool::False(FuncIndex::empty())));
    child.push_scope(kind);
    child
}

fn condition(
    config: &mut Config,
    prototype: Spanned<TempExpr>,
    symbols: &mut SymbolTable,
    context: Option<&StatementContext>,
) -> Option<Expression> {
    let span = prototype.1;
    let expression = Expression::new(config, prototype, symbols, context);
    if config.is_poisoned() {
        return None;
    }
    let ty = expression.type_inference(config, symbols);
    if ty.is_empty() || !expression.is_condition(config, symbols) {
        config.submit_error(
            CompileError::IllegalUse(IllegalUseError::ExpressionMustConditional),
            span,
        );
        return None;
    }
    Some(expression)
}

fn same_type(
    config: &mut Config,
    expected: TypeIndex,
    found: TypeIndex,
    symbols: &mut SymbolTable,
    span: Span,
) -> bool {
    let expected = expected.into_value_type(ValueType::Flex, &mut symbols.types);
    let found = found.into_value_type(ValueType::Flex, &mut symbols.types);
    if !expected.is_empty() && expected == found {
        return true;
    }
    let format = |ty: TypeIndex| {
        if ty.is_empty() {
            "void".to_owned()
        } else {
            ty.format(&symbols.types)
        }
    };
    config.submit_error(
        CompileError::IllegalUse(IllegalUseError::TypeMismatched {
            expected: format(expected),
            found: format(found),
        }),
        span,
    );
    false
}

impl Statement {
    pub(super) fn create_if(
        config: &mut Config,
        prototype: Spanned<TempExpr>,
        then_scope: TempScope,
        else_scope: Option<TempScope>,
        symbols: &mut SymbolTable,
        context: Option<&StatementContext>,
    ) -> Self {
        let Some(condition) = condition(config, prototype, symbols, context) else {
            return Self::Poison;
        };
        let then_branch = Self::parse_scope(
            config,
            then_scope,
            symbols,
            &mut child_context(context, crate::ast::symbols::SupperScopeType::If),
        );
        if config.is_poisoned() {
            return Self::Poison;
        }
        let else_branch = else_scope.map(|scope| {
            Self::parse_scope(
                config,
                scope,
                symbols,
                &mut child_context(context, crate::ast::symbols::SupperScopeType::If),
            )
        });
        Self::IfBlock(IfStatement {
            condition: Box::new(condition),
            then_branch,
            else_branch,
        })
    }

    pub(super) fn create_while(
        config: &mut Config,
        prototype: Spanned<TempExpr>,
        scope: TempScope,
        symbols: &mut SymbolTable,
        context: Option<&StatementContext>,
    ) -> Self {
        let Some(condition) = condition(config, prototype, symbols, context) else {
            return Self::Poison;
        };
        let stmts = Self::parse_scope(
            config,
            scope,
            symbols,
            &mut child_context(context, crate::ast::symbols::SupperScopeType::While),
        );
        Self::WhileBlock(WhileStatement {
            condition: Box::new(condition),
            stmts,
        })
    }

    pub(super) fn create_match(
        config: &mut Config,
        prototype: Spanned<TempExpr>,
        branches: Vec<(TempMatchPattern, TempScope)>,
        symbols: &mut SymbolTable,
        context: Option<&StatementContext>,
    ) -> Self {
        let span = prototype.1;
        let value = Expression::new(config, prototype, symbols, context);
        if config.is_poisoned() {
            return Self::Poison;
        }
        let ty = value.type_inference(config, symbols);
        if ty.is_empty() {
            config.submit_error(
                CompileError::IllegalUse(IllegalUseError::CannotInferenceType),
                span,
            );
            return Self::Poison;
        }
        let mut default_seen = false;
        let mut parsed = Vec::new();
        for (pattern, scope) in branches {
            let pattern = match pattern {
                TempMatchPattern::Default => {
                    if default_seen {
                        config.submit_error(
                            CompileError::IllegalUse(IllegalUseError::DuplicateDefaultBranch),
                            span,
                        );
                        return Self::Poison;
                    }
                    default_seen = true;
                    MatchPattern::Default
                }
                TempMatchPattern::Expression(prototype) => {
                    let span = prototype.1;
                    let pattern = Expression::new(config, prototype, symbols, context);
                    if config.is_poisoned() {
                        return Self::Poison;
                    }
                    let branch_ty = pattern.type_inference(config, symbols);
                    if !same_type(config, ty, branch_ty, symbols, span) {
                        return Self::Poison;
                    }
                    MatchPattern::Value(pattern)
                }
            };
            let body = Self::parse_scope(
                config,
                scope,
                symbols,
                &mut child_context(context, crate::ast::symbols::SupperScopeType::Match),
            );
            if config.is_poisoned() {
                return Self::Poison;
            }
            parsed.push((pattern, body));
        }
        Self::MatchBlock(MatchStatement {
            value: Box::new(value),
            branches: parsed,
        })
    }

    pub(super) fn create_for(
        config: &mut Config,
        binding: Spanned<String>,
        start: Spanned<TempExpr>,
        end: Spanned<TempExpr>,
        scope: TempScope,
        symbols: &mut SymbolTable,
        context: Option<&StatementContext>,
    ) -> Self {
        let start_span = start.1;
        let end_span = end.1;
        let start = Expression::new(config, start, symbols, context);
        if config.is_poisoned() {
            return Self::Poison;
        }
        let end = Expression::new(config, end, symbols, context);
        if config.is_poisoned() {
            return Self::Poison;
        }
        let start_ty = start.type_inference(config, symbols);
        let end_ty = end.type_inference(config, symbols);
        for (ty, span) in [(start_ty, start_span), (end_ty, end_span)] {
            if ty.is_empty() || !ty.is_integer(&symbols.types) {
                config.submit_error(
                    CompileError::IllegalUse(IllegalUseError::ForBoundMustBeInteger),
                    span,
                );
                return Self::Poison;
            }
        }
        if !same_type(config, start_ty, end_ty, symbols, end_span) {
            return Self::Poison;
        }
        let var_type = start_ty.into_value_type(ValueType::Flex, &mut symbols.types);
        let variable = Rc::new(Variable {
            name: binding.0,
            var_type,
            init_val: Box::new(start),
        });
        let mut child = child_context(context, crate::ast::symbols::SupperScopeType::For);
        if !child.declare(config, variable.clone(), binding.1) {
            return Self::Poison;
        }
        let condition = Expression::CompositeE(Composite {
            members: vec![
                CompAtom::VarValueA(variable.clone()),
                CompAtom::from_expr(end),
            ],
            operators: vec![Operator::Less],
        });
        let increment = Expression::CompositeE(Composite {
            members: vec![
                CompAtom::VarValueA(variable.clone()),
                CompAtom::ConstValueA(ConstValue {
                    value: "1".into(),
                    ty: var_type,
                }),
            ],
            operators: vec![Operator::Add],
        });
        let execution = Statement::Assignment(Assignment {
            variable: Box::new(Expression::VarValueE(variable.clone())),
            value: Box::new(increment),
        });
        let stmts = Self::parse_scope(config, scope, symbols, &mut child);
        Self::ForBlock(ForStatement {
            init: Some(Box::new(Statement::VariableDecl(variable))),
            condition: Box::new(condition),
            execution: Some(Box::new(execution)),
            stmts,
        })
    }
}

#[cfg(test)]
mod tests;
