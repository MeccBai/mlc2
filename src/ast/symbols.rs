use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use super::arena::{FuncArena, FuncIndex, GenericIndex, TypeArena, TypeIndex};
use super::config::Config;
use super::function::FuncBody;
use super::generic::GenericRequire;
use crate::ast::arena::{self, InterfaceArena, InterfaceIndex, get_ident};
use crate::ast::function::{FuncSymbol, Interface, InterfaceSymbol};
use crate::ast::generic::GenericTable;
use crate::ast::statement::Variable;
use crate::ast::types::CompileType::{Base, Enum, List, Ref, Unit};
use crate::ast::types::{BaseType, CompileType, EnumType, UnitType, base_type::DataType};
use crate::error::{CompileError, ErrorHandle, ResolveError, ice::ice};
use crate::parser::out::{
    TempEnum, TempFunc, TempGeneric, TempInterface, TempUnit, TempUsing, TempVar,
};
use crate::parser::{TempGlobalStmt, TempModule};

pub struct SymbolTable {
    pub types: TypeArena,
    pub base_type_view: HashMap<usize, TypeIndex>,
    pub functions: FuncArena,
    pub interfaces: InterfaceArena,
    pub generics: GenericTable,
    pub globals: HashMap<String, Rc<Variable>>,
    pub names: HashSet<String>,
    pub usings: HashMap<String, String>,
    pub function_instances: HashMap<FuncIndex, FuncBody>,
    pub interface_instances: HashMap<InterfaceIndex, Interface>,
}

impl SymbolTable {
    pub fn new() -> Self {
        let (types, view) = TypeArena::new();
        let mut temp = Self {
            types,
            functions: FuncArena::empty(),
            interfaces: InterfaceArena::empty(),
            generics: GenericTable::new(),
            globals: HashMap::new(),
            names: HashSet::new(),
            usings: HashMap::new(),
            base_type_view: view,
            function_instances: HashMap::new(),
            interface_instances: HashMap::new(),
        };
        let base_types = BaseType::base_types();
        base_types
            .into_iter()
            .for_each(|(_, base_type)| match base_type {
                Base(base) => {
                    let name = base.name();
                    let ident = arena::get_ident(&name);
                    let ret_type = temp.types.get_by_ident(ident.clone()).unwrap();
                    let func = FuncSymbol {
                        name,
                        params: vec![(TypeIndex::empty(), "...".to_string())],
                        ret_type: Some(ret_type),
                        generics: Vec::new(),
                        attributes: Vec::new(),
                        generic_map: HashMap::new(),
                        exported: false,
                    };
                    temp.functions.insert(ident, func);
                }
                _ => ice("Expected a Base type."),
            });
        temp
    }

    pub fn get_base(&self, data: DataType, bits: usize, signed: bool) -> TypeIndex {
        let index = BaseType::to_index(data, bits, signed);
        match self.base_type_view.get(&index) {
            Some(&ty_index) => ty_index,
            None => ice("Base type not found in type arena."),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnumBool<T, F> {
    True(T),
    False(F),
}

mod declarations;
mod frame;
mod path;
pub use frame::{ContextBinding, ScopeFrame, StatementContext, SupperScopeType};
pub use path::{PATH_SEARCH_ORDER, PathSymbol, PathSymbolKind};
