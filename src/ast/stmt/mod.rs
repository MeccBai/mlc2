use crate::ast::TypeIndex;
use crate::ast::expr::{Expression, FunctionCall};
use std::sync::Arc;

pub struct Variable {
    pub name: String,
    pub var_type: TypeIndex,
    pub init_val: Box<Expression>,
    pub immutable: bool,
}

pub struct Assignment {
    pub variable: Arc<Variable>,
    pub value: Box<Expression>,
}

pub struct IfStatement {
    pub condition: Box<Expression>,
    pub then_branch: Vec<Statement>,
    pub else_branch: Option<Vec<Statement>>,
}

pub struct WhileStatement {
    pub condition: Box<Expression>,
    pub stmts: Vec<Statement>,
}

pub struct ForStatement {
    pub init: Option<Box<Statement>>,
    pub condition: Box<Expression>,
    pub execution: Option<Box<Statement>>,
    pub stmts: Vec<Statement>,
}

pub struct MatchStatement {
    pub value: Box<Expression>,
    pub branches: Vec<(Expression, Vec<Statement>)>,
}

pub struct ReturnStatement {
    pub value: Option<Expression>,
}

pub struct AnonymousBlock {
    pub statements: Vec<Statement>,
}
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
