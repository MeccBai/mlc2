use super::CompileType;
use crate::ast::{TypeArena, TypeIndex, arena::ArenaStore, symbols::SymbolTable};

/// Resolve each nested index in its own file arena.
pub trait TypeLookup {
    fn get_type(&self, index: TypeIndex) -> &CompileType;
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
