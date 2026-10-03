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
use crate::diagnostic::error::{CompileError, ErrorHandle, ResolveError};
use crate::diagnostic::ice::ice;
use crate::parser::out::{
    TempEnum, TempFunc, TempGeneric, TempInterface, TempUnit, TempUsing, TempVar,
};
use crate::parser::{TempGlobalStmt, TempModule};

#[derive(Debug, Clone, PartialEq)]
pub struct SymbolTable {
    pub types: TypeArena,
    pub base_type_view: HashMap<usize, TypeIndex>,
    pub functions: FuncArena,
    pub interfaces: InterfaceArena,
    pub generics: GenericTable,
    pub globals: HashMap<String, (usize, Rc<Variable>)>,
    pub names: HashSet<String>,
    pub(crate) declaration_spans: HashMap<String, crate::parser::out::Span>,
    pub usings: HashMap<String, String>,
    pub function_instances: HashMap<FuncIndex, FuncBody>,
    pub interface_instances: HashMap<InterfaceIndex, Interface>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self::for_file(crate::ast::config::FileId::new(0))
    }

    pub fn for_file(file_id: crate::ast::config::FileId) -> Self {
        let (types, view) = TypeArena::new_for_file(file_id);
        Self {
            types,
            functions: FuncArena::for_file(file_id),
            interfaces: InterfaceArena::for_file(file_id),
            generics: GenericTable::for_file(file_id),
            globals: HashMap::new(),
            names: HashSet::new(),
            declaration_spans: HashMap::new(),
            usings: HashMap::new(),
            base_type_view: view,
            function_instances: HashMap::new(),
            interface_instances: HashMap::new(),
        }
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

mod concat;
mod declarations;
pub use concat::ConcatError;
pub(crate) mod export;
mod import;
pub use export::{ExportSymbol, ExportTable};
pub use import::ImportModule;
mod frame;
mod globals;
pub(crate) mod owner;
mod package;
mod path;
mod resolution;
pub use frame::{ContextBinding, ScopeFrame, StatementContext, SupperScopeType};
pub use package::PackageSymbolTable;
pub use path::{PATH_SEARCH_ORDER, PathSymbol, PathSymbolKind};
pub use resolution::{Resolution, ResolveContext, ResolveScope};
