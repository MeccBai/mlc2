//! Conservative scalar evaluation for control-flow pruning, not propagation of locals.
use crate::ast::{
    TypeIndex,
    builtins::Builtin,
    expression::{CompAtom, Composite, Expression, UnaryExpr, operators::Operator},
    statement::{ForStatement, MatchPattern, MatchStatement, Statement, Variable},
    symbols::{EnumBool, PackageSymbolTable},
    types::{CompileType, ValueType, base_type::DataType},
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Scalar {
    Integer(i128, TypeIndex),
    Float(f64, TypeIndex),
    Bool(bool),
}

impl Scalar {
    pub(super) fn boolean(self) -> Option<bool> {
        match self {
            Self::Bool(value) => Some(value),
            _ => None,
        }
    }

    fn same_value(self, other: Self) -> bool {
        match (self, other) {
            (Self::Integer(a, _), Self::Integer(b, _)) => a == b,
            (Self::Float(a, _), Self::Float(b, _)) => a == b,
            (Self::Bool(a), Self::Bool(b)) => a == b,
            _ => false,
        }
    }
}

pub(super) enum MatchSelection<'a> {
    Dynamic,
    Skip,
    Scope(&'a [Statement]),
}

pub(super) fn select_match<'a>(
    statement: &'a MatchStatement,
    package: &PackageSymbolTable,
) -> MatchSelection<'a> {
    let Some(scrutinee) = value(&statement.value, package) else {
        return MatchSelection::Dynamic;
    };
    let mut default = None;
    for (pattern, body) in &statement.branches {
        match pattern {
            MatchPattern::Default => default = Some(body.as_slice()),
            MatchPattern::Value(pattern) => {
                let Some(pattern) = value(pattern, package) else {
                    return MatchSelection::Dynamic;
                };
                if scrutinee.same_value(pattern) {
                    return MatchSelection::Scope(body);
                }
            }
        }
    }
    default.map_or(MatchSelection::Skip, MatchSelection::Scope)
}

pub(super) fn value(expression: &Expression, package: &PackageSymbolTable) -> Option<Scalar> {
    evaluate(expression, package, None)
}

pub(super) fn empty_for(statement: &ForStatement, package: &PackageSymbolTable) -> bool {
    let binding = match statement.init.as_deref() {
        Some(Statement::VariableDecl(variable)) => {
            let Some(initial) = value(&variable.init_val, package) else {
                return false;
            };
            Some((&**variable, initial))
        }
        None => None,
        _ => return false,
    };
    evaluate(&statement.condition, package, binding).and_then(Scalar::boolean) == Some(false)
}

fn evaluate(
    expression: &Expression,
    package: &PackageSymbolTable,
    binding: Option<(&Variable, Scalar)>,
) -> Option<Scalar> {
    if matches!(expression, Expression::ConstValueE(constant) if constant.ty.is_empty()) {
        return None;
    }
    match expression {
        Expression::ConstValueE(constant) => match package.get_type(constant.ty).unqualified() {
            CompileType::Base(base) => match base.data_type() {
                DataType::Integer => integer(constant.value.parse().ok()?, constant.ty, package),
                DataType::Float => {
                    let number: f64 = constant.value.parse().ok()?;
                    if !number.is_finite() {
                        return None;
                    }
                    Some(Scalar::Float(
                        if base.bits() == 32 {
                            number as f32 as f64
                        } else {
                            number
                        },
                        constant.ty,
                    ))
                }
                DataType::Boolean => Some(Scalar::Bool(match constant.value.as_str() {
                    "true" | "1" => true,
                    "false" | "0" => false,
                    _ => return None,
                })),
            },
            CompileType::Enum(_) => {
                Some(Scalar::Integer(constant.value.parse().ok()?, constant.ty))
            }
            _ => None,
        },
        Expression::VarValueE(variable) => {
            if let Some((target, initial)) = binding {
                if std::ptr::eq(&**variable, target) {
                    return Some(initial);
                }
            }
            (variable.var_type.value_type(package) == ValueType::Constant)
                .then(|| evaluate(&variable.init_val, package, binding))
                .flatten()
        }
        Expression::CompositeE(composite) => composite_value(composite, package, binding),
        Expression::UnaryExprE(UnaryExpr::Operator { op, value }) => {
            let value = atom(value, package, binding)?;
            match (op, value) {
                (Operator::LogicalNot, Scalar::Bool(value)) => Some(Scalar::Bool(!value)),
                (Operator::Negate, Scalar::Integer(value, ty)) => {
                    integer(value.checked_neg()?, ty, package)
                }
                (Operator::Negate, Scalar::Float(value, ty)) => Some(Scalar::Float(-value, ty)),
                _ => None,
            }
        }
        Expression::FuncCallE(call) => {
            let EnumBool::False(index) = call.func else {
                return None;
            };
            let symbol = package.get_function(index, false);
            if symbol.builtin() != Some(Builtin::Cast) {
                return None;
            }
            let input = evaluate(call.args.first()?, package, binding)?;
            let ty = symbol.ret_type?;
            let CompileType::Base(base) = package.get_type(ty).unqualified() else {
                return None;
            };
            match (input, base.data_type()) {
                (Scalar::Integer(input, _), DataType::Integer) => {
                    let modulus = 1i128.checked_shl(base.bits() as u32)?;
                    let mut converted = input & (modulus - 1);
                    if base.signed() && converted >= modulus / 2 {
                        converted -= modulus;
                    }
                    Some(Scalar::Integer(converted, ty))
                }
                _ => None,
            }
        }
        _ => None,
    }
}

fn atom(
    atom: &CompAtom,
    package: &PackageSymbolTable,
    binding: Option<(&Variable, Scalar)>,
) -> Option<Scalar> {
    evaluate(&atom.clone().to_expression(), package, binding)
}

fn composite_value(
    composite: &Composite,
    package: &PackageSymbolTable,
    binding: Option<(&Variable, Scalar)>,
) -> Option<Scalar> {
    fn part(
        members: &[CompAtom],
        operators: &[Operator],
        package: &PackageSymbolTable,
        binding: Option<(&Variable, Scalar)>,
    ) -> Option<Scalar> {
        if members.len() != operators.len() + 1 {
            return None;
        }
        let Some((split, op)) = operators
            .iter()
            .enumerate()
            .min_by_key(|(index, op)| (op.precedence(), std::cmp::Reverse(*index)))
        else {
            return atom(members.first()?, package, binding);
        };
        let left = part(&members[..split + 1], &operators[..split], package, binding)?;
        // Only skip operands whose runtime evaluation would also be skipped.
        match (op, left) {
            (Operator::LogicalAnd, Scalar::Bool(false)) => return Some(Scalar::Bool(false)),
            (Operator::LogicalOr, Scalar::Bool(true)) => return Some(Scalar::Bool(true)),
            _ => {}
        }
        let right = part(
            &members[split + 1..],
            &operators[split + 1..],
            package,
            binding,
        )?;
        binary(left, op, right, package)
    }
    part(&composite.members, &composite.operators, package, binding)
}

fn integer(value: i128, ty: TypeIndex, package: &PackageSymbolTable) -> Option<Scalar> {
    let (bits, signed) = match package.get_type(ty).unqualified() {
        CompileType::Base(base) => (base.bits(), base.signed()),
        CompileType::Enum(_) => (32, true),
        _ => return None,
    };
    let bound = 1i128.checked_shl((bits - usize::from(signed)) as u32)?;
    let minimum = if signed { -bound } else { 0 };
    (minimum..bound)
        .contains(&value)
        .then_some(Scalar::Integer(value, ty))
}

fn binary(
    left: Scalar,
    op: &Operator,
    right: Scalar,
    package: &PackageSymbolTable,
) -> Option<Scalar> {
    use Operator::*;
    match (left, right) {
        (Scalar::Integer(a, ty), Scalar::Integer(b, _)) => {
            let comparison = match op {
                Equal => Some(a == b),
                NotEqual => Some(a != b),
                Less => Some(a < b),
                LessOrEqual => Some(a <= b),
                Greater => Some(a > b),
                GreaterOrEqual => Some(a >= b),
                _ => None,
            };
            if let Some(value) = comparison {
                return Some(Scalar::Bool(value));
            }
            let result = match op {
                Add => a.checked_add(b),
                Subtract => a.checked_sub(b),
                Multiply => a.checked_mul(b),
                Divide => a.checked_div(b),
                Remainder => a.checked_rem(b),
                BitAnd => Some(a & b),
                BitOr => Some(a | b),
                BitXor => Some(a ^ b),
                _ => None,
            }?;
            integer(result, ty, package)
        }
        (Scalar::Float(a, _), Scalar::Float(b, _)) => Some(Scalar::Bool(match op {
            Equal => a == b,
            NotEqual => a != b,
            Less => a < b,
            LessOrEqual => a <= b,
            Greater => a > b,
            GreaterOrEqual => a >= b,
            _ => return None,
        })),
        (Scalar::Bool(a), Scalar::Bool(b)) => Some(Scalar::Bool(match op {
            Equal => a == b,
            NotEqual => a != b,
            LogicalAnd => a && b,
            LogicalOr => a || b,
            _ => return None,
        })),
        _ => None,
    }
}
