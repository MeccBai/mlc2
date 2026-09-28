use crate::ast::function::{FuncSymbol, InterfaceSymbol};
use crate::ast::generic::GenericRequire;
use crate::ast::types::CompileType;
use std::collections::HashMap;
use std::fmt;
use std::marker::PhantomData;

pub type Ident = String;

pub struct ArenaIndex<T> {
    index: usize,
    _marker: PhantomData<fn() -> T>,
}

impl<T> fmt::Debug for ArenaIndex<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ArenaIndex({})", self.index)
    }
}

impl<T> PartialEq for ArenaIndex<T> {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index
    }
}

impl<T> Eq for ArenaIndex<T> {}

impl<T> std::hash::Hash for ArenaIndex<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.index.hash(state);
    }
}

impl<T> Copy for ArenaIndex<T> {}

impl<T> Clone for ArenaIndex<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> ArenaIndex<T> {
    fn new(index: usize) -> Self {
        Self {
            index,
            _marker: PhantomData,
        }
    }

    pub fn empty() -> Self {
        Self::new(usize::MAX)
    }
}

pub struct NamedArena<T> {
    values: Vec<T>,
    by_name: HashMap<Ident, ArenaIndex<T>>,
}

impl<T> NamedArena<T> {
    pub fn empty() -> Self {
        Self {
            values: Vec::new(),
            by_name: HashMap::new(),
        }
    }

    pub fn insert(&mut self, name: Ident, value: T) -> ArenaIndex<T> {
        if let Some(index) = self.by_name.get(&name) {
            return *index;
        }

        let index = ArenaIndex::new(self.values.len());

        self.values.push(value);
        self.by_name.insert(name, index);

        index
    }

    pub fn get(&self, index: ArenaIndex<T>) -> &T {
        &self.values[index.index]
    }

    pub fn get_mut(&mut self, index: ArenaIndex<T>) -> &mut T {
        &mut self.values[index.index]
    }

    pub fn get_by_name(&self, name: &String) -> Option<ArenaIndex<T>> {
        let ident = get_ident(name);
        self.get_by_ident(ident)
    }

    pub fn get_by_ident(&self, name: Ident) -> Option<ArenaIndex<T>> {
        self.by_name.get(&name).copied()
    }

    pub fn value_by_ident(&self, name: Ident) -> Option<&T> {
        self.get_by_ident(name).map(|index| self.get(index))
    }

    pub fn set(&mut self, index: &ArenaIndex<T>, data: T) {
        self.values[index.index] = data;
    }
}

pub fn get_ident(name: &String) -> Ident {
    name.clone()
}

pub type TypeArena = NamedArena<CompileType>;
pub type TypeIndex = ArenaIndex<CompileType>;

pub type GenericArena = NamedArena<GenericRequire>;
pub type GenericIndex = ArenaIndex<GenericRequire>;

pub type FuncArena = NamedArena<FuncSymbol>;
pub type FuncIndex = ArenaIndex<FuncSymbol>;

pub type InterfaceArena = NamedArena<InterfaceSymbol>;
pub type InterfaceIndex = ArenaIndex<InterfaceSymbol>;
