use crate::ast::TypeIndex;
use crate::ast::expr::{Expression, FunctionCall};
use std::sync::Arc;

pub struct Variable {
    pub name: String,
    pub var_type: TypeIndex,
    pub init_val: Arc<Expression>,
}

pub struct Assignment {
    pub variable: Arc<Variable>,
    pub value: Arc<Expression>,
}

pub struct IfStatement {
    pub condition: Arc<Expression>,
    pub then_branch: Vec<Statement>,
    pub else_branch: Option<Vec<Statement>>,
}

pub struct WhileStatement {
    pub condition: Arc<Expression>,
    pub stmts: Vec<Statement>,
}

pub struct ForStatement {
    pub init: Option<Arc<Statement>>,
    pub condition: Arc<Expression>,
    pub execution: Option<Arc<Statement>>,
    pub stmts: Vec<Statement>,
}

pub struct MatchStatement {
    pub value: Arc<Expression>,
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
