//! Check constant conversions before LLVM can silently narrow an integer.
use super::{Expression, InitialList, UnaryExpr, operators::Operator};
use crate::ast::{
    TypeIndex,
    config::Config,
    symbols::Resolution,
    types::{CompileType, base_type::DataType},
};
use crate::diagnostic::error::{CompileError, IllegalUseError};
use crate::parser::out::Span;

impl Expression {
    pub(crate) fn check_constant_range(
        &self,
        target: TypeIndex,
        config: &mut Config,
        symbols: &dyn Resolution,
        span: Span,
    ) {
        if target.is_empty() || config.is_poisoned() {
            return;
        }
        let folded = self.clone().const_fold(config, symbols);
        match (symbols.get_type(target).unqualified(), &folded) {
            (CompileType::Base(ty), _) if ty.data_type() == DataType::Integer => {
                let value = match &folded {
                    Expression::ConstValueE(value) => value.value.parse::<i128>().ok(),
                    Expression::UnaryExprE(UnaryExpr::Operator {
                        op: Operator::Negate,
                        value,
                    }) => match value.as_ref() {
                        super::CompAtom::ConstValueA(value) => {
                            value.value.parse::<i128>().ok().and_then(i128::checked_neg)
                        }
                        _ => None,
                    },
                    _ => None,
                };
                if let Some(value) = value {
                    let bits = ty.bits();
                    let (min, max) = if ty.signed() {
                        (-(1i128 << (bits - 1)), (1i128 << (bits - 1)) - 1)
                    } else {
                        (0, (1i128 << bits) - 1)
                    };
                    if value < min || value > max {
                        config.submit_error(
                            CompileError::IllegalUse(IllegalUseError::IntegerConstantOutOfRange {
                                value: value.to_string(),
                                target: ty.format(),
                            }),
                            span,
                        );
                    }
                }
            }
            (CompileType::List(list), Expression::InitListE(InitialList::Array { values, .. })) => {
                for value in values {
                    value.check_constant_range(list.element_type, config, symbols, span);
                }
            }
            (CompileType::Unit(unit), Expression::InitListE(InitialList::List { values, .. })) => {
                for (value, member) in values.iter().zip(&unit.members) {
                    value.check_constant_range(member.member_type, config, symbols, span);
                }
            }
            _ => {}
        }
        if !config.is_poisoned() && folded.numeric_constant_loss(target, symbols) {
            config.submit_error(
                CompileError::IllegalUse(IllegalUseError::NumericConstantLoss {
                    target: target.format(symbols),
                }),
                span,
            );
        }
    }
}
