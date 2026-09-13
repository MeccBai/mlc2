use crate::ast::stmt::Statement;
use crate::ast::{GenericIndex, TypeIndex};
use crate::parser::out::TempFunc;

pub struct FuncBody {
    pub name: String,
    pub params: Vec<(TypeIndex, String)>,
    pub ret_type: Option<TypeIndex>,
    pub body: Vec<Statement>,
    pub attributes: Vec<String>,
    pub generics: Vec<GenericIndex>,
    pub exported: bool,
}

pub struct FuncSymbol {
    pub name: String,
    pub params: Vec<(TypeIndex, String)>,
    pub ret_type: Option<TypeIndex>,
    pub generics: Vec<GenericIndex>,
    pub attributes: Vec<String>,
    pub exported: bool,
}

impl FuncBody {
    pub fn new(prototype: TempFunc) -> Self {
        todo!("Implement FuncBody::new")
    }
}
