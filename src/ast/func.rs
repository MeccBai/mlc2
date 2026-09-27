use chumsky::primitive::todo;

use crate::ast::arena::{FuncArena, FuncIndex, GenericArena, InterfaceIndex, TypeArena};
use crate::ast::generic::GenericTable;
use crate::ast::stmt::Statement;
use crate::ast::types::CompileType::Base;
use crate::ast::types::{BaseType, CompileType, resolve_type};
use crate::ast::{Config, GenericIndex, SymbolTable, TypeIndex, arena};
use crate::parser::Scope;
use crate::parser::out::{TempFunc, TempFuncSymbol, TempInterfaceSymbol};

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
    pub has_self: bool,
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
    pub fn new(config: &mut Config, prototype: TempFuncSymbol, symbols: &mut SymbolTable) -> Self {
        let params = prototype
            .params
            .into_iter()
            .filter_map(|param| {
                let ty = resolve_type(config, param.ty?, symbols)?;
                Some((ty, param.name))
            })
            .collect();
        let ret_type = prototype
            .return_type
            .and_then(|ty| resolve_type(config, ty, symbols));
        Self {
            name: prototype.name,
            params,
            ret_type,
            generics: Vec::new(),
            attributes: prototype.attributes,
            exported: matches!(
                prototype.visibility,
                crate::parser::out::TempVisibility::Export
                    | crate::parser::out::TempVisibility::Api
            ),
        }
    }
}

impl FuncBody {
    pub fn new(
        config: &mut Config,
        index: FuncIndex,
        prototype: Option<Scope>,
        symbols: &mut SymbolTable,
    ) -> Self {
        todo!("Implement FuncBody::new")
    }
}

impl InterfaceSymbol {
    pub fn new(
        config: &mut Config,
        prototype: TempInterfaceSymbol,
        symbols: &mut SymbolTable,
    ) -> Self {
        todo!()
    }
}

impl Interface {
    pub fn new(
        config: &mut Config,
        index: InterfaceIndex,
        prototype: Option<Scope>,
        symbols: &mut SymbolTable,
    ) -> Self {
        todo!("Implement FuncBody::new")
    }
}
