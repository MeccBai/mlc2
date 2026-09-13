use std::collections::HashMap;
use std::fmt;
use std::marker::PhantomData;
use lasso::{Rodeo, Spur};
use crate::ast::func::FuncBody;
use crate::ast::generic::GenericRequire;
use crate::ast::types::CompileType;

pub type Ident = Spur;

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

    pub fn get_by_name(&self, name: Ident) -> Option<ArenaIndex<T>> {
        self.by_name.get(&name).copied()
    }

    pub fn value_by_name(&self, name: Ident) -> Option<&T> {
        self.get_by_name(name).map(|index| self.get(index))
    }
}

pub fn get_ident(name:&String) -> Ident {
    let mut interner = Rodeo::default();
    interner.get_or_intern(name)
}

pub type TypeArena = NamedArena<CompileType>;
pub type TypeIndex = ArenaIndex<CompileType>;

pub type GenericArena = NamedArena<GenericRequire>;
pub type GenericIndex = ArenaIndex<GenericRequire>;

pub type FuncArena = NamedArena<FuncBody>;
pub type FuncIndex = ArenaIndex<FuncBody>;
