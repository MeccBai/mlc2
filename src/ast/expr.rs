pub mod operators;
pub mod parse;

use crate::ast::config::Config;
use crate::ast::stmt::Variable;
use crate::ast::{FuncIndex, SymbolTable, TypeIndex};
use crate::parser::Spanned;
use crate::parser::out::TempExpr;
use operators::Operator;
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompAtom {
    VarValue(Variable),
    Composite(Composite),
    FunctionCall(FunctionCall),
    EnumValue(EnumValue),
    ConstValue(ConstValue),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Composite {
    pub members: Vec<CompAtom>,
    pub operators: Vec<Operator>,
    pub operator_first: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitialList {
    pub values: Vec<(TypeIndex, Expression)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionCall {
    pub func: FuncIndex,
    pub args: Vec<Expression>,
    pub piped: bool,
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

impl ConstValue {
    pub fn null() -> Self {
        ConstValue {
            value: "null".to_string(),
            ty: TypeIndex::empty(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expression {
    VarValue(Variable),
    InitList(InitialList),
    Composite(Composite),
    FunctionCall(FunctionCall),
    EnumValue(EnumValue),
    ConstValue(ConstValue),
}

impl Expression {
    pub fn null() -> Self {
        Expression::ConstValue(ConstValue::null())
    }

    pub fn type_inference(&self, config: &mut Config, symbols: &mut SymbolTable) -> TypeIndex {
        todo!()
    }


}
