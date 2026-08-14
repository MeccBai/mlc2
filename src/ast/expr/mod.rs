pub mod operators;

use crate::ast::class::CompileType;
use crate::ast::func::FuncDecl;
use crate::ast::stmt::Variable;
use crate::ast::{FuncIndex, TypeIndex};
use operators::Operator;
use std::sync::Arc;

pub enum CompAtom {
    VarValue(Variable),
    Composite(Composite),
    FunctionCall(FunctionCall),
    EnumValue(EnumValue),
    ConstValue(ConstValue),
}

pub struct Composite {
    pub members: Vec<CompAtom>,
    pub operators: Vec<Operator>,
    pub operator_first: bool,
}

pub struct InitialList {
    pub values: Vec<(TypeIndex, Expression)>,
}

pub struct FunctionCall {
    pub func: FuncIndex,
    pub args: Vec<Expression>,
}

pub struct EnumValue {
    pub enum_type: TypeIndex,
    pub value: usize,
}

pub struct ConstValue {
    pub value: String,
    pub ty: TypeIndex,
}

pub enum Expression {
    VarValue(Variable),
    InitList(InitialList),
    Composite(Composite),
    FunctionCall(FunctionCall),
    EnumValue(EnumValue),
    ConstValue(ConstValue),
}
