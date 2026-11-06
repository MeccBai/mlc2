//! Lower compound assignment without evaluating its place expression twice.
use crate::parser::out::{Span, Spanned, TempExpr, TempPath, TempScope, TempStmt, TempVar};
use mlc_syntax::{language::ValueType, operators::Operator};

use super::Statement;
use crate::{
    ast::{
        config::Config,
        expression::{CompAtom, Expression},
        symbols::Resolution,
        types::{CompileType, base_type::DataType},
    },
    diagnostic::error::{CompileError, IllegalUseError},
};

pub(super) fn check(
    mut statement: Statement,
    config: &mut Config,
    symbols: &mut dyn Resolution,
    span: Span,
) -> Statement {
    if config.is_poisoned() {
        return Statement::Poison;
    }
    let Statement::AnonymousBlock(block) = &mut statement else {
        return statement;
    };
    // Debug builds interleave SourcePosition markers; find the synthesized assignment.
    let Some(Statement::Assignment(assignment)) = block
        .statements
        .iter_mut()
        .rev()
        .find(|statement| matches!(statement, Statement::Assignment(_)))
    else {
        return statement;
    };
    let target = assignment.variable.type_inference(config, symbols);
    if !matches!(symbols.get_type(target).unqualified(), CompileType::Base(base)
        if matches!(base.data_type(), DataType::Integer | DataType::Float))
    {
        config.submit_error(
            CompileError::IllegalUse(IllegalUseError::InvalidAssignment),
            span,
        );
        return Statement::Poison;
    }
    let Expression::CompositeE(composite, _) = &mut *assignment.value else {
        return statement;
    };
    let Some(rhs) = normalize_rhs(
        composite.members[1].clone().to_expression(),
        target,
        config,
        symbols,
        span,
    ) else {
        return Statement::Poison;
    };
    composite.members[1] = CompAtom::from_expr(rhs);
    statement
}

fn normalize_rhs(
    expression: Expression,
    target: crate::ast::TypeIndex,
    config: &mut Config,
    symbols: &mut dyn Resolution,
    span: Span,
) -> Option<Expression> {
    let expression = expression.const_fold(config, symbols);
    expression.check_constant_range(target, config, symbols, span);
    if config.is_poisoned() {
        return None;
    }
    let mut expression = expression.normalize_initializer(target, symbols);
    if let Expression::CompositeE(composite, _) = &mut expression {
        for member in &mut composite.members {
            *member = CompAtom::from_expr(normalize_rhs(
                member.clone().to_expression(),
                target,
                config,
                symbols,
                span,
            )?);
        }
    }
    let actual = expression.type_inference(config, symbols);
    if actual.is_empty()
        || symbols.get_type(target).unqualified() != symbols.get_type(actual).unqualified()
    {
        config.submit_error(
            CompileError::IllegalUse(IllegalUseError::TypeMismatched {
                expected: target.format(symbols),
                found: actual.format(symbols),
            }),
            span,
        );
        return None;
    }
    Some(expression)
}

pub(super) fn lower(
    target: Spanned<TempExpr>,
    op: Operator,
    value: Spanned<TempExpr>,
    span: Span,
) -> TempStmt {
    // '$' cannot occur in a source identifier, so this binding cannot shadow user names.
    let name = "$compound_place".to_owned();
    let place = (
        TempExpr::Path(TempPath {
            segments: vec![name.clone()],
        }),
        span,
    );
    let dereference = (
        TempExpr::Unary {
            op: Operator::Dereference,
            value: Box::new(place),
        },
        span,
    );
    TempStmt::Anonymous(TempScope {
        statements: vec![
            (
                TempStmt::Variable(TempVar {
                    value_type: ValueType::Final,
                    name,
                    name_span: span,
                    ty: None,
                    initializer: (
                        TempExpr::Unary {
                            op: Operator::MutOf,
                            value: Box::new(target),
                        },
                        span,
                    ),
                }),
                span,
            ),
            (
                TempStmt::Assignment {
                    target: dereference.clone(),
                    value: (
                        TempExpr::Binary {
                            operands: vec![dereference, (TempExpr::Group(Box::new(value)), span)],
                            operators: vec![op],
                        },
                        span,
                    ),
                },
                span,
            ),
        ],
    })
}
