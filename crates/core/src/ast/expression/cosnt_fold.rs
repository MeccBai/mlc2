use super::{
    Access, CompAtom, Composite, ConstValue, Expression, InitialList, UnaryExpr,
    operators::Operator,
};
use crate::ast::symbols::Resolution;
use crate::ast::{
    config::Config,
    symbols::SymbolTable,
    types::{CompileType, ValueType, base_type::DataType},
};

impl Expression {
    /// Unsupported or invalid arithmetic is preserved, not evaluated with host overflow rules.
    pub fn const_fold(self, config: &mut Config, symbols: &dyn Resolution) -> Self {
        if config.is_poisoned() {
            return Self::Poison;
        }
        match self {
            Self::VarValueE(var) if var.var_type.value_type(symbols) == ValueType::Constant => {
                (*var.init_val).clone()
            }
            Self::CompositeE(mut c) => {
                c.members = c
                    .members
                    .into_iter()
                    .map(|a| fold_atom(a, config, symbols))
                    .collect();
                if c.members.len() != c.operators.len() + 1 || c.members.is_empty() {
                    return Self::CompositeE(c);
                }
                fold_composite(c, symbols).to_expression()
            }
            Self::FuncCallE(mut call) => {
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
                    if symbol.builtin() == Some(crate::ast::builtins::Builtin::Cast) {
                        if let (Some(target), Some(Self::ConstValueE(input))) =
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
                                        return Self::ConstValueE(ConstValue {
                                            value: converted.to_string(),
                                            ty: target,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
                Self::FuncCallE(call)
            }
            Self::UnaryExprE(UnaryExpr::Operator {
                op: Operator::Negate,
                value,
            }) => {
                let value = fold_atom(*value, config, symbols);
                if let CompAtom::ConstValueA(constant) = &value {
                    if constant.ty.is_integer(symbols) {
                        if let Some(negated) = constant
                            .value
                            .parse::<i128>()
                            .ok()
                            .and_then(i128::checked_neg)
                        {
                            return Self::ConstValueE(ConstValue {
                                value: negated.to_string(),
                                ty: constant.ty,
                            });
                        }
                    }
                }
                Self::UnaryExprE(UnaryExpr::Operator {
                    op: Operator::Negate,
                    value: Box::new(value),
                })
            }
            Self::UnaryExprE(unary) => Self::UnaryExprE(match unary {
                function @ UnaryExpr::Function { .. } => function,
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
            }),
            Self::InitListE(list) => Self::InitListE(match list {
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
            }),
            other => other,
        }
    }
}

fn fold_atom(a: CompAtom, config: &mut Config, symbols: &dyn Resolution) -> CompAtom {
    CompAtom::from_expr(a.to_expression().const_fold(config, symbols))
}

fn fold_composite(mut c: Composite, symbols: &dyn Resolution) -> CompAtom {
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
    if let (CompAtom::ConstValueA(a), CompAtom::ConstValueA(b)) = (&left, &right) {
        if let Some(value) = arithmetic(a, &op, b, symbols) {
            return CompAtom::ConstValueA(value);
        }
    }
    CompAtom::CompositeA(Composite {
        members: vec![left, right],
        operators: vec![op],
    })
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
        DataType::Boolean => return None,
    };
    Some(ConstValue { value, ty: a.ty })
}

#[cfg(test)]
mod tests;
