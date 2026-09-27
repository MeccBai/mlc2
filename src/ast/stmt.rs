use crate::ast::TypeIndex;
use crate::ast::expr::{Expression, FunctionCall};
use std::sync::Arc;
pub mod variable;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variable {
    pub name: String,
    pub var_type: TypeIndex,
    pub init_val: Box<Expression>,
    pub immutable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assignment {
    pub variable: Arc<Variable>,
    pub value: Box<Expression>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IfStatement {
    pub condition: Box<Expression>,
    pub then_branch: Vec<Statement>,
    pub else_branch: Option<Vec<Statement>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WhileStatement {
    pub condition: Box<Expression>,
    pub stmts: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForStatement {
    pub init: Option<Box<Statement>>,
    pub condition: Box<Expression>,
    pub execution: Option<Box<Statement>>,
    pub stmts: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchStatement {
    pub value: Box<Expression>,
    pub branches: Vec<(Expression, Vec<Statement>)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReturnStatement {
    pub value: Option<Expression>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnonymousBlock {
    pub statements: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Statement {
    VariableDecl(Variable),
    Assignment(Assignment),
    FuncCall(FunctionCall),
    IfBlock(IfStatement),
    WhileBlock(WhileStatement),
    ForBlock(ForStatement),
    ReturnBlock(ReturnStatement),
    MatchBlock(MatchStatement),
    AnonymousBlock(AnonymousBlock),
    Continue,
    Break,
}


