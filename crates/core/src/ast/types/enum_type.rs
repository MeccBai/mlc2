use crate::ast::config::Config;
use crate::parser::out::{Span, TempEnum};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumType {
    pub name: String,
    pub variants: Vec<String>,
    pub underlying: crate::ast::TypeIndex,
    pub values: Vec<crate::ast::expression::Expression>,
}

impl EnumType {
    pub fn new(
        config: &Config,
        prototype: TempEnum,
        underlying: crate::ast::TypeIndex,
    ) -> (Self, Span) {
        let name_span = prototype.name_span;
        (
            Self {
                name: config.symbol_name(&prototype.name),
                underlying,
                values: Vec::new(),
                variants: prototype
                    .variants
                    .into_iter()
                    .map(|(name, _)| name)
                    .collect(),
            },
            name_span,
        )
    }

    pub fn format(&self) -> String {
        self.name.clone()
    }

    pub fn dump(&self) -> String {
        format!("enum:{},variants:{:?}", self.name, self.variants)
    }

    pub fn size(&self, arena: &(impl super::TypeLookup + ?Sized)) -> usize {
        self.underlying.size(arena)
    }

    pub fn align(&self, arena: &(impl super::TypeLookup + ?Sized)) -> usize {
        self.underlying.align(arena)
    }
}
