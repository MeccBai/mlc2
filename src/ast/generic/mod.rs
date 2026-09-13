mod instance;

use crate::ast::TypeIndex;
use crate::ast::arena::{ArenaIndex, FuncArena, GenericArena, NamedArena};
use crate::ast::types::UnitType;
use crate::parser::out::TempGeneric;

pub struct Interface {
    pub name: String,
    pub ret_type: TypeIndex,
    pub params: Vec<(TypeIndex, String)>,
}

pub enum GenericTypeRequire {
    Integer,
    Float,
    Signed,
    MinBits(usize),
    MaxBits(usize),
}

pub enum Constraints {
    Function(Interface),
    Type(GenericTypeRequire),
}

pub struct GenericRequire {
    pub name: String,
    pub requires: Vec<Constraints>,
}

impl GenericRequire {
    pub fn new(temp_generic: TempGeneric) -> Self {
        todo!()
    }
}

pub type UnitArena = NamedArena<UnitType>;
pub type UnitIndex = ArenaIndex<UnitType>;
pub struct GenericTable {
    pub generics: GenericArena,
    pub units: UnitArena,
    pub funcs: FuncArena,
}

impl GenericTable {
    pub fn new() -> Self {
        Self {
            generics: GenericArena::empty(),
            units: UnitArena::empty(),
            funcs: FuncArena::empty(),
        }
    }
}
