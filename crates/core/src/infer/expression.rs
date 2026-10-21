use super::TypeInfer;
use crate::ast::{
    TypeIndex,
    arena::get_ident,
    config::Config,
    expression::{
        Access, CompAtom, Composite, Expression, FuncCall, InitialList, UnaryExpr,
        operators::Operator,
    },
    symbols::{EnumBool, Resolution},
    types::{CompileType, ListType, ValueType, base_type::DataType},
};
impl TypeInfer<'_> {
    pub(crate) fn atom_type(&mut self, expression: &CompAtom) -> TypeIndex {
        let config = &mut *self.config;
        let symbols = &mut *self.symbols;
        if config.is_poisoned() {
            return TypeIndex::empty();
        }
        match expression {
            CompAtom::Poison(..) => TypeIndex::empty(),
            CompAtom::VarValueA(var, ..) => var.var_type,
            CompAtom::CompositeA(composite, ..) => composite.type_inference(config, symbols),
            CompAtom::FuncCallA(func_call, ..) => func_call.type_inference(symbols),
            CompAtom::ConstValueA(const_value, ..) => const_value.ty.into(ValueType::Constant, symbols),
            CompAtom::UnaryExprA(unary, ..) => unary.type_inference(config, symbols),
        }
    }
    pub fn type_of(&mut self, expression: &Expression) -> TypeIndex {
        let config = &mut *self.config;
        let symbols = &mut *self.symbols;
        if config.is_poisoned() {
            return TypeIndex::empty();
        }
        match expression {
            Expression::Poison(..) => TypeIndex::empty(),
            Expression::VarValueE(var, ..) => var.var_type,
            Expression::ConstValueE(const_value, ..) => {
                const_value.ty.into(ValueType::Constant, symbols)
            }
            Expression::UnaryExprE(unary, ..) => unary.type_inference(config, symbols),
            Expression::InitListE(init_list, ..) => match init_list {
                InitialList::List { onwer, values } => match onwer {
                    Some(ty) => *ty,
                    None => TypeIndex::empty(),
                },
                InitialList::Array { ty, values } => {
                    let value = if values.iter().all(|value| value.is_const(symbols)) {
                        ValueType::Constant
                    } else {
                        ValueType::Flex
                    };
                    (*ty).into(value, symbols)
                }
                InitialList::String { .. } => symbols.local_type("str").expect("native str type"),
            },
            Expression::CompositeE(composite, ..) => composite.type_inference(config, symbols),
            Expression::FuncCallE(func_call, ..) => func_call.type_inference(symbols),
        }
    }
    pub(crate) fn composite_type(&mut self, expression: &Composite) -> TypeIndex {
        let config = &mut *self.config;
        let symbols = &mut *self.symbols;
        if expression
            .operators
            .first()
            .is_some_and(|op| op.changes_binary_result_type())
        {
            symbols.get_base(DataType::Boolean, 8, true)
        } else {
            expression
                .members
                .first()
                .map_or(TypeIndex::empty(), |member| {
                    member.type_inference(config, symbols)
                })
        }
    }
    pub(crate) fn unary_type(&mut self, expression: &UnaryExpr) -> TypeIndex {
        let config = &mut *self.config;
        let symbols = &mut *self.symbols;
        match expression {
            UnaryExpr::Function { ty, .. } => *ty,
            UnaryExpr::Operator { op, value } => {
                let value_type = value.type_inference(config, symbols);
                match op {
                    Operator::AddressOf => value_type.make_ref(symbols, false),
                    Operator::MutOf => value_type.make_ref(symbols, true),
                    Operator::Dereference => value_type.deref(symbols).unwrap(),
                    _ => value_type,
                }
            }
            UnaryExpr::Access(access) => access.type_inference(config, symbols),
        }
    }
    pub(crate) fn access_type(&mut self, expression: &Access) -> TypeIndex {
        let config = &mut *self.config;
        let symbols = &mut *self.symbols;
        match expression {
            Access::Member {
                base,
                indirect,
                name,
            } => {
                let mut base_type = base.type_inference(config, symbols);
                if *indirect && base_type.is_ref(symbols) {
                    base_type = base_type.deref(symbols).unwrap_or(TypeIndex::empty());
                }
                let value = base_type.value_type(symbols);
                let member = match symbols.get_type(base_type).unqualified() {
                    CompileType::Unit(unit) => unit
                        .get_member(name)
                        .map_or(TypeIndex::empty(), |member| member.member_type),
                    _ => TypeIndex::empty(),
                };
                member.into(value, symbols)
            }
            Access::Index { base, .. } => {
                let base_type = base.type_inference(config, symbols);
                let value = base_type.value_type(symbols);
                let element = match symbols.get_type(base_type).unqualified() {
                    CompileType::List(list) => list.element_type(),
                    CompileType::Ref(reference) => {
                        let value = if reference.mut_base {
                            ValueType::Flex
                        } else {
                            ValueType::Final
                        };
                        let element = base_type.deref(symbols).unwrap_or(TypeIndex::empty());
                        return element.into(value, symbols);
                    }
                    _ => TypeIndex::empty(),
                };
                element.into(value, symbols)
            }
        }
    }
    pub(crate) fn call_result(expression: &FuncCall, symbols: &dyn Resolution) -> TypeIndex {
        match expression.func {
            EnumBool::True(index) => symbols.get_interface_regular(index).ret_type,
            EnumBool::False(index) => symbols.get_function_regular(index).ret_type,
        }
        .unwrap_or(TypeIndex::empty())
    }
}
