use super::TypeInfer;
use crate::ast::expression::{
    Access, CompAtom, Composite, ConstValue, Expression, InitialList, UnaryExpr,
    operators::Operator,
};
use crate::ast::symbols::Resolution;
use crate::ast::{
    config::Config,
    symbols::SymbolTable,
    types::{CompileType, base_type::DataType},
};
use std::ops::Deref;
impl<'a, S> TypeInfer<'a, S>
where
    S: Deref<Target = dyn Resolution + 'a>,
{
    pub fn constant(&mut self, expression: Expression) -> Expression {
        let span = expression.span();
        let config = &mut *self.config;
        let symbols: &dyn Resolution = &*self.symbols;
        if config.is_poisoned() {
            return Expression::Poison(span);
        }
        match expression {
            Expression::VarValueE(var, ..) if var.constant => {
                (*var.init_val).clone().with_span(span)
            }
            Expression::CompositeE(mut c, ..) => {
                c.members = c
                    .members
                    .into_iter()
                    .map(|a| fold_atom(a, config, symbols))
                    .collect();
                if c.members.len() != c.operators.len() + 1 || c.members.is_empty() {
                    return Expression::CompositeE(c, span);
                }
                fold_composite(c, symbols).to_expression().with_span(span)
            }
            Expression::UnaryExprE(
                UnaryExpr::Operator {
                    op: Operator::BitPattern,
                    value,
                },
                ..,
            ) => {
                let value = fold_atom(*value, config, symbols);
                if let CompAtom::ConstValueA(constant, ..) = &value {
                    if let CompileType::Base(base) = symbols.get_type(constant.ty).unqualified() {
                        if base.data_type() == DataType::Boolean {
                            return Expression::ConstValueE(constant.clone(), span);
                        }
                        let bits = base.bits();
                        let pattern = match base.data_type() {
                            DataType::Integer => constant
                                .value
                                .parse::<i128>()
                                .ok()
                                .map(|n| (n as u128) & ((1u128 << bits) - 1)),
                            DataType::Float if bits == 32 => constant
                                .value
                                .parse::<f32>()
                                .ok()
                                .map(|n| n.to_bits() as u128),
                            DataType::Float => constant
                                .value
                                .parse::<f64>()
                                .ok()
                                .map(|n| n.to_bits() as u128),
                            _ => None,
                        };
                        if let Some(pattern) = pattern {
                            return Expression::ConstValueE(
                                ConstValue {
                                    ty: symbols.get_base(DataType::Integer, bits, false),
                                    value: pattern.to_string(),
                                },
                                span,
                            );
                        }
                    }
                }
                Expression::UnaryExprE(
                    UnaryExpr::Operator {
                        op: Operator::BitPattern,
                        value: Box::new(value),
                    },
                    span,
                )
            }
            Expression::UnaryExprE(
                UnaryExpr::Operator {
                    op: Operator::BitNot,
                    value,
                },
                ..,
            ) => {
                let value = fold_atom(*value, config, symbols);
                if let CompAtom::ConstValueA(constant, ..) = &value {
                    if let CompileType::Base(base) = symbols.get_type(constant.ty).unqualified() {
                        let result = match base.data_type() {
                            DataType::Integer => constant
                                .value
                                .parse::<i128>()
                                .ok()
                                .map(|n| {
                                    let mask = (1i128 << base.bits()) - 1;
                                    let pattern = !n & mask;
                                    if base.signed() && pattern & (1i128 << (base.bits() - 1)) != 0
                                    {
                                        pattern - (1i128 << base.bits())
                                    } else {
                                        pattern
                                    }
                                })
                                .map(|n| n.to_string()),
                            DataType::Boolean => match constant.value.as_str() {
                                "true" => Some("false".into()),
                                "false" => Some("true".into()),
                                _ => None,
                            },
                            _ => None,
                        };
                        if let Some(value) = result {
                            return Expression::ConstValueE(
                                ConstValue {
                                    ty: constant.ty,
                                    value,
                                },
                                span,
                            );
                        }
                    }
                }
                Expression::UnaryExprE(
                    UnaryExpr::Operator {
                        op: Operator::BitNot,
                        value: Box::new(value),
                    },
                    span,
                )
            }
            Expression::FuncCallE(mut call, ..) => {
                call.callee = call
                    .callee
                    .map(|callee| Box::new(callee.const_fold(config, symbols)));
                call.args = call
                    .args
                    .into_iter()
                    .map(|a| a.const_fold(config, symbols))
                    .collect();
                if let crate::ast::symbols::EnumBool::False(index) = call.func {
                    let symbol = symbols.get_function_regular(index);
                    if symbol.builtin() == Some(crate::ast::builtins::Builtin::StrLen) {
                        if let Some(Expression::InitListE(InitialList::String { value }, ..)) =
                            call.args.first()
                        {
                            return Expression::ConstValueE(
                                ConstValue {
                                    value: value.len().to_string(),
                                    ty: symbol.ret_type.unwrap(),
                                },
                                span,
                            );
                        }
                    }
                    if symbol.builtin() == Some(crate::ast::builtins::Builtin::Cast) {
                        if let (Some(target), Some(Expression::ConstValueE(input, ..))) =
                            (symbol.ret_type, call.args.first())
                        {
                            if let CompileType::Base(ty) = symbols.get_type(target).unqualified() {
                                if ty.data_type() == DataType::Integer {
                                    if let Ok(value) = input.value.parse::<i128>() {
                                        let bits = ty.bits();
                                        let modulus = 1i128 << bits;
                                        let mut converted = value & (modulus - 1);
                                        if ty.signed() && converted >= (1i128 << (bits - 1)) {
                                            converted -= modulus;
                                        }
                                        return Expression::ConstValueE(
                                            ConstValue {
                                                value: converted.to_string(),
                                                ty: target,
                                            },
                                            span,
                                        );
                                    }
                                }
                                if ty.data_type() == DataType::Float {
                                    if let Ok(value) = input.value.parse::<f64>() {
                                        let value = if ty.bits() == 32 {
                                            (value as f32) as f64
                                        } else {
                                            value
                                        };
                                        if value.is_finite() {
                                            return Expression::ConstValueE(
                                                ConstValue {
                                                    value: value.to_string(),
                                                    ty: target,
                                                },
                                                span,
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                if let Some(value) = crate::constant_eval::evaluate(&call, config, symbols, span) {
                    return value;
                }
                Expression::FuncCallE(call, span)
            }
            Expression::UnaryExprE(
                UnaryExpr::Operator {
                    op: Operator::Negate,
                    value,
                },
                ..,
            ) => {
                let value = fold_atom(*value, config, symbols);
                if let CompAtom::ConstValueA(constant, ..) = &value {
                    if constant.ty.is_integer(symbols) {
                        if let Some(negated) = constant
                            .value
                            .parse::<i128>()
                            .ok()
                            .and_then(i128::checked_neg)
                        {
                            return Expression::ConstValueE(
                                ConstValue {
                                    value: negated.to_string(),
                                    ty: constant.ty,
                                },
                                span,
                            );
                        }
                    }
                }
                Expression::UnaryExprE(
                    UnaryExpr::Operator {
                        op: Operator::Negate,
                        value: Box::new(value),
                    },
                    span,
                )
            }
            Expression::UnaryExprE(
                UnaryExpr::Access(Access::Member {
                    base,
                    indirect: false,
                    name,
                }),
                ..,
            ) => {
                let folded = base.clone().to_expression().const_fold(config, symbols);
                if let Expression::InitListE(
                    InitialList::List {
                        onwer: Some(owner),
                        values,
                    },
                    ..,
                ) = &folded
                {
                    if let CompileType::Unit(unit) = symbols.get_type(*owner).unqualified() {
                        if let Some(index) = unit.members.iter().position(|m| m.name == name) {
                            if let Some(value) = values.get(index) {
                                return value.clone().with_span(span);
                            }
                        }
                    }
                }
                let base = if matches!(folded, Expression::InitListE(..)) {
                    *base
                } else {
                    CompAtom::from_expr(folded)
                };
                Expression::UnaryExprE(
                    UnaryExpr::Access(Access::Member {
                        base: Box::new(base),
                        indirect: false,
                        name,
                    }),
                    span,
                )
            }
            Expression::UnaryExprE(UnaryExpr::Access(Access::Index { base, index }), ..) => {
                let constant = base.is_const(symbols);
                let base = base.const_fold(config, symbols);
                let index = index.const_fold(config, symbols);
                if constant {
                    if let (
                        Expression::InitListE(InitialList::Array { values, .. }, ..),
                        Expression::ConstValueE(i, ..),
                    ) = (&base, &index)
                    {
                        if let Some(value) =
                            i.value.parse::<usize>().ok().and_then(|i| values.get(i))
                        {
                            return value.clone().with_span(span);
                        }
                    }
                }
                Expression::UnaryExprE(
                    UnaryExpr::Access(Access::Index {
                        base: Box::new(base),
                        index: Box::new(index),
                    }),
                    span,
                )
            }
            Expression::UnaryExprE(unary, ..) => Expression::UnaryExprE(
                match unary {
                    function @ UnaryExpr::Function { .. } => function,
                    UnaryExpr::Operator {
                        op: op @ (Operator::AddressOf | Operator::MutOf),
                        value,
                    } => UnaryExpr::Operator { op, value },
                    UnaryExpr::Operator { op, value } => UnaryExpr::Operator {
                        op,
                        value: Box::new(fold_atom(*value, config, symbols)),
                    },
                    UnaryExpr::Access(Access::Member {
                        base,
                        indirect,
                        name,
                    }) => UnaryExpr::Access(Access::Member {
                        base: Box::new(fold_atom(*base, config, symbols)),
                        indirect,
                        name,
                    }),
                    UnaryExpr::Access(Access::Index { base, index }) => {
                        UnaryExpr::Access(Access::Index {
                            base: Box::new(base.const_fold(config, symbols)),
                            index: Box::new(index.const_fold(config, symbols)),
                        })
                    }
                },
                span,
            ),
            Expression::InitListE(list, ..) => Expression::InitListE(
                match list {
                    InitialList::List { onwer, values } => InitialList::List {
                        onwer,
                        values: values
                            .into_iter()
                            .map(|v| v.const_fold(config, symbols))
                            .collect(),
                    },
                    InitialList::Array { ty, values } => InitialList::Array {
                        ty,
                        values: values
                            .into_iter()
                            .map(|v| v.const_fold(config, symbols))
                            .collect(),
                    },
                    string @ InitialList::String { .. } => string,
                },
                span,
            ),
            other => other,
        }
    }
}
fn fold_atom(a: CompAtom, config: &mut Config, symbols: &dyn Resolution) -> CompAtom {
    let folded = a.clone().to_expression().const_fold(config, symbols);
    if matches!(folded, Expression::InitListE(..)) {
        a
    } else {
        CompAtom::from_expr(folded)
    }
}

fn fold_composite(mut c: Composite, symbols: &dyn Resolution) -> CompAtom {
    let span = c.span();
    if c.operators.is_empty() {
        return c.members.remove(0);
    }
    // Rightmost minimum gives left associativity; nested composites preserve parentheses.
    let split = c
        .operators
        .iter()
        .enumerate()
        .min_by_key(|(i, op)| (op.precedence(), std::cmp::Reverse(*i)))
        .map(|(i, _)| i)
        .unwrap();
    let right_members = c.members.split_off(split + 1);
    let right_ops = c.operators.split_off(split + 1);
    let op = c.operators.pop().unwrap();
    let left = fold_composite(c, symbols);
    let right = fold_composite(
        Composite {
            members: right_members,
            operators: right_ops,
        },
        symbols,
    );
    if let (CompAtom::ConstValueA(a, ..), CompAtom::ConstValueA(b, ..)) = (&left, &right) {
        if let Some(value) = arithmetic(a, &op, b, symbols) {
            return CompAtom::ConstValueA(value, span);
        }
    }
    CompAtom::CompositeA(
        Composite {
            members: vec![left, right],
            operators: vec![op],
        },
        span,
    )
}

fn arithmetic(
    a: &ConstValue,
    op: &Operator,
    b: &ConstValue,
    symbols: &dyn Resolution,
) -> Option<ConstValue> {
    use Operator::*;
    if a.ty.is_empty() || b.ty.is_empty() {
        return None;
    }
    let CompileType::Base(ty) = symbols.get_type(a.ty).unqualified() else {
        return None;
    };
    if symbols.get_type(a.ty).unqualified() != symbols.get_type(b.ty).unqualified() {
        return None;
    }
    let value = match ty.data_type() {
        DataType::Integer => {
            let a: i128 = a.value.parse().ok()?;
            let b: i128 = b.value.parse().ok()?;
            let result = match op {
                Add => a.checked_add(b),
                Subtract => a.checked_sub(b),
                Multiply => a.checked_mul(b),
                Divide => a.checked_div(b),
                Remainder => a.checked_rem(b),
                BitAnd => Some(a & b),
                BitOr => Some(a | b),
                BitXor => Some(a ^ b),
                ShiftLeft if b >= 0 && b < ty.bits() as i128 => {
                    Some((a << b) & ((1i128 << ty.bits()) - 1))
                }
                ShiftRight if b >= 0 && b < ty.bits() as i128 => Some(a >> b),
                _ => None,
            }?;
            let bits = ty.bits();
            let (min, max) = if ty.signed() {
                let bound = 1i128.checked_shl(bits.checked_sub(1)? as u32)?;
                (-bound, bound - 1)
            } else {
                (0, 1i128.checked_shl(bits as u32)? - 1)
            };
            if ![a, b, result].iter().all(|v| (min..=max).contains(v)) {
                return None;
            }
            result.to_string()
        }
        DataType::Float => {
            if ty.bits() == 32 {
                let a: f32 = a.value.parse().ok()?;
                let b: f32 = b.value.parse().ok()?;
                let result = match op {
                    Add => a + b,
                    Subtract => a - b,
                    Multiply => a * b,
                    Divide if b != 0.0 => a / b,
                    Remainder if b != 0.0 => a % b,
                    _ => return None,
                };
                if !result.is_finite() {
                    return None;
                }
                result.to_string()
            } else {
                let a: f64 = a.value.parse().ok()?;
                let b: f64 = b.value.parse().ok()?;
                let result = match op {
                    Add => a + b,
                    Subtract => a - b,
                    Multiply => a * b,
                    Divide if b != 0.0 => a / b,
                    Remainder if b != 0.0 => a % b,
                    _ => return None,
                };
                if !result.is_finite() {
                    return None;
                }
                result.to_string()
            }
        }
        DataType::Boolean => {
            let a = match a.value.as_str() {
                "true" => true,
                "false" => false,
                _ => return None,
            };
            let b = match b.value.as_str() {
                "true" => true,
                "false" => false,
                _ => return None,
            };
            match op {
                BitAnd => a & b,
                BitOr => a | b,
                BitXor => a ^ b,
                ShiftLeft | ShiftRight if !b => a,
                _ => return None,
            }
            .to_string()
        }
    };
    Some(ConstValue { value, ty: a.ty })
}
