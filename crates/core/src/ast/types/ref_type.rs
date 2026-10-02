use crate::ast::arena::{GenericIndex, Ident, TypeArena, TypeIndex, get_ident};
use crate::ast::config::Config;
use crate::ast::symbol_name::SymbolName;
use crate::ast::symbols::Resolution;
use crate::ast::{
    symbols::{EnumBool, SymbolTable},
    types::CompileType::{self, Ref},
};
use crate::parser::out::Span;
use std::collections::HashMap;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct RefType {
    pub base: TypeIndex,
    pub mut_base: bool,
    pub level: usize,
}

const REF_SIZE: usize = 8;
#[cfg(test)]
mod tests;

impl RefType {
    pub fn new(base: TypeIndex, level: usize, mut_base: bool) -> Self {
        Self {
            base,
            level,
            mut_base,
        }
    }

    pub fn format(&self, arena: &(impl crate::ast::types::TypeLookup + ?Sized)) -> String {
        SymbolName::reference(&self.base.format(arena), self.level, self.mut_base)
    }

    pub fn deref(
        mut self,
        arena: &mut (impl crate::ast::types::TypeStorage + ?Sized),
    ) -> Option<TypeIndex> {
        if self.level > 1 {
            self.level -= 1;
            let type_str = self.format(arena);
            let ident: Ident = get_ident(&type_str);
            Some(arena.insert_type(ident, Ref(self)))
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

    pub fn dump(&self, arena: &(impl crate::ast::types::TypeLookup + ?Sized)) -> String {
        self.format(arena)
    }

    pub fn generic_instance_name(
        &self,
        arena: &(impl crate::ast::types::TypeLookup + ?Sized),
        params: &HashMap<GenericIndex, TypeIndex>,
    ) -> String {
        SymbolName::reference(
            &self.base.generic_instance_name(arena, params),
            self.level,
            self.mut_base,
        )
    }

    pub fn instantiation(
        self,
        config: &mut Config,
        params: &HashMap<GenericIndex, TypeIndex>,
        symbols: &mut dyn Resolution,
        actives: Option<&crate::ast::function::InstantiationActives>,
        span: Span,
    ) -> Option<TypeIndex> {
        let child = self
            .base
            .instantiation(config, params, symbols, actives, span)?;
        let instance = RefType::new(child, self.level, self.mut_base);

        let name = instance.format(symbols);
        let ident = get_ident(&name);

        if let Some(index) = symbols.local().types.get_by_name(&ident) {
            return Some(index);
        }

        Some(symbols.local_mut().types.insert(ident, Ref(instance)))
    }

    pub fn type_check(
        &self,
        other: &RefType,
        symbols: &(impl crate::ast::types::TypeLookup + ?Sized),
    ) -> bool {
        if !self.base.type_check(false, &other.base, symbols) {
            return false;
        }
        if self.level != other.level {
            return false;
        }
        if self.mut_base == true && other.mut_base == false {
            return false;
        }
        true
    }
}
