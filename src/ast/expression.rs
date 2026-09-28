pub mod operators;
pub mod creator;

use std::rc::Rc;

use crate::ast::arena::InterfaceIndex;
use crate::ast::statement::Variable;
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
