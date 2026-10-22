//! Unified semantic inference. Writable sessions may intern derived types;
//! read-only sessions evaluate constants without requiring mutable arena access.
use crate::ast::{
    TypeIndex, config::Config, expression::Expression, symbols::Resolution, types::TypeContext,
};
use std::ops::Deref;

mod check;
mod constantness;
mod constants;
mod expression;
mod generics;
mod numeric;
mod place;
mod range;
mod resolve;
#[cfg(test)]
mod tests;

pub struct TypeInfer<'a, S = &'a mut (dyn Resolution + 'a)> {
    config: &'a mut Config,
    symbols: S,
    context: Option<&'a dyn TypeContext>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueCategory {
    Value,
    Place { mutable: bool },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Inferred {
    pub ty: TypeIndex,
    pub category: ValueCategory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Inference {
    Value(Inferred),
    Void,
    NeedsExpectedType,
    Poison,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckMode {
    Initialization,
    Assignment,
    Argument,
    Return,
}

impl<'a> TypeInfer<'a> {
    pub fn new(config: &'a mut Config, symbols: &'a mut dyn Resolution) -> Self {
        Self {
            config,
            symbols,
            context: None,
        }
    }

    pub fn with_context(mut self, context: &'a dyn TypeContext) -> Self {
        self.context = Some(context);
        self
    }

    pub fn infer(&mut self, expression: &Expression) -> Inference {
        if self.config.is_poisoned() || expression.is_poisoned() {
            return Inference::Poison;
        }
        let ty = self.type_of(expression);
        if self.config.is_poisoned() {
            return Inference::Poison;
        }
        if ty.is_empty() {
            return if matches!(expression, Expression::FuncCallE(_, ..)) {
                Inference::Void
            } else {
                Inference::NeedsExpectedType
            };
        }
        let category = if matches!(
            expression,
            Expression::VarValueE(_, ..)
                | Expression::UnaryExprE(crate::ast::expression::UnaryExpr::Access(_), ..)
                | Expression::UnaryExprE(
                    crate::ast::expression::UnaryExpr::Operator {
                        op: crate::ast::expression::operators::Operator::Dereference,
                        ..
                    },
                    ..
                )
        ) {
            ValueCategory::Place {
                mutable: self.assignable(expression),
            }
        } else {
            ValueCategory::Value
        };
        Inference::Value(Inferred { ty, category })
    }
}

impl<'a> TypeInfer<'a, &'a (dyn Resolution + 'a)> {
    pub fn read_only(config: &'a mut Config, symbols: &'a dyn Resolution) -> Self {
        Self {
            config,
            symbols,
            context: None,
        }
    }
}

impl<'a, S> TypeInfer<'a, S>
where
    S: Deref<Target = dyn Resolution + 'a>,
{
    pub fn is_const(&self, expression: &Expression) -> bool {
        TypeInfer::is_constant(expression, &*self.symbols)
    }

    /// Canonicalize an already accepted initializer without introducing conversions.
    pub fn normalize(&self, expression: Expression, target: TypeIndex) -> Expression {
        TypeInfer::normalize_value(expression, target, &*self.symbols)
    }
}
