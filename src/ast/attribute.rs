use crate::lexer::Span;
use crate::{ast::config::Config, diagnostic::warning::Warning};
use std::{
    collections::{HashMap, HashSet},
    sync::LazyLock,
};

static FUNC_ATTRIBUTE_MAP: LazyLock<HashMap<&'static str, FuncAttibute>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    m.insert("c_abi", FuncAttibute::Cabi);
    m
});

static UNIT_ATTRIBUTE_MAP: LazyLock<HashMap<&'static str, UnitAttribute>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    m.insert("c_abi", UnitAttribute::Cabi);
    m
});

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum FuncAttibute {
    Empty,
    Cabi,
}
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum UnitAttribute {
    Empty,
    Cabi,
}

impl FuncAttibute {
    pub fn parse(config: &mut Config, names: Vec<String>, span: Span) -> HashSet<Self> {
        names
            .into_iter()
            .map(|name| Self::new(config, &name, span))
            .filter(|attribute| *attribute != Self::Empty)
            .collect()
    }
    pub fn new(config: &mut Config, name: &str, span: Span) -> Self {
        match FUNC_ATTRIBUTE_MAP.get(name) {
            Some(attr) => attr.clone(),
            None => {
                config.submit_warning(Warning::AttributeNotFound, span);
                FuncAttibute::Empty
            }
        }
    }
}

impl UnitAttribute {
    pub fn parse(config: &mut Config, names: Vec<String>, span: Span) -> HashSet<Self> {
        names
            .into_iter()
            .map(|name| Self::new(config, &name, span))
            .filter(|attribute| *attribute != Self::Empty)
            .collect()
    }
    pub fn new(config: &mut Config, name: &str, span: Span) -> Self {
        match UNIT_ATTRIBUTE_MAP.get(name) {
            Some(attr) => attr.clone(),
            None => {
                config.submit_warning(Warning::AttributeNotFound, span);
                UnitAttribute::Empty
            }
        }
    }
}

#[cfg(test)]
mod tests;
