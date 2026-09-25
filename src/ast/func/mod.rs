use crate::ast::arena::{FuncArena, GenericArena, TypeArena};
use crate::ast::stmt::Statement;
use crate::ast::types::CompileType::Base;
use crate::ast::types::{BaseType, CompileType};
use crate::ast::{GenericIndex, SymbolTable, TypeIndex, arena};
use crate::parser::out::TempFunc;

pub struct FuncBody {
    pub symbol: FuncSymbol,
    pub body: Vec<Statement>,
}

pub struct FuncSymbol {
    pub name: String,
    pub params: Vec<(TypeIndex, String)>,
    pub ret_type: Option<TypeIndex>,
    pub generics: Vec<GenericIndex>,
    pub attributes: Vec<String>,
    pub exported: bool,
    /// Whether this method requires a mutable implicit receiver.
    pub mutable: bool,
}

impl FuncBody {
    pub fn new(prototype: TempFunc, generics: &mut GenericArena, symbols: &SymbolTable) -> Self {
        todo!("Implement FuncBody::new")
    }
}
