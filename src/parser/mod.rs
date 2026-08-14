mod split;
mod stmt;
mod func;
mod expr;
mod generic;

use crate::ast::class::CompileType;
use crate::ast::class::EnumType;
use crate::ast::stmt::Variable;
use crate::lexer::TokenPack;
use std::sync::Arc;
use std::vec::IntoIter;

pub struct ImportModule {
    pub path: Vec<String>,
}

impl ImportModule {
    pub fn new(path: Vec<String>) -> Self {
        Self { path }
    }
}

pub struct FunctionBody {
    pub name: String,
    pub generics: Vec<String>,
    pub params: Vec<(String, String)>,
    pub ret_type: Option<String>,
    pub body: Vec<TokenPack>,
    pub attributes: Vec<String>,
}

pub struct UnitDecl {
    pub name: String,
    pub members: Vec<(String, String)>,
    pub attributes: Vec<String>,
}

pub struct UsingType {
    pub name: String,
    pub alias: String,
}

pub struct GenericDecl {
    pub name: String,
    pub requires: Vec<Vec<TokenPack>>,
}

pub struct TempStmt {
    tokens: Vec<TokenPack>,
}

pub struct VarTemp {
    pub name: String,
    pub ty: Option<String>,
    pub value: Option<Vec<TokenPack>>,
}

pub enum TempGlobalStmt {
    Import(ImportModule),
    Function(FunctionBody),
    Unit(UnitDecl),
    Using(UsingType),
    Generic(GenericDecl),
    Enum(EnumType),
    GlobalVar(Variable)
}

pub type TokenIter = IntoIter<TokenPack>;

