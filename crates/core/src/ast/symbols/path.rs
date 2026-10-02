use std::rc::Rc;

use super::{StatementContext, SymbolTable};
use crate::ast::{
    Config, GenericIndex,
    arena::{FuncIndex, InterfaceIndex},
    statement::Variable,
    symbol_name::SymbolName,
};
use crate::parser::out::TempPath;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathSymbolKind {
    GenericParameter,
    LocalVariable,
    EnumValue,
    GlobalVariable,
    Interface,
    Function,
    Using,
}

pub const PATH_SEARCH_ORDER: [PathSymbolKind; 7] = [
    PathSymbolKind::GenericParameter,
    PathSymbolKind::LocalVariable,
    PathSymbolKind::EnumValue,
    PathSymbolKind::GlobalVariable,
    PathSymbolKind::Interface,
    PathSymbolKind::Function,
    PathSymbolKind::Using,
];

#[derive(Debug, Clone)]
pub enum PathSymbol {
    GenericParameter(GenericIndex),
    Variable(Rc<Variable>),
    EnumValue(crate::ast::expression::EnumValue),
    Interface {
        index: InterfaceIndex,
        generic: bool,
    },
    Function {
        index: FuncIndex,
        generic: bool,
    },
}

impl SymbolTable {
    /// Resolve once, then let the use site check whether this symbol is usable.
    /// Explicit generic arguments never bypass a higher-priority binding.
    pub fn resolve_path(
        &self,
        config: &Config,
        path: &TempPath,
        context: Option<&StatementContext>,
    ) -> Option<PathSymbol> {
        for kind in PATH_SEARCH_ORDER {
            if let Some(found) = self.resolve_kind(config, path, context, kind) {
                return Some(found);
            }
        }
        None
    }

    pub(crate) fn resolve_kind(
        &self,
        config: &Config,
        path: &TempPath,
        context: Option<&StatementContext>,
        kind: PathSymbolKind,
    ) -> Option<PathSymbol> {
        let name = SymbolName::path(&path.segments);
        let prefix = SymbolName::member(&config.symbol_prefix(), "");
        let name = name.strip_prefix(&prefix).unwrap_or(&name).to_owned();
        let qualified = config.symbol_name(&name);
        match kind {
            PathSymbolKind::GenericParameter => context
                .and_then(|ctx| ctx.generic_parameter(&name))
                .map(PathSymbol::GenericParameter),
            PathSymbolKind::LocalVariable => context
                .and_then(|ctx| ctx.lookup(&name))
                .cloned()
                .map(PathSymbol::Variable),
            PathSymbolKind::EnumValue => self
                .resolve_enum_value(config, path)
                .map(PathSymbol::EnumValue),
            PathSymbolKind::GlobalVariable => self
                .globals
                .get(&name)
                .or_else(|| self.globals.get(&qualified))
                .map(|(_, variable)| PathSymbol::Variable(variable.clone())),
            PathSymbolKind::Interface => self
                .interfaces
                .get_by_name(&name)
                .or_else(|| self.interfaces.get_by_name(&qualified))
                .map(|index| PathSymbol::Interface {
                    index,
                    generic: false,
                })
                .or_else(|| {
                    self.generics
                        .interfaces
                        .get_by_name(&name)
                        .or_else(|| self.generics.interfaces.get_by_name(&qualified))
                        .map(|index| PathSymbol::Interface {
                            index,
                            generic: true,
                        })
                }),
            PathSymbolKind::Using => {
                let target = self
                    .usings
                    .get(&name)
                    .or_else(|| self.usings.get(&qualified));
                target
                    .and_then(|target| self.expand_using(target).ok())
                    .and_then(|target| {
                        self.resolve_path(
                            config,
                            &TempPath {
                                segments: target.split("::").map(str::to_owned).collect(),
                            },
                            None,
                        )
                    })
            }
            PathSymbolKind::Function => self
                .functions
                .get_by_name(&name)
                .or_else(|| self.functions.get_by_name(&qualified))
                .map(|index| PathSymbol::Function {
                    index,
                    generic: false,
                })
                .or_else(|| {
                    self.generics
                        .functions
                        .get_by_name(&name)
                        .or_else(|| self.generics.functions.get_by_name(&qualified))
                        .map(|index| PathSymbol::Function {
                            index,
                            generic: true,
                        })
                }),
        }
    }

    fn resolve_enum_value(
        &self,
        config: &Config,
        path: &TempPath,
    ) -> Option<crate::ast::expression::EnumValue> {
        let (variant, owner) = path.segments.split_last()?;
        if owner.is_empty() {
            return None;
        }
        let name = SymbolName::path(owner);
        let index = self
            .types
            .get_by_name(&name)
            .or_else(|| self.types.get_by_name(&config.symbol_name(&name)))?;
        let crate::ast::types::CompileType::Enum(enumeration) = self.types.get(index).unqualified()
        else {
            return None;
        };
        let value = enumeration
            .variants
            .iter()
            .position(|name| name == variant)?;
        Some(crate::ast::expression::EnumValue {
            enum_type: index,
            value,
        })
    }
}
