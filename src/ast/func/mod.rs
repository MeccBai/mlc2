use crate::ast::stmt::Statement;
use crate::ast::{FuncIndex, TypeIndex};
use crate::parser::GenericDecl;
use std::sync::Arc;

pub struct FuncDecl {
    pub func: FuncIndex,
}
pub struct FuncDef {
    pub name: String,
    pub generics: Vec<Arc<GenericDecl>>,
    pub params: Vec<(TypeIndex, String)>,
    pub ret_type: Option<TypeIndex>,
    pub body: Vec<Statement>,
    pub attributes: Vec<String>,
}
