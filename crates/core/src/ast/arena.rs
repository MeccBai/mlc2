use crate::ast::config::FileId;
use crate::ast::function::{FuncSymbol, InterfaceSymbol};
use crate::ast::generic::GenericRequire;
use crate::ast::types::CompileType;
use std::collections::HashMap;
use std::fmt;
use std::marker::PhantomData;

pub type Ident = String;
mod store;
pub use store::ArenaStore;

pub struct ArenaIndex<T> {
    file_id: FileId,
    index: usize,
    _marker: PhantomData<fn() -> T>,
}

impl<T> fmt::Debug for ArenaIndex<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ArenaIndex({:?}, {})", self.file_id, self.index)
    }
}

impl<T> PartialEq for ArenaIndex<T> {
    fn eq(&self, other: &Self) -> bool {
        self.file_id == other.file_id && self.index == other.index
    }
}

impl<T> Eq for ArenaIndex<T> {}

impl<T> std::hash::Hash for ArenaIndex<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.index.hash(state);
        self.file_id.hash(state);
    }
}

impl<T> Copy for ArenaIndex<T> {}

impl<T> Clone for ArenaIndex<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> ArenaIndex<T> {
    fn new(file_id: FileId, index: usize) -> Self {
        Self {
            file_id,
            index,
            _marker: PhantomData,
        }
    }

    pub fn empty() -> Self {
        Self::new(FileId::new(usize::MAX), usize::MAX)
    }

    pub fn is_empty(self) -> bool {
        self.index == usize::MAX
    }

    pub fn file_id(self) -> FileId {
        self.file_id
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedArena<T> {
    file_id: FileId,
    values: Vec<T>,
    by_name: HashMap<Ident, ArenaIndex<T>>,
}

impl<T> NamedArena<T> {
    pub fn file_id(&self) -> FileId {
        self.file_id
    }

    /// Failed construction may leave an allocated slot, but must not publish
    /// its name as a valid cached instance. Existing indices are never shifted.
    pub(crate) fn forget_name(&mut self, name: &str) {
        self.by_name.remove(name);
    }

    pub fn empty() -> Self {
        Self::for_file(FileId::new(0))
    }

    pub fn for_file(file_id: FileId) -> Self {
        Self {
            file_id,
            values: Vec::new(),
            by_name: HashMap::new(),
        }
    }

    pub fn insert(&mut self, name: Ident, value: T) -> ArenaIndex<T> {
        if let Some(index) = self.by_name.get(&name) {
            return *index;
        }

        let index = ArenaIndex::new(self.file_id, self.values.len());

        self.values.push(value);
        self.by_name.insert(name, index);

        index
    }

    pub fn get(&self, index: ArenaIndex<T>) -> &T {
        assert_eq!(
            index.file_id, self.file_id,
            "index belongs to another file arena"
        );
        &self.values[index.index]
    }

    pub fn contains(&self, index: ArenaIndex<T>) -> bool {
        index.file_id == self.file_id && index.index < self.values.len()
    }

    pub fn get_mut(&mut self, index: ArenaIndex<T>) -> &mut T {
        assert_eq!(
            index.file_id, self.file_id,
            "index belongs to another file arena"
        );
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
        *self.get_mut(*index) = data;
    }

    pub fn entries(&self) -> impl Iterator<Item = (&String, ArenaIndex<T>, &T)> {
        self.by_name
            .iter()
            .map(|(name, index)| (name, *index, self.get(*index)))
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

impl FuncIndex {}
