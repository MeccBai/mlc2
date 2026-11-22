use crate::ast::symbols::Resolution;

use super::{IfStatement, MatchPattern, MatchStatement, Statement, WhileStatement};
use crate::ast::{
    TypeIndex,
    arena::FuncIndex,
    config::Config,
    expression::{Expression, InitialList},
    symbols::{EnumBool, StatementContext, SymbolTable},
    types::{CompileType, ValueType},
};
use crate::diagnostic::error::{CompileError, IllegalUseError};
use crate::parser::out::{Span, Spanned, TempExpr, TempMatchPattern, TempScope};

fn matches_enum_value(pattern: &Expression, value: &Expression) -> bool {
    let pattern = match pattern {
        Expression::InitListE(InitialList::List { values, .. }, _) if values.len() == 1 => {
            &values[0]
        }
        _ => pattern,
    };
    match (pattern, value) {
        (Expression::ConstValueE(pattern, _), Expression::ConstValueE(value, _)) => {
            pattern.value == value.value
        }
        _ => pattern == value,
    }
}

pub(super) fn child_context(
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
    symbols: &mut dyn Resolution,
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
    symbols: &mut dyn Resolution,
    span: Span,
) -> bool {
    let expected = expected.with_immutability(false, symbols);
    let found = found.with_immutability(false, symbols);
    // Qualifier conversion can intern an imported type in the current arena.
    // Equal checked types need not have the same arena/index identity.
    if !expected.is_empty()
        && !found.is_empty()
        && (expected == found || symbols.get_type(expected) == symbols.get_type(found))
    {
        return true;
    }
    let format = |ty: TypeIndex| {
        if ty.is_empty() {
            "void".to_owned()
        } else {
            ty.format(symbols)
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
        symbols: &mut dyn Resolution,
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
        symbols: &mut dyn Resolution,
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
        symbols: &mut dyn Resolution,
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
                CompileError::IllegalUse(IllegalUseError::CannotInferType),
                span,
            );
            return Self::Poison;
        }
        let mut default_seen = false;
        if matches!(symbols.get_type(ty).unqualified(), crate::ast::types::CompileType::Unit(unit) if unit.variant.is_some())
        {
            config.submit_error(CompileError::IllegalUse(IllegalUseError::InvalidUnion { reason: "Union match requires a binding and a reference: match (y = reference); by-value match is not supported".into() }), span);
            return Self::Poison;
        }
        if matches!(
            symbols.get_type(ty).unqualified(),
            crate::ast::types::CompileType::Function(_)
        ) {
            config.submit_error(
                CompileError::IllegalUse(IllegalUseError::InvalidFunctionPointerOperation),
                span,
            );
            return Self::Poison;
        }
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
                    if !pattern.is_const(symbols) {
                        config.submit_error(
                            CompileError::IllegalUse(IllegalUseError::NonConstantMatchCase),
                            span,
                        );
                        return Self::Poison;
                    }
                    MatchPattern::Value(pattern.const_fold(config, symbols))
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
        if !default_seen {
            let error = match symbols.get_type(ty).unqualified() {
                CompileType::Enum(enumeration) => {
                    let missing = enumeration.variants.iter().zip(&enumeration.values)
                        .filter(|(_, value)| !parsed.iter().any(|(pattern, _)| {
                            matches!(pattern, MatchPattern::Value(pattern) if matches_enum_value(pattern, value))
                        }))
                        .map(|(name, _)| name.as_str())
                        .collect::<Vec<_>>();
                    (!missing.is_empty()).then(|| IllegalUseError::NonExhaustiveEnumMatch {
                        missing: missing.join(", "),
                    })
                }
                _ => Some(IllegalUseError::MissingDefaultBranch),
            };
            if let Some(error) = error {
                config.submit_error(CompileError::IllegalUse(error), span);
                return Self::Poison;
            }
        }
        Self::MatchBlock(MatchStatement {
            value: Box::new(value),
            branches: parsed,
        })
    }
}

#[cfg(test)]
mod tests;
