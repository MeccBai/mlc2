use super::TypeInfer;
use crate::ast::{builtins::Builtin, expression::FuncCall};
use crate::ast::{
    expression::{Access, CompAtom, Expression, InitialList, UnaryExpr, operators::Operator},
    symbols::Resolution,
    types::{CompileType, ValueType},
};
impl TypeInfer<'_> {
    pub(crate) fn atom_is_constant(expression: &CompAtom, symbols: &dyn Resolution) -> bool {
        match expression {
            CompAtom::ConstValueA(_, ..) => true,
            CompAtom::CompositeA(composite, ..) => composite
                .members
                .iter()
                .all(|member| member.is_const(symbols)),
            CompAtom::FuncCallA(call, ..) => call.is_const(symbols),
            CompAtom::VarValueA(var, ..) => var.var_type.value_type(symbols) == ValueType::Constant,
            CompAtom::UnaryExprA(unary, ..) => unary.is_const(symbols),
            _ => false,
        }
    }
    pub fn is_constant(expression: &Expression, symbols: &dyn Resolution) -> bool {
        match expression {
            Expression::ConstValueE(_, ..) => true,
            Expression::CompositeE(composite, ..) => composite
                .members
                .iter()
                .all(|member| member.is_const(symbols)),
            Expression::UnaryExprE(unary, ..) => unary.is_const(symbols),
            Expression::VarValueE(var, ..) => var.var_type.value_type(symbols) == ValueType::Constant,
            Expression::InitListE(
                InitialList::Array { values, .. } | InitialList::List { values, .. },
             ..) => values.iter().all(|value| value.is_const(symbols)),
            Expression::InitListE(InitialList::String { .. }, ..) => true,
            Expression::FuncCallE(call, ..) => call.is_const(symbols),
            _ => false,
        }
    }
    pub(crate) fn unary_is_constant(expression: &UnaryExpr, symbols: &dyn Resolution) -> bool {
        match expression {
            UnaryExpr::Function { .. } => true,
            UnaryExpr::Operator { value, .. } => value.is_const(symbols),
            UnaryExpr::Access(access) => access.is_const(symbols),
        }
    }
    pub(crate) fn access_is_constant(expression: &Access, symbols: &dyn Resolution) -> bool {
        match expression {
            Access::Member { base, .. } => base.is_const(symbols),
            Access::Index { base, index } => base.is_const(symbols) && index.is_const(symbols),
        }
    }
    pub(crate) fn call_is_constant(expression: &FuncCall, symbols: &dyn Resolution) -> bool {
        expression.callee.is_none()
            && matches!(expression.func, crate::ast::symbols::EnumBool::False(index)
            if matches!(symbols.get_function_regular(index).builtin(), Some(Builtin::Cast | Builtin::ConstCStr | Builtin::StrLen))
                && expression.args.iter().all(|argument| argument.is_const(symbols)))
    }
}
