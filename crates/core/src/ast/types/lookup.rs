use super::CompileType;
use crate::ast::{TypeArena, TypeIndex, arena::ArenaStore, symbols::SymbolTable};

/// Resolve each nested index in its own file arena.
pub trait TypeLookup {
    fn get_type(&self, index: TypeIndex) -> &CompileType;
}

/// Derived types are interned in the active file; their bases keep original indices.
pub trait TypeStorage: TypeLookup {
    fn insert_type(&mut self, name: String, ty: CompileType) -> TypeIndex;
}

impl TypeStorage for TypeArena {
    fn insert_type(&mut self, name: String, ty: CompileType) -> TypeIndex {
        self.insert(name, ty)
    }
}

impl TypeLookup for SymbolTable {
    fn get_type(&self, index: TypeIndex) -> &CompileType {
        self.types.get(index)
    }
}

impl TypeLookup for crate::ast::symbols::PackageSymbolTable {
    fn get_type(&self, index: TypeIndex) -> &CompileType {
        crate::ast::symbols::PackageSymbolTable::get_type(self, index)
    }
}

impl<T: TypeLookup + ?Sized> TypeLookup for &mut T {
    fn get_type(&self, index: TypeIndex) -> &CompileType {
        (**self).get_type(index)
    }
}

impl<T: TypeStorage + ?Sized> TypeStorage for &mut T {
    fn insert_type(&mut self, name: String, ty: CompileType) -> TypeIndex {
        (**self).insert_type(name, ty)
    }
}

impl TypeLookup for TypeArena {
    fn get_type(&self, index: TypeIndex) -> &CompileType {
        self.get(index)
    }
}

impl TypeLookup for ArenaStore<SymbolTable> {
    fn get_type(&self, index: TypeIndex) -> &CompileType {
        self.get(index.file_id())
            .expect("registered type arena")
            .types
            .get(index)
    }
}

#[cfg(test)]
mod tests;
