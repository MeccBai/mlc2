use crate::ast::config::Config;
use crate::parser::out::{Span, TempEnum};

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct EnumType {
    pub name: String,
    pub variants: Vec<String>,
}

impl EnumType {
    pub fn new(config: &Config, prototype: TempEnum) -> (Self, Span) {
        let name_span = prototype.name_span;
        (
            Self {
                name: config.symbol_name(&prototype.name),
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

    pub fn size(&self) -> usize {
        4
    }

    pub fn align(&self) -> usize {
        4
    }
}
