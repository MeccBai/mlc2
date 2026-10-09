//! Value-aware scalar initialization. Runtime conversions must preserve the full domain.
use super::{CompAtom, ConstValue, Expression, UnaryExpr, operators::Operator};
use crate::ast::{
    TypeIndex,
    config::Config,
    symbols::Resolution,
    types::{
        CompileType,
        base_type::{BaseType, DataType},
    },
};

enum Number {
    Integer(i128),
    Float(f64),
}

impl Expression {
    pub(super) fn numeric_initialization(
        &self,
        target: TypeIndex,
        config: &mut Config,
        symbols: &mut dyn Resolution,
    ) -> Option<bool> {
        let source = self.type_inference(config, symbols);
        if target.is_empty() || source.is_empty() {
            return None;
        }
        let (CompileType::Base(to), CompileType::Base(from)) = (
            symbols.get_type(target).unqualified(),
            symbols.get_type(source).unqualified(),
        ) else {
            return None;
        };
        if !numeric(to) || !numeric(from) {
            return None;
        }
        Some(match number(self, symbols) {
            Some(value) => representable(&value, to),
            None => to.type_check(true, from),
        })
    }

    pub(super) fn numeric_constant_loss(
        &self,
        target: TypeIndex,
        symbols: &dyn Resolution,
    ) -> bool {
        let CompileType::Base(to) = symbols.get_type(target).unqualified() else {
            return false;
        };
        numeric(to) && number(self, symbols).is_some_and(|value| !representable(&value, to))
    }

    /// Give accepted constants their destination type before lowering. This avoids
    /// truncating a large literal through its default i32 storage first.
    pub fn normalize_initializer(
        self,
        target: TypeIndex,
        symbols: &(impl crate::ast::types::TypeLookup + ?Sized),
    ) -> Self {
        if let CompileType::Base(to) = symbols.get_type(target).unqualified() {
            if numeric(to) {
                if let Some(value) = number(&self, symbols).filter(|value| representable(value, to))
                {
                    let value = match (value, to.data_type()) {
                        (Number::Integer(value), DataType::Integer) => value.to_string(),
                        (Number::Float(value), DataType::Integer) => (value as i128).to_string(),
                        (Number::Integer(value), DataType::Float) => (value as f64).to_string(),
                        (Number::Float(value), DataType::Float) => value.to_string(),
                        _ => unreachable!("numeric target checked"),
                    };
                    return Self::ConstValueE(ConstValue { value, ty: target });
                }
            }
        }
        match (self, symbols.get_type(target).unqualified()) {
            (
                Self::InitListE(super::InitialList::Array { ty, values }),
                CompileType::List(list),
            ) => Self::InitListE(super::InitialList::Array {
                ty,
                values: values
                    .into_iter()
                    .map(|value| value.normalize_initializer(list.element_type, symbols))
                    .collect(),
            }),
            (
                Self::InitListE(super::InitialList::List { onwer, values }),
                CompileType::Unit(unit),
            ) => Self::InitListE(super::InitialList::List {
                onwer,
                values: values
                    .into_iter()
                    .zip(&unit.members)
                    .map(|(value, member)| value.normalize_initializer(member.member_type, symbols))
                    .collect(),
            }),
            (value, _) => value,
        }
    }
}

fn numeric(ty: &BaseType) -> bool {
    matches!(ty.data_type(), DataType::Integer | DataType::Float)
}

fn number(
    expression: &Expression,
    symbols: &(impl crate::ast::types::TypeLookup + ?Sized),
) -> Option<Number> {
    match expression {
        Expression::ConstValueE(value) => {
            let CompileType::Base(base) = symbols.get_type(value.ty).unqualified() else {
                return None;
            };
            match base.data_type() {
                DataType::Integer => value.value.parse().ok().map(Number::Integer),
                DataType::Float => value.value.parse::<f64>().ok().map(|value| {
                    Number::Float(if base.bits() == 32 {
                        value as f32 as f64
                    } else {
                        value
                    })
                }),
                _ => None,
            }
        }
        Expression::UnaryExprE(UnaryExpr::Operator {
            op: Operator::Negate,
            value,
        }) => {
            let CompAtom::ConstValueA(value) = value.as_ref() else {
                return None;
            };
            match number(&Expression::ConstValueE(value.clone()), symbols)? {
                Number::Integer(value) => value.checked_neg().map(Number::Integer),
                Number::Float(value) => Some(Number::Float(-value)),
            }
        }
        _ => None,
    }
}

fn representable(value: &Number, target: &BaseType) -> bool {
    match (value, target.data_type()) {
        (Number::Integer(value), DataType::Integer) => integer_fits(*value, target),
        (Number::Float(value), DataType::Integer) => {
            value.is_finite() && value.fract() == 0.0 && integer_fits(*value as i128, target)
        }
        (Number::Integer(value), DataType::Float) => {
            let magnitude = value.unsigned_abs();
            let significant = if magnitude == 0 {
                0
            } else {
                128 - magnitude.leading_zeros() - magnitude.trailing_zeros()
            };
            significant <= if target.bits() == 32 { 24 } else { 53 }
        }
        (Number::Float(value), DataType::Float) => {
            value.is_finite() && (target.bits() == 64 || *value == *value as f32 as f64)
        }
        _ => false,
    }
}

fn integer_fits(value: i128, target: &BaseType) -> bool {
    let (min, max) = if target.signed() {
        (
            -(1i128 << (target.bits() - 1)),
            (1i128 << (target.bits() - 1)) - 1,
        )
    } else {
        (0, (1i128 << target.bits()) - 1)
    };
    (min..=max).contains(&value)
}
