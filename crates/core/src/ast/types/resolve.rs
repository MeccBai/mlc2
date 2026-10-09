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
    if config.is_poisoned() {
        return None;
    }
    let (temp, span) = temp;
    match temp {
        TempType::Function {
            params,
            returns,
            variadic,
        } => {
            let params = params
                .into_iter()
                .map(|ty| resolve_type_with_bindings(config, ty, symbols, bindings))
                .collect::<Option<Vec<_>>>()?;
            let returns = match returns {
                Some(ty) => Some(resolve_type_with_bindings(config, *ty, symbols, bindings)?),
                None => None,
            };
            Some(super::FunctionType::declare(
                params, returns, variadic, symbols,
            ))
        }
        TempType::InferredResource => {
            config.submit_error(
                CompileError::IllegalUse(
                    crate::diagnostic::error::IllegalUseError::ResourceInferenceRequiresInitializer,
                ),
                span,
            );
            None
        }
        TempType::Array { element, length } => {
            let element = resolve_type_with_bindings(config, *element, symbols, bindings)?;
            let array = super::ListType::new(element, length);
            let name = array.format(symbols);
            Some(
                symbols
                    .local_mut()
                    .types
                    .insert(name, CompileType::List(array)),
            )
        }
        TempType::Path(path) => {
            let path = SymbolName::path(&path.segments);
            if let Some(ty) = bindings.resolve_generic(&path, symbols) {
                return Some(ty);
            }
            let local = config.symbol_name(&path);
            match symbols
                .local_type(&local)
                .or_else(|| symbols.local_type(&path))
                .or_else(|| match symbols.named_export(config, &path) {
                    Some(crate::ast::symbols::ExportSymbol::Type(index)) => Some(index),
                    _ => None,
                })
                .or_else(|| {
                    symbols.using_target(config, &path).and_then(|name| {
                        symbols
                            .local_type(&name)
                            .or_else(|| symbols.local_type(&config.symbol_name(&name)))
                            .or_else(|| match symbols.named_export(config, &name) {
                                Some(crate::ast::symbols::ExportSymbol::Type(index)) => Some(index),
                                _ => None,
                            })
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
                .local()
                .generics
                .units
                .get_by_name(&name)
                .or_else(|| symbols.local().generics.units.get_by_name(&path))
                .or_else(|| match symbols.named_export(config, &path) {
                    Some(crate::ast::symbols::ExportSymbol::GenericUnit(index)) => Some(index),
                    _ => None,
                })
                .or_else(|| {
                    symbols.using_target(config, &path).and_then(|name| {
                        symbols
                            .local()
                            .generics
                            .units
                            .get_by_name(&name)
                            .or_else(|| {
                                symbols
                                    .local()
                                    .generics
                                    .units
                                    .get_by_name(&config.symbol_name(&name))
                            })
                            .or_else(|| match symbols.named_export(config, &name) {
                                Some(crate::ast::symbols::ExportSymbol::GenericUnit(index)) => {
                                    Some(index)
                                }
                                _ => None,
                            })
                    })
                }) {
                Some(index) => index,
                None => {
                    config.submit_error(CompileError::Resolve(ResolveError::UnknownGeneric), span);
                    return None;
                }
            };

            let unit = symbols.get_generic_unit(index).clone();
            if args.len() != unit.generics.len() {
                config.submit_error(
                    CompileError::IllegalUse(
                        crate::diagnostic::error::IllegalUseError::GenericCountMismatch,
                    ),
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
        TempType::Resource { inner } => {
            let index = resolve_type_with_bindings(config, *inner, symbols, bindings)?;
            let CompileType::Ref(reference) = symbols.get_type(index).unqualified() else {
                config.submit_error(
                    CompileError::IllegalUse(
                        crate::diagnostic::error::IllegalUseError::TypeMismatched {
                            expected: "reference after res".into(),
                            found: index.format(symbols),
                        },
                    ),
                    span,
                );
                return None;
            };
            let reference = reference.clone().owned();
            let name = reference.format(symbols);
            Some(
                symbols
                    .local_mut()
                    .types
                    .insert(get_ident(&name), CompileType::Ref(reference)),
            )
        }
        TempType::Reference { inner, mutable } => {
            let base = resolve_type_with_bindings(config, *inner, symbols, bindings)?;
            let new_ref = match symbols.get_type(base).unqualified() {
                CompileType::Ref(reference) if !reference.ownership => {
                    RefType::new(reference.base, reference.level + 1, mutable)
                }
                _ => RefType::new(base, 1, mutable),
            };
            let type_str = new_ref.format(symbols);
            Some(
                symbols
                    .local_mut()
                    .types
                    .insert(get_ident(&type_str), CompileType::Ref(new_ref)),
            )
        }
    }
}

#[cfg(test)]
mod tests;
