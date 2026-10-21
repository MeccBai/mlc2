use super::TypeInfer;
use crate::ast::{
    expression::{Access, CompAtom, Expression, InitialList, UnaryExpr, operators::Operator},
    symbols::Resolution,
    types::{CompileType, ValueType},
};
impl TypeInfer<'_> {
    pub(crate) fn atom_assignable(&mut self, expression: &CompAtom) -> bool {
        let ty = self.atom_type(expression);
        if !ty.is_empty() && matches!(self.symbols.get_type(ty).unqualified(), CompileType::Str) {
            return false;
        }
        let config = &mut *self.config;
        let symbols = &mut *self.symbols;
        match expression {
            CompAtom::VarValueA(var, ..) => var.var_type.value_type(symbols) == ValueType::Flex,
            CompAtom::UnaryExprA(unary, ..) => unary.is_left_value(config, symbols),
            _ => false,
        }
    }
    pub fn assignable(&mut self, expression: &Expression) -> bool {
        let ty = self.type_of(expression);
        if !ty.is_empty() && matches!(self.symbols.get_type(ty).unqualified(), CompileType::Str) {
            return false;
        }
        let config = &mut *self.config;
        let symbols = &mut *self.symbols;
        match expression {
            Expression::VarValueE(var, ..) => var.var_type.value_type(symbols) == ValueType::Flex,
            Expression::UnaryExprE(unary, ..) => unary.is_left_value(config, symbols),
            _ => false,
        }
    }
    pub(crate) fn unary_assignable(&mut self, expression: &UnaryExpr) -> bool {
        let config = &mut *self.config;
        let symbols = &mut *self.symbols;
        match expression {
            UnaryExpr::Operator {
                op: Operator::Dereference,
                value,
            } => {
                let ty = value.type_inference(config, symbols);
                matches!(symbols.get_type(ty).unqualified(), CompileType::Ref(reference) if reference.mut_base)
            }
            UnaryExpr::Access(access) => access.is_left_value(config, symbols),
            _ => false,
        }
    }
    pub(crate) fn access_assignable(&mut self, expression: &Access) -> bool {
        let config = &mut *self.config;
        let symbols = &mut *self.symbols;
        match expression {
            Access::Member { base, indirect, .. } => {
                if *indirect {
                    let ty = base.type_inference(config, symbols);
                    matches!(
                        symbols.get_type(ty).unqualified(),
                        CompileType::Ref(reference) if reference.mut_base
                    )
                } else {
                    base.is_left_value(config, symbols)
                }
            }
            Access::Index { base, .. } => {
                let ty = base.type_inference(config, symbols);
                match symbols.get_type(ty).unqualified() {
                    CompileType::Ref(reference) => reference.mut_base,
                    _ => base.assignable(config, symbols),
                }
            }
        }
    }
}
