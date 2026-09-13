use crate::ast::types::CompileType;
use crate::ast::arena::{TypeArena,TypeIndex,Ident};
use lasso::Rodeo;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct RefType {
    pub base: TypeIndex,
    pub level: usize,
}

impl RefType {
    pub fn new(base: TypeIndex, level: usize) -> Self {
        Self { base, level }
    }

    pub fn format(&self, arena: &TypeArena) -> String {
        let level = std::format!("{}", "$".repeat(self.level));
        let type_name = self.base.format(arena);
        format!("{}{}", level, type_name)
    }

    pub fn deref(&self, arena: &mut TypeArena) -> Option<TypeIndex> {
        let new_type = self.clone();
        if new_type.level > 1 {
            let deref_type = RefType::new(new_type.base, new_type.level - 1);
            let type_str = deref_type.format(arena);
            let mut interner = Rodeo::default();
            let ident: Ident = interner.get_or_intern(type_str);
            arena.insert(ident, CompileType::Ref(deref_type));
            Some(arena.get_by_name(ident).unwrap())
        } else if new_type.level == 1 {
            Some(new_type.base)
        } else {
            None
        }
    }
}
