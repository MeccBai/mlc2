mod instance;

use crate::ast::arena::{ArenaIndex, FuncArena, GenericArena, GenericIndex, NamedArena, get_ident};
use crate::ast::types::CompileType::{Base, Enum};
use crate::ast::types::{CompileType, UnitType, ref_type};
use crate::ast::{SymbolTable, TypeIndex};
use crate::error::ice::ice;
use crate::parser::out::TempGeneric;
use std::collections::HashMap;

pub struct InterfaceRequire {
    pub name: String,
    pub ret_type: Option<TypeIndex>,
    pub params: Vec<(TypeIndex, String)>,
    pub mutable: bool,
}

impl InterfaceRequire {
    pub fn check(&self, param: TypeIndex, symbols: &SymbolTable) -> bool {
        let param_name = param.format(&symbols.types);
        let interface_name = format!("{}::{}", param_name, self.name);
        let interface = symbols.functions.get(&interface_name);

        let interface = match interface {
            Some(interface) => interface,
            None => return false,
        };
        let ret_type = &interface.ret_type;
        if self.ret_type != *ret_type {
            return false;
        }
        if self.mutable != interface.mutable {
            return false;
        }
        if self.params.len() != interface.params.len() {
            return false;
        }
        if self
            .params
            .iter()
            .zip(interface.params.iter())
            .any(|((param_type, _), (expected_type, _))| param_type != expected_type)
        {
            return false;
        }
        true
    }
}

pub enum GenericTypeRequire {
    Integer,
    Float,
    Signed,
    MinBits(usize),
    MaxBits(usize),
}

impl GenericTypeRequire {
    pub fn check(&self, param: TypeIndex, symbols: &SymbolTable) -> bool {
        match self {
            GenericTypeRequire::Integer => param.is_integer(&symbols.types),
            GenericTypeRequire::Float => param.is_float(&symbols.types),
            GenericTypeRequire::Signed => param.is_signed(&symbols.types),
            GenericTypeRequire::MinBits(bits) => {
                let size = param.size(&symbols.types);
                let param_bits = size * 8;
                param_bits >= *bits
            }
            GenericTypeRequire::MaxBits(bits) => {
                let size = param.size(&symbols.types);
                let param_bits = size * 8;
                param_bits <= *bits
            }
        }
    }
}

pub enum Constraints {
    Function(InterfaceRequire),
    Type(GenericTypeRequire),
}

impl Constraints {
    //pub fn check(&self, param: TypeIndex, symbols: &SymbolTable) -> bool {
    //    match self {
    //        Self::Function(interface) => interface.check(param, symbols),
    //        Self::Type(requirement) => requirement.check(param, symbols),
    //    }
    //}
    pub fn check(&self, param: TypeIndex, symbols: &SymbolTable) -> bool {
        match self {
            Constraints::Function(interface) => interface.check(param, symbols),
            Constraints::Type(requirement) => requirement.check(param, symbols),
        }
    }
}

pub struct GenericRequire {
    pub name: String,
    pub requires: Vec<Constraints>,
}

impl GenericRequire {
    pub fn new(temp: TempGeneric) -> Self {
        todo!()
    }
    pub fn check(&self, param: TypeIndex, symbols: &SymbolTable) -> bool {
        self.requires
            .iter()
            .all(|requirement| requirement.check(param, symbols))
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

pub enum InsFailed {
    NoGenerics,
    RequireUnMet,
    CountMismatch,
}

impl GenericIndex {
    pub fn check(&self, param: TypeIndex, symbols: &SymbolTable) -> bool {
        let generic = symbols.generics.get(*self);
        generic.check(param, symbols)
    }

    pub fn instantiation(
        self,
        params: &HashMap<GenericIndex, TypeIndex>,
        symbols: &SymbolTable,
    ) -> Result<TypeIndex, InsFailed> {
        let require = symbols.generics.get(self);

        let param = match params.get(&self) {
            Some(param) => *param,
            None => ice("Generic param not found in instantiation."),
        };

        if require.check(param, symbols) {
            Ok(param)
        } else {
            Err(InsFailed::RequireUnMet)
        }
    }
}

impl TypeIndex {
    pub fn instantiation(
        self,
        params: &HashMap<GenericIndex, TypeIndex>,
        symbols: &mut SymbolTable,
        actives: Option<&mut HashMap<String, TypeIndex>>,
    ) -> Result<TypeIndex, InsFailed> {
        let ty = symbols.types.get(self).clone();
        match ty {
            Base(_) | Enum(_) => Ok(self),
            CompileType::Generic(generic) => generic.instantiation(params, symbols),
            CompileType::Unit(unit) => unit.instantiation(params, symbols, actives),
            CompileType::List(list) => list.instantiation(params, symbols, actives),
            CompileType::Ref(ref_type) => ref_type.instantiation(params, symbols, actives),
        }
    }
}
