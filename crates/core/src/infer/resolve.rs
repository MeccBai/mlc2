use super::TypeInfer;
use crate::ast::{
    TypeIndex,
    arena::get_ident,
    symbol_name::SymbolName,
    symbols::Resolution,
    types::{CompileType, RefType, TypeContext, resolve_type_with_bindings},
};
use crate::diagnostic::error::{CompileError, ResolveError};
use crate::parser::out::{Spanned, TempType};
use std::collections::HashMap;
struct EmptyBindings;
impl TypeContext for EmptyBindings {
    fn resolve_generic(&self, _: &str, _: &mut dyn Resolution) -> Option<TypeIndex> {
        None
    }
}
impl TypeInfer<'_> {
    pub fn resolve_type(&mut self, temp: Spanned<TempType>) -> Option<TypeIndex> {
        let config = &mut *self.config;
        let symbols = &mut *self.symbols;
        let bindings = self.context.unwrap_or(&EmptyBindings);
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
                Some(crate::ast::types::FunctionType::declare(
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
                let array = crate::ast::types::ListType::new(element, length);
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
                                    Some(crate::ast::symbols::ExportSymbol::Type(index)) => {
                                        Some(index)
                                    }
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
                        config.submit_error(
                            CompileError::Resolve(ResolveError::UnknownGeneric),
                            span,
                        );
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
                        config.submit_error(
                            CompileError::Resolve(ResolveError::UnknownGeneric),
                            span,
                        );
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
}
