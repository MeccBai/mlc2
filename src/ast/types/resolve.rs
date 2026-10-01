use crate::ast::{
    GenericIndex, TypeIndex,
    arena::get_ident,
    config::Config,
    function::{FuncSymbol, InterfaceSymbol},
    symbol_name::SymbolName,
    symbols::{StatementContext, SymbolTable},
    types::{CompileType, RefType, UnitType},
};
use crate::error::{CompileError, ResolveError};
use crate::parser::out::{Spanned, TempType};
use std::collections::HashMap;

/// The generic names visible while resolving a declaration or a body.
/// Template contexts yield symbolic types; instantiated bodies yield concrete bindings.
pub trait TypeContext {
    fn resolve_generic(&self, name: &str, symbols: &mut SymbolTable) -> Option<TypeIndex>;
}

struct EmptyContext;
impl TypeContext for EmptyContext {
    fn resolve_generic(&self, _: &str, _: &mut SymbolTable) -> Option<TypeIndex> {
        None
    }
}

impl TypeContext for HashMap<String, TypeIndex> {
    fn resolve_generic(&self, name: &str, _: &mut SymbolTable) -> Option<TypeIndex> {
        self.get(name).copied()
    }
}

impl TypeContext for HashMap<String, GenericIndex> {
    fn resolve_generic(&self, name: &str, symbols: &mut SymbolTable) -> Option<TypeIndex> {
        let index = *self.get(name)?;
        Some(
            symbols
                .types
                .insert(SymbolName::generic_type(index), CompileType::Generic(index)),
        )
    }
}

impl TypeContext for StatementContext {
    fn resolve_generic(&self, name: &str, symbols: &mut SymbolTable) -> Option<TypeIndex> {
        if let Some(ty) = self.type_bindings().get(name) {
            return Some(*ty);
        }
        match self.resolve(name) {
            Some(crate::ast::symbols::ContextBinding::Generic(index)) => Some(
                symbols
                    .types
                    .insert(SymbolName::generic_type(index), CompileType::Generic(index)),
            ),
            _ => None,
        }
    }
}

impl TypeContext for FuncSymbol {
    fn resolve_generic(&self, name: &str, symbols: &mut SymbolTable) -> Option<TypeIndex> {
        self.generic_map.resolve_generic(name, symbols)
    }
}

impl TypeContext for InterfaceSymbol {
    fn resolve_generic(&self, name: &str, symbols: &mut SymbolTable) -> Option<TypeIndex> {
        self.generic_map.resolve_generic(name, symbols)
    }
}

impl TypeContext for UnitType {
    fn resolve_generic(&self, name: &str, symbols: &mut SymbolTable) -> Option<TypeIndex> {
        self.generic_map.resolve_generic(name, symbols)
    }
}

pub fn resolve_type(
    config: &mut Config,
    temp: Spanned<TempType>,
    symbols: &mut SymbolTable,
    context: Option<&dyn TypeContext>,
) -> Option<TypeIndex> {
    resolve_type_with_bindings(config, temp, symbols, context.unwrap_or(&EmptyContext))
}

pub fn resolve_type_with_bindings(
    config: &mut Config,
    temp: Spanned<TempType>,
    symbols: &mut SymbolTable,
    bindings: &dyn TypeContext,
) -> Option<TypeIndex> {
    if config.is_poisoned() {
        return None;
    }
    let (temp, span) = temp;
    match temp {
        TempType::Path(path) => {
            let path = SymbolName::path(&path.segments);
            if let Some(ty) = bindings.resolve_generic(&path, symbols) {
                return Some(ty);
            }
            let local = config.symbol_name(&path);
            match symbols
                .types
                .get_by_name(&local)
                .or_else(|| symbols.types.get_by_name(&path))
                .or_else(|| {
                    symbols.using_target(config, &path).and_then(|name| {
                        symbols
                            .types
                            .get_by_name(&name)
                            .or_else(|| symbols.types.get_by_name(&config.symbol_name(&name)))
                    })
                }) {
                Some(ty) => Some(ty),
                None => {
                    config.submit_error(CompileError::Resolve(ResolveError::UnknownType), span);
                    None
                }
            }
        }
        TempType::Generic { base, args } => {
            let path = SymbolName::path(&base.segments);
            let name = config.symbol_name(&path);
            let index = match symbols
                .generics
                .units
                .get_by_name(&name)
                .or_else(|| symbols.generics.units.get_by_name(&path))
                .or_else(|| {
                    symbols.using_target(config, &path).and_then(|name| {
                        symbols.generics.units.get_by_name(&name).or_else(|| {
                            symbols
                                .generics
                                .units
                                .get_by_name(&config.symbol_name(&name))
                        })
                    })
                }) {
                Some(index) => index,
                None => {
                    config.submit_error(CompileError::Resolve(ResolveError::UnknownGeneric), span);
                    return None;
                }
            };

            let unit = symbols.generics.units.get(index).clone();
            if args.len() != unit.generics.len() {
                config.submit_error(
                    CompileError::IllegalUse(crate::error::IllegalUseError::GenericCountMismatch),
                    span,
                );
                return None;
            }
            let resolved = args
                .into_iter()
                .map(|arg| resolve_type_with_bindings(config, arg, symbols, bindings))
                .collect::<Option<Vec<_>>>()?;

            let mut params = HashMap::new();
            for (name, ty) in unit.generics.iter().zip(resolved) {
                let Some(&index) = unit.generic_map.get(name) else {
                    config.submit_error(CompileError::Resolve(ResolveError::UnknownGeneric), span);
                    return None;
                };
                params.insert(index, ty);
            }
            unit.instantiation(config, &params, symbols, None, span)
        }
        TempType::Reference { inner, mutable } => {
            let base = resolve_type_with_bindings(config, *inner, symbols, bindings)?;
            let new_ref = match symbols.types.get(base).unqualified() {
                CompileType::Ref(reference) => {
                    RefType::new(reference.base, reference.level + 1, mutable)
                }
                _ => RefType::new(base, 1, mutable),
            };
            let type_str = new_ref.format(&symbols.types);
            Some(
                symbols
                    .types
                    .insert(get_ident(&type_str), CompileType::Ref(new_ref)),
            )
        }
    }
}

#[cfg(test)]
mod tests;
