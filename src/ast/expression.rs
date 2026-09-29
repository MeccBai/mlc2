pub mod creator;
pub mod operators;

use std::rc::Rc;

use crate::ast::arena::{InterfaceIndex, get_ident};
use crate::ast::config::Config;
use crate::ast::statement::Variable;
use crate::ast::types::CompileType::{self, List};
use crate::ast::types::ListType;
use crate::ast::types::base_type::DataType;
use crate::ast::{EnumBool, FuncIndex, SymbolTable, TypeIndex};
use crate::error::ice::ice;
use crate::parser::out::TempLiteralKind;

use operators::Operator;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompAtom {
    VarValueA(Rc<Variable>),
    CompositeA(Composite),
    FuncCallA(FuncCall),
    ConstValueA(ConstValue),
    UnaryExprA(UnaryExpr),
    MemberAccessA(MemberAccess),
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
pub struct MemberAccess {
    pub base: Box<CompAtom>,
    pub indirect: bool,
    pub name: String,
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
pub struct UnaryExpr {
    pub op: Operator,
    pub value: Box<CompAtom>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expression {
    VarValueE(Rc<Variable>),
    InitListE(InitialList),
    CompositeE(Composite),
    FuncCallE(FuncCall),
    ConstValueE(ConstValue),
    UnaryExprE(UnaryExpr),
    MemberAccessE(MemberAccess),
}
impl CompAtom {
    pub fn from_expr(expr: Expression) -> Self {
        match expr {
            Expression::CompositeE(composite) => CompAtom::CompositeA(composite),
            Expression::FuncCallE(func_call) => CompAtom::FuncCallA(func_call),
            Expression::ConstValueE(const_value) => CompAtom::ConstValueA(const_value),
            Expression::VarValueE(var) => CompAtom::VarValueA(var),
            Expression::UnaryExprE(unary) => CompAtom::UnaryExprA(unary),
            Expression::MemberAccessE(access) => CompAtom::MemberAccessA(access),
            Expression::InitListE(_) => ice("InitialList cannot be computed."),
        }
    }

    pub fn to_expression(self) -> Expression {
        match self {
            CompAtom::CompositeA(composite) => Expression::CompositeE(composite),
            CompAtom::FuncCallA(func_call) => Expression::FuncCallE(func_call),
            CompAtom::ConstValueA(const_value) => Expression::ConstValueE(const_value),
            CompAtom::VarValueA(var) => Expression::VarValueE(var),
            CompAtom::UnaryExprA(unary) => Expression::UnaryExprE(unary),
            CompAtom::MemberAccessA(access) => Expression::MemberAccessE(access),
        }
    }

    pub fn type_inference(&self, config: &mut Config, symbols: &mut SymbolTable) -> TypeIndex {
        match self {
            CompAtom::VarValueA(var) => var.var_type,
            CompAtom::CompositeA(composite) => composite.type_inference(config, symbols),
            CompAtom::FuncCallA(func_call) => func_call.type_inference(symbols),
            CompAtom::ConstValueA(const_value) => const_value.ty,
            CompAtom::UnaryExprA(unary) => unary.type_inference(config, symbols),
            CompAtom::MemberAccessA(access) => access.type_inference(config, symbols),
        }
    }
}

impl Expression {
    pub fn type_inference(&self, config: &mut Config, symbols: &mut SymbolTable) -> TypeIndex {
        match self {
            Expression::VarValueE(var) => var.var_type,
            Expression::ConstValueE(const_value) => const_value.ty,
            Expression::UnaryExprE(unary) => unary.type_inference(config, symbols),
            Expression::MemberAccessE(access) => access.type_inference(config, symbols),
            Expression::InitListE(init_list) => match init_list {
                InitialList::List { onwer, values } => match onwer {
                    Some(ty) => *ty,
                    None => TypeIndex::empty(),
                },
                InitialList::Array { ty, values } => ty.clone(),
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
        target: TypeIndex,
        config: &mut Config,
        symbols: &mut SymbolTable,
    ) -> bool {
        match self {
            Expression::VarValueE(var) => target.type_check(true, &var.var_type, &symbols.types),
            Expression::ConstValueE(const_value) => {
                target.type_check(true, &const_value.ty, &symbols.types)
            }
            Expression::UnaryExprE(unary) => {
                unary
                    .type_inference(config, symbols)
                    .type_check(true, &target, &symbols.types)
            }
            Expression::MemberAccessE(access) => {
                access
                    .type_inference(config, symbols)
                    .type_check(false, &target, &symbols.types)
            }
            Expression::InitListE(init_list) => match init_list {
                InitialList::List { onwer, values } => match onwer {
                    Some(ty) => *ty == target,
                    None => {
                        let check_list_types = match symbols.types.get(target) {
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
                    if target.type_check(true, ty, &symbols.types) {
                        return false;
                    }
                    let element_type = match symbols.types.get(*ty) {
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
                    let target = symbols.types.get(target);

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
}

impl Composite {
    fn type_inference(&self, config: &mut Config, symbols: &mut SymbolTable) -> TypeIndex {
        self.members.first().map_or(TypeIndex::empty(), |member| {
            member.type_inference(config, symbols)
        })
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
        let value_type = self.value.type_inference(config, symbols);
        match self.op {
            Operator::AddressOf => value_type.make_ref(&mut symbols.types, false),
            Operator::MutableAddressOf => value_type.make_ref(&mut symbols.types, true),
            Operator::Dereference => value_type.deref(&mut symbols.types).unwrap(),
            _ => value_type,
        }
    }
}

impl MemberAccess {
    fn type_inference(&self, config: &mut Config, symbols: &mut SymbolTable) -> TypeIndex {
        let base_type = self.base.type_inference(config, symbols);
        match symbols.types.get(base_type) {
            CompileType::Unit(unit) => unit
                .get_member(&self.name)
                .map_or(TypeIndex::empty(), |member| member.member_type),
            _ => TypeIndex::empty(),
        }
    }
}
