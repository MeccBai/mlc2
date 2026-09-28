use crate::ast::arena::{GenericIndex, TypeArena, get_ident};
use crate::ast::config::Config;
use crate::ast::symbol_name::SymbolName;
use crate::ast::types::CompileType::List;
use crate::ast::{SymbolTable, TypeIndex};
use crate::parser::out::Span;
use std::collections::HashMap;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct ListType {
    element_type: TypeIndex,
    length: usize,
}

impl ListType {
    pub fn new(element_type: TypeIndex, length: usize) -> Self {
        Self {
            element_type,
            length,
        }
    }

    pub fn element_type(&self) -> TypeIndex {
        self.element_type
    }

    pub fn length(&self) -> usize {
        self.length
    }

    pub fn size(&self, arena: &TypeArena) -> usize {
        self.length * self.element_type.size(arena)
    }

    pub fn align(&self, arena: &TypeArena) -> usize {
        self.element_type.align(arena)
    }

    pub fn format(&self, arena: &TypeArena) -> String {
        SymbolName::array(&self.element_type.format(arena), self.length)
    }

    pub fn dump(&self, arena: &TypeArena) -> String {
        self.format(arena)
    }

    pub fn is_generic(&self, arena: &TypeArena) -> bool {
        self.element_type.is_generic(arena)
    }

    pub fn generic_instance_name(
        &self,
        arena: &TypeArena,
        params: &HashMap<GenericIndex, TypeIndex>,
    ) -> String {
        SymbolName::array(
            &self.element_type.generic_instance_name(arena, params),
            self.length,
        )
    }

    pub fn instantiation(
        self,
        config: &mut Config,
        params: &HashMap<GenericIndex, TypeIndex>,
        symbols: &mut SymbolTable,
        actives: Option<&mut HashMap<String, TypeIndex>>,
        span: Span,
    ) -> Option<TypeIndex> {
        let child = self
            .element_type
            .instantiation(config, params, symbols, actives, span)?;
        let instance = ListType::new(child, self.length);

        let name = instance.format(&symbols.types);
        let ident = get_ident(&name);

        if let Some(index) = symbols.types.get_by_name(&ident) {
            return Some(index);
        }

        Some(symbols.types.insert(ident, List(instance)))
    }
}
