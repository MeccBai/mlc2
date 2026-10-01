use crate::ast::TypeIndex;
use crate::ast::expression::{Expression, FuncCall};
use std::rc::Rc;
mod control;
pub mod creator;
mod exits;
pub mod variable;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variable {
    pub name: String,
    pub var_type: TypeIndex,
    pub init_val: Box<Expression>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assignment {
    pub variable: Box<Expression>,
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
    pub branches: Vec<(MatchPattern, Vec<Statement>)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchPattern {
    Default,
    Value(Expression),
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
    Poison,
    VariableDecl(Rc<Variable>),
    Assignment(Assignment),
    FuncCall(FuncCall),
    IfBlock(IfStatement),
    WhileBlock(WhileStatement),
    ForBlock(ForStatement),
    ReturnBlock(ReturnStatement),
    MatchBlock(MatchStatement),
    AnonymousBlock(AnonymousBlock),
    Expression(Expression),
    Continue,
    Break,
}
