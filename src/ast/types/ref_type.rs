use crate::ast::arena::{GenericIndex, Ident, TypeArena, TypeIndex, get_ident};
use crate::ast::types::CompileType::{self, Ref};
use lasso::Rodeo;
use std::collections::HashMap;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct RefType {
    pub base: TypeIndex,
    pub level: usize,
}

const REF_SIZE: usize = 8;

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
            Some(arena.get_by_ident(ident).unwrap())
        } else if new_type.level == 1 {
            Some(new_type.base)
        } else {
            None
        }
    }

    pub fn size(&self) -> usize {
        REF_SIZE
    }
    pub fn align(&self) -> usize {
        REF_SIZE
    }

    pub fn dump(&self, arena: &TypeArena) -> String {
        self.format(arena)
    }

    pub fn generic_instance_name(
        &self,
        arena: &TypeArena,
        params: &HashMap<GenericIndex, TypeIndex>,
    ) -> String {
        let base_name = self.base.generic_instance_name(arena, params);
        let level_str = "$".repeat(self.level);
        format!("{}{}", level_str, base_name)
    }

    pub fn instantiation(
        &self,
        params: &HashMap<GenericIndex, TypeIndex>,
        symbols: &mut crate::ast::SymbolTable,
        actives: Option<&mut HashMap<String, TypeIndex>>,
    ) -> Result<TypeIndex, crate::ast::generic::InsFailed> {
        let child = self.base.instantiation(params, symbols, actives)?;
        let instance = RefType::new(child, self.level);

        let name = instance.format(&symbols.types);
        let ident = get_ident(&name);

        if let Some(index) = symbols.types.get_by_ident(ident) {
            return Ok(index);
        }

        Ok(symbols.types.insert(ident, Ref(instance)))
    }
}
