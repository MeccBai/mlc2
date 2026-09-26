pub mod build;
pub mod operators;

use crate::ast::stmt::Variable;
use crate::ast::{FuncIndex, TypeIndex};
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expression {
    VarValue(Variable),
    InitList(InitialList),
    Composite(Composite),
    FunctionCall(FunctionCall),
    EnumValue(EnumValue),
    ConstValue(ConstValue),
}
