use crate::ast::arena::{GenericIndex, Ident, TypeArena, TypeIndex, get_ident};
use crate::ast::config::Config;
use crate::ast::symbol_name::SymbolName;
use crate::ast::types::CompileType::{self, Ref};
use crate::parser::out::Span;
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
        SymbolName::reference(&self.base.format(arena), self.level)
    }

    pub fn deref(mut self, arena: &mut TypeArena) -> Option<TypeIndex> {
        if self.level > 1 {
            self.level -= 1;
            let type_str = self.format(arena);
            let ident: Ident = get_ident(&type_str);
            Some(arena.insert(ident, Ref(self)))
        } else if self.level == 1 {
            Some(self.base)
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
        SymbolName::reference(&self.base.generic_instance_name(arena, params), self.level)
    }

    pub fn instantiation(
        self,
        config: &mut Config,
        params: &HashMap<GenericIndex, TypeIndex>,
        symbols: &mut crate::ast::SymbolTable,
        actives: Option<&mut HashMap<String, TypeIndex>>,
        span: Span,
    ) -> Option<TypeIndex> {
        let child = self
            .base
            .instantiation(config, params, symbols, actives, span)?;
        let instance = RefType::new(child, self.level);

        let name = instance.format(&symbols.types);
        let ident = get_ident(&name);

        if let Some(index) = symbols.types.get_by_name(&ident) {
            return Some(index);
        }

        Some(symbols.types.insert(ident, Ref(instance)))
    }
}
