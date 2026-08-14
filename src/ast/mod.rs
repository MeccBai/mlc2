pub(crate) mod class;
pub mod expr;
pub mod func;
pub mod stmt;
mod generic;

use std::{
    collections::HashMap,
    marker::PhantomData,
};

use crate::ast::class::CompileType;
use crate::parser::FunctionBody;
use generic::GenericDef;
use lasso::Spur;

pub type Ident = Spur;

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct ArenaIndex<T> {
    index: usize,
    _marker: PhantomData<fn() -> T>,
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

pub type TypeArena = NamedArena<CompileType>;
pub type TypeIndex = ArenaIndex<CompileType>;

pub type GenericArena = NamedArena<GenericDef>;
pub type GenericIndex = ArenaIndex<GenericDef>;

pub type FuncArena = NamedArena<FunctionBody>;
pub type FuncIndex = ArenaIndex<FunctionBody>;

