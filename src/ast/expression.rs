pub mod creator;
pub mod operators;

use std::rc::Rc;

use crate::ast::arena::{InterfaceIndex, get_ident};
use crate::ast::config::Config;
use crate::ast::statement::Variable;
use crate::ast::types::CompileType::{self, List};
use crate::ast::types::ListType;
use crate::ast::types::ValueType;
use crate::ast::types::base_type::DataType;
use crate::ast::{
    FuncIndex, TypeIndex,
    symbols::{EnumBool, SymbolTable},
};
use crate::diagnostic::ice::ice;
use crate::parser::out::TempLiteralKind;

use operators::Operator;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompAtom {
    Poison,
    VarValueA(Rc<Variable>),
    CompositeA(Composite),
    FuncCallA(FuncCall),
    ConstValueA(ConstValue),
    UnaryExprA(UnaryExpr),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Composite {
    pub members: Vec<CompAtom>,
    pub operators: Vec<Operator>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InitialList {
    List {
        onwer: Option<TypeIndex>,
        values: Vec<Expression>,
    },
    Array {
        ty: TypeIndex,
        values: Vec<Expression>,
    },
    //BaseArray {
    //    ty: TypeIndex,
    //    values: Vec<Expression>,
    //},
    String {
        value: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FuncCall {
    pub func: EnumBool<InterfaceIndex, FuncIndex>,
    pub args: Vec<Expression>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumValue {
    pub enum_type: TypeIndex,
    pub value: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstValue {
    pub value: String,
    pub ty: TypeIndex,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Access {
    Member {
        base: Box<CompAtom>,
        indirect: bool,
        name: String,
    },
    Index {
        base: Box<Expression>,
        index: Box<Expression>,
    },
}

impl ConstValue {
    pub fn null() -> Self {
        ConstValue {
            value: String::new(),
            ty: TypeIndex::empty(),
        }
    }

    pub fn new(value: (TempLiteralKind, String), symbols: &SymbolTable) -> Self {
        let (kind, text) = value;

        let ty = match kind {
            TempLiteralKind::Integer => symbols.get_base(DataType::Integer, 32, true),
            TempLiteralKind::Float => symbols.get_base(DataType::Float, 64, true),
            TempLiteralKind::String => ice(
                "String literal type cannot be a base type, it must be a reference to a string type!",
            ),
            TempLiteralKind::Boolean => symbols.get_base(DataType::Boolean, 8, false),
            TempLiteralKind::Null => TypeIndex::empty(),
        };

        ConstValue { value: text, ty }
    }
}

impl InitialList {
    pub fn empty() -> Self {
        Self::Array {
            ty: TypeIndex::empty(),
            values: Vec::new(),
        }
    }

    pub fn from_string(text: String) -> Self {
        Self::String { value: text }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnaryExpr {
    Operator { op: Operator, value: Box<CompAtom> },
    Access(Access),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expression {
    Poison,
    VarValueE(Rc<Variable>),
    InitListE(InitialList),
    CompositeE(Composite),
    FuncCallE(FuncCall),
    ConstValueE(ConstValue),
    UnaryExprE(UnaryExpr),
}
impl CompAtom {
    pub fn from_expr(expr: Expression) -> Self {
        match expr {
            Expression::Poison => CompAtom::Poison,
            Expression::CompositeE(composite) => CompAtom::CompositeA(composite),
            Expression::FuncCallE(func_call) => CompAtom::FuncCallA(func_call),
            Expression::ConstValueE(const_value) => CompAtom::ConstValueA(const_value),
            Expression::VarValueE(var) => CompAtom::VarValueA(var),
            Expression::UnaryExprE(unary) => CompAtom::UnaryExprA(unary),
            Expression::InitListE(_) => ice("InitialList cannot be computed."),
        }
    }

    pub fn to_expression(self) -> Expression {
        match self {
            CompAtom::Poison => Expression::Poison,
            CompAtom::CompositeA(composite) => Expression::CompositeE(composite),
            CompAtom::FuncCallA(func_call) => Expression::FuncCallE(func_call),
            CompAtom::ConstValueA(const_value) => Expression::ConstValueE(const_value),
            CompAtom::VarValueA(var) => Expression::VarValueE(var),
            CompAtom::UnaryExprA(unary) => Expression::UnaryExprE(unary),
        }
    }

    pub fn type_inference(&self, config: &mut Config, symbols: &mut SymbolTable) -> TypeIndex {
        if config.is_poisoned() {
            return TypeIndex::empty();
        }
        match self {
            CompAtom::Poison => TypeIndex::empty(),
            CompAtom::VarValueA(var) => var.var_type,
            CompAtom::CompositeA(composite) => composite.type_inference(config, symbols),
            CompAtom::FuncCallA(func_call) => func_call.type_inference(symbols),
            CompAtom::ConstValueA(const_value) => {
                const_value.ty.into(ValueType::Constant, &mut symbols.types)
            }
            CompAtom::UnaryExprA(unary) => unary.type_inference(config, symbols),
        }
    }

    pub fn is_condition(&self, config: &mut Config, symbols: &mut SymbolTable) -> bool {
        self.clone().to_expression().is_condition(config, symbols)
    }

    pub fn is_left_value(&self, config: &mut Config, symbols: &mut SymbolTable) -> bool {
        match self {
            CompAtom::VarValueA(var) => var.var_type.value_type(&symbols.types) == ValueType::Flex,
            CompAtom::UnaryExprA(unary) => unary.is_left_value(config, symbols),
            _ => false,
        }
    }

    pub fn is_const(&self, symbols: &SymbolTable) -> bool {
        match self {
            CompAtom::ConstValueA(_) => true,
            CompAtom::CompositeA(composite) => composite
                .members
                .iter()
                .all(|member| member.is_const(symbols)),
            CompAtom::FuncCallA(_) => false,
            CompAtom::VarValueA(var) => {
                var.var_type.value_type(&symbols.types) == ValueType::Constant
            }
            CompAtom::UnaryExprA(unary) => unary.is_const(symbols),
            _ => false,
        }
    }
}

impl Expression {
    pub fn is_poisoned(&self) -> bool {
        matches!(self, Self::Poison)
    }

    pub fn type_inference(&self, config: &mut Config, symbols: &mut SymbolTable) -> TypeIndex {
        if config.is_poisoned() {
            return TypeIndex::empty();
        }
        match self {
            Expression::Poison => TypeIndex::empty(),
            Expression::VarValueE(var) => var.var_type,
            Expression::ConstValueE(const_value) => {
                const_value.ty.into(ValueType::Constant, &mut symbols.types)
            }
            Expression::UnaryExprE(unary) => unary.type_inference(config, symbols),
            Expression::InitListE(init_list) => match init_list {
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
                    (*ty).into(value, &mut symbols.types)
                }
                InitialList::String { value } => {
                    let string_type = symbols.get_base(DataType::Integer, 8, true);
                    let ty = List(ListType::new(string_type, value.len()));
                    let name = ty.format(&symbols.types);
                    let ident = get_ident(&name);
                    symbols.types.insert(ident, ty)
                }
            },
            Expression::CompositeE(composite) => composite.type_inference(config, symbols),
            Expression::FuncCallE(func_call) => func_call.type_inference(symbols),
        }
    }

    pub fn is_null(&self) -> bool {
        match self {
            Expression::ConstValueE(const_value) => const_value.ty.is_empty(),
            _ => false,
        }
    }

    pub fn type_check(
        &self,
        target: &TypeIndex,
        config: &mut Config,
        symbols: &mut SymbolTable,
    ) -> bool {
        if config.is_poisoned() || self.is_poisoned() {
            return false;
        }
        match self {
            Expression::Poison => false,
            Expression::VarValueE(var) => target.type_check(true, &var.var_type, &symbols.types),
            Expression::ConstValueE(const_value) => {
                target.type_check(true, &const_value.ty, &symbols.types)
            }
            Expression::UnaryExprE(unary) => {
                unary
                    .type_inference(config, symbols)
                    .type_check(true, &target, &symbols.types)
            }
            Expression::InitListE(init_list) => match init_list {
                InitialList::List { onwer, values } => match onwer {
                    Some(ty) => ty == target,
                    None => {
                        let check_list_types = match symbols.types.get(target.clone()).unqualified()
                        {
                            CompileType::Unit(unit) => {
                                let members: Vec<_> = unit
                                    .members
                                    .iter()
                                    .map(|member| member.member_type.clone())
                                    .collect();
                                Some(members)
                            }
                            _ => None,
                        };
                        match check_list_types {
                            Some(list_types) => {
                                if list_types.len() != values.len() {
                                    return false;
                                }
                                values.iter().zip(list_types.iter()).all(|(value, ty)| {
                                    ty.type_check(
                                        true,
                                        &value.type_inference(config, symbols),
                                        &symbols.types,
                                    )
                                })
                            }
                            None => false,
                        }
                    }
                },
                InitialList::Array { ty, values } => {
                    if !target.type_check(true, ty, &symbols.types) {
                        return false;
                    }
                    let element_type = match symbols.types.get(*ty).unqualified() {
                        CompileType::List(list_type) => list_type.element_type,
                        _ => return false,
                    };
                    values.iter().all(|value| {
                        element_type.type_check(
                            true,
                            &value.type_inference(config, symbols),
                            &symbols.types,
                        )
                    })
                }
                InitialList::String { value } => {
                    let string_type = symbols.get_base(DataType::Integer, 8, true);
                    let target = symbols.types.get(target.clone()).unqualified();

                    let list = match target {
                        CompileType::List(list_type) => list_type,
                        _ => return false,
                    };

                    if list.length != value.len() {
                        return false;
                    }

                    list.element_type
                        .type_check(false, &string_type, &symbols.types)
                }
            },
            Expression::CompositeE(composite) => composite
                .type_inference(config, symbols)
                .type_check(false, &target, &symbols.types),
            Expression::FuncCallE(func_call) => {
                func_call
                    .type_inference(symbols)
                    .type_check(false, &target, &symbols.types)
            }
        }
    }

    pub fn is_condition(&self, config: &mut Config, symbols: &mut SymbolTable) -> bool {
        let ty = self.type_inference(config, symbols);
        if ty.is_empty() {
            return false;
        }
        let boolean = symbols.get_base(DataType::Boolean, 8, true);
        ty.type_check(false, &boolean, &symbols.types)
    }

    pub fn assignable(&self, config: &mut Config, symbols: &mut SymbolTable) -> bool {
        match self {
            Expression::VarValueE(var) => {
                var.var_type.value_type(&symbols.types) == ValueType::Flex
            }
            Expression::UnaryExprE(unary) => unary.is_left_value(config, symbols),
            _ => false,
        }
    }

    pub fn is_const(&self, symbols: &SymbolTable) -> bool {
        match self {
            Expression::ConstValueE(_) => true,
            Expression::CompositeE(composite) => composite
                .members
                .iter()
                .all(|member| member.is_const(symbols)),
            Expression::UnaryExprE(unary) => unary.is_const(symbols),
            Expression::VarValueE(var) => {
                var.var_type.value_type(&symbols.types) == ValueType::Constant
            }
            Expression::InitListE(
                InitialList::Array { values, .. } | InitialList::List { values, .. },
            ) => values.iter().all(|value| value.is_const(symbols)),
            Expression::InitListE(InitialList::String { .. }) => true,
            _ => false,
        }
    }

    pub fn ref_calculate_check(&self) -> bool {
        true
    }
}

impl Composite {
    fn type_inference(&self, config: &mut Config, symbols: &mut SymbolTable) -> TypeIndex {
        if self
            .operators
            .first()
            .is_some_and(|op| op.changes_binary_result_type())
        {
            symbols.get_base(DataType::Boolean, 8, true)
        } else {
            self.members.first().map_or(TypeIndex::empty(), |member| {
                member.type_inference(config, symbols)
            })
        }
    }
}

impl FuncCall {
    fn type_inference(&self, symbols: &SymbolTable) -> TypeIndex {
        match self.func {
            EnumBool::True(index) => symbols.interfaces.get(index).ret_type,
            EnumBool::False(index) => symbols.functions.get(index).ret_type,
        }
        .unwrap_or(TypeIndex::empty())
    }
}

impl UnaryExpr {
    fn type_inference(&self, config: &mut Config, symbols: &mut SymbolTable) -> TypeIndex {
        match self {
            Self::Operator { op, value } => {
                let value_type = value.type_inference(config, symbols);
                match op {
                    Operator::AddressOf => value_type.make_ref(&mut symbols.types, false),
                    Operator::MutOf => value_type.make_ref(&mut symbols.types, true),
                    Operator::Dereference => value_type.deref(&mut symbols.types).unwrap(),
                    _ => value_type,
                }
            }
            Self::Access(access) => access.type_inference(config, symbols),
        }
    }

    fn is_left_value(&self, config: &mut Config, symbols: &mut SymbolTable) -> bool {
        match self {
            Self::Operator {
                op: Operator::Dereference,
                value,
            } => {
                let ty = value.type_inference(config, symbols);
                matches!(symbols.types.get(ty).unqualified(), CompileType::Ref(reference) if reference.mut_base)
            }
            Self::Access(access) => access.is_left_value(config, symbols),
            _ => false,
        }
    }

    fn is_const(&self, symbols: &SymbolTable) -> bool {
        match self {
            Self::Operator { value, .. } => value.is_const(symbols),
            Self::Access(access) => access.is_const(symbols),
        }
    }
}

impl Access {
    fn type_inference(&self, config: &mut Config, symbols: &mut SymbolTable) -> TypeIndex {
        match self {
            Self::Member {
                base,
                indirect,
                name,
            } => {
                let mut base_type = base.type_inference(config, symbols);
                if *indirect && base_type.is_ref(&symbols.types) {
                    base_type = base_type
                        .deref(&mut symbols.types)
                        .unwrap_or(TypeIndex::empty());
                }
                let value = base_type.value_type(&symbols.types);
                let member = match symbols.types.get(base_type).unqualified() {
                    CompileType::Unit(unit) => unit
                        .get_member(name)
                        .map_or(TypeIndex::empty(), |member| member.member_type),
                    _ => TypeIndex::empty(),
                };
                member.into(value, &mut symbols.types)
            }
            Self::Index { base, .. } => {
                let base_type = base.type_inference(config, symbols);
                let value = base_type.value_type(&symbols.types);
                let element = match symbols.types.get(base_type).unqualified() {
                    CompileType::List(list) => list.element_type(),
                    _ => TypeIndex::empty(),
                };
                element.into(value, &mut symbols.types)
            }
        }
    }

    fn is_left_value(&self, config: &mut Config, symbols: &mut SymbolTable) -> bool {
        match self {
            Self::Member { base, indirect, .. } => {
                if *indirect {
                    let ty = base.type_inference(config, symbols);
                    matches!(
                        symbols.types.get(ty).unqualified(),
                        CompileType::Ref(reference) if reference.mut_base
                    )
                } else {
                    base.is_left_value(config, symbols)
                }
            }
            Self::Index { base, .. } => base.assignable(config, symbols),
        }
    }

    fn is_const(&self, symbols: &SymbolTable) -> bool {
        match self {
            Self::Member { base, .. } => base.is_const(symbols),
            Self::Index { base, index } => base.is_const(symbols) && index.is_const(symbols),
        }
    }
}
