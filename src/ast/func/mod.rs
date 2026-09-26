use crate::ast::arena::{FuncArena, FuncIndex, GenericArena, InterfaceIndex, TypeArena};
use crate::ast::generic::GenericTable;
use crate::ast::stmt::Statement;
use crate::ast::types::CompileType::Base;
use crate::ast::types::{BaseType, CompileType};
use crate::ast::{Config, GenericIndex, SymbolTable, TypeIndex, arena};
use crate::parser::Scope;
use crate::parser::out::{TempFunc, TempFuncSymbol};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FuncBody {
    pub name: FuncIndex,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FuncSymbol {
    pub name: String,
    pub params: Vec<(TypeIndex, String)>,
    pub ret_type: Option<TypeIndex>,
    pub generics: Vec<GenericIndex>,
    pub attributes: Vec<String>,
    pub exported: bool, // Whether this method requires a mutable implicit receiver.
                        //pub mutable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Interface {
    pub symbol: InterfaceIndex,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterfaceSymbol {
    pub public: bool,
    pub mutable: bool,
    pub exported: bool,
    pub owner: TypeIndex,
    pub attributes: Vec<String>,
    pub generics: Vec<GenericIndex>,
    pub name: String,
    pub params: Vec<(TypeIndex, String)>,
    pub ret_type: Option<TypeIndex>,
}

impl FuncSymbol {
    pub fn new(config: &Config, prototype: TempFuncSymbol, symbols: &SymbolTable) -> Self {
        todo!()
    }
}

impl FuncBody {
    pub fn new(
        index: FuncIndex,
        prototype: Option<Scope>,
        generics: &mut GenericTable,
        symbols: &mut SymbolTable,
    ) -> Self {
        todo!("Implement FuncBody::new")
    }
}

impl InterfaceSymbol {
    pub fn new(config: &Config, prototype: TempFuncSymbol, symbols: &SymbolTable) -> Self {
        todo!()
    }
}

impl Interface {
    pub fn new(
        index: InterfaceIndex,
        prototype: Option<Scope>,
        generics: &mut GenericTable,
        symbols: &mut SymbolTable,
    ) -> Self {
        todo!("Implement FuncBody::new")
    }
}
