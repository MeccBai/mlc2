use crate::ast::symbols::Resolution;
use crate::ast::{
    GenericIndex, TypeIndex,
    arena::get_ident,
    config::Config,
    function::{FuncSymbol, InterfaceSymbol},
    symbol_name::SymbolName,
    symbols::{StatementContext, SymbolTable},
    types::{CompileType, RefType, UnitType},
};
use crate::diagnostic::error::{CompileError, ResolveError};
use crate::parser::out::{Spanned, TempType};
use std::collections::HashMap;

/// The generic names visible while resolving a declaration or a body.
/// Template contexts yield symbolic types; instantiated bodies yield concrete bindings.
pub trait TypeContext {
    fn resolve_generic(&self, name: &str, symbols: &mut dyn Resolution) -> Option<TypeIndex>;
}

struct EmptyContext;
impl TypeContext for EmptyContext {
    fn resolve_generic(&self, _: &str, _: &mut dyn Resolution) -> Option<TypeIndex> {
        None
    }
}

impl TypeContext for HashMap<String, TypeIndex> {
    fn resolve_generic(&self, name: &str, _: &mut dyn Resolution) -> Option<TypeIndex> {
        self.get(name).copied()
    }
}

impl TypeContext for HashMap<String, GenericIndex> {
    fn resolve_generic(&self, name: &str, symbols: &mut dyn Resolution) -> Option<TypeIndex> {
        let index = *self.get(name)?;
        Some(
            symbols
                .local_mut()
                .types
                .insert(SymbolName::generic_type(index), CompileType::Generic(index)),
        )
    }
}

impl TypeContext for StatementContext {
    fn resolve_generic(&self, name: &str, symbols: &mut dyn Resolution) -> Option<TypeIndex> {
        if let Some(ty) = self.type_bindings().get(name) {
            return Some(*ty);
        }
        match self.resolve(name) {
            Some(crate::ast::symbols::ContextBinding::Generic(index)) => Some(
                symbols
                    .local_mut()
                    .types
                    .insert(SymbolName::generic_type(index), CompileType::Generic(index)),
            ),
            _ => None,
        }
    }
}

impl TypeContext for FuncSymbol {
    fn resolve_generic(&self, name: &str, symbols: &mut dyn Resolution) -> Option<TypeIndex> {
        self.generic_map.resolve_generic(name, symbols)
    }
}

impl TypeContext for InterfaceSymbol {
    fn resolve_generic(&self, name: &str, symbols: &mut dyn Resolution) -> Option<TypeIndex> {
        self.generic_map.resolve_generic(name, symbols)
    }
}

impl TypeContext for UnitType {
    fn resolve_generic(&self, name: &str, symbols: &mut dyn Resolution) -> Option<TypeIndex> {
        self.generic_map.resolve_generic(name, symbols)
    }
}

pub fn resolve_type(
    config: &mut Config,
    temp: Spanned<TempType>,
    symbols: &mut dyn Resolution,
    context: Option<&dyn TypeContext>,
) -> Option<TypeIndex> {
    resolve_type_with_bindings(config, temp, symbols, context.unwrap_or(&EmptyContext))
}

pub fn resolve_type_with_bindings(
    config: &mut Config,
    temp: Spanned<TempType>,
    symbols: &mut dyn Resolution,
    bindings: &dyn TypeContext,
) -> Option<TypeIndex> {
    crate::TypeInfer::new(config, symbols)
        .with_context(bindings)
        .resolve_type(temp)
}

#[cfg(test)]
mod tests;
