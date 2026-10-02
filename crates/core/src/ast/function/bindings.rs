use crate::ast::symbols::Resolution;
use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
};

use crate::ast::{
    GenericIndex, TypeIndex,
    config::Config,
    expression::Expression,
    generic::GenericRequire,
    statement::Variable,
    symbol_name::SymbolName,
    symbols::{StatementContext, SymbolTable},
    types::{CompileType, resolve_type_with_bindings},
};
use crate::diagnostic::error::{CompileError, IllegalUseError, ResolveError};
use crate::parser::out::{Span, Spanned, TempGenericParam, TempParam, TempType};

pub(crate) fn generics(
    config: &mut Config,
    kind: &str,
    owner: &str,
    parameters: Vec<TempGenericParam>,
    symbols: &mut dyn Resolution,
) -> (
    Vec<String>,
    HashMap<String, GenericIndex>,
    HashMap<String, TypeIndex>,
) {
    let mut names = Vec::new();
    let mut indices = HashMap::new();
    let mut types = HashMap::new();
    for parameter in parameters {
        if config.is_poisoned() {
            break;
        }
        if indices.contains_key(&parameter.name) {
            config.submit_error(
                CompileError::IllegalUse(IllegalUseError::DuplicateVariable {
                    name: parameter.name,
                }),
                parameter.name_span,
            );
            break;
        }
        let identity = SymbolName::generic_param(kind, owner, &parameter.name);
        let mut require = if let Some((path, span)) = parameter.constraint {
            let name = SymbolName::path(&path.segments);
            let Some(index) = symbols
                .local()
                .generics
                .requires
                .get_by_name(&config.symbol_name(&name))
                .or_else(|| symbols.local().generics.requires.get_by_name(&name))
                .or_else(|| match symbols.named_export(config, &name) {
                    Some(crate::ast::symbols::ExportSymbol::GenericRequire(index)) => Some(index),
                    _ => None,
                })
            else {
                config.submit_error(CompileError::Resolve(ResolveError::UnknownConstraint), span);
                break;
            };
            symbols.get_generic(index).clone()
        } else {
            GenericRequire::empty(identity.clone())
        };
        require.name = identity.clone();
        let index = symbols
            .local_mut()
            .generics
            .requires
            .insert(identity.clone(), require);
        let ty = symbols
            .local_mut()
            .types
            .insert(SymbolName::generic_type(index), CompileType::Generic(index));
        types.insert(parameter.name.clone(), ty);
        indices.insert(parameter.name.clone(), index);
        names.push(parameter.name);
    }
    (names, indices, types)
}

pub(super) fn signature(
    config: &mut Config,
    parameters: Vec<TempParam>,
    returns: Option<Spanned<TempType>>,
    types: &HashMap<String, TypeIndex>,
    symbols: &mut dyn Resolution,
) -> (Vec<(TypeIndex, String)>, Option<TypeIndex>) {
    let mut seen: HashSet<String> = types.keys().cloned().collect();
    let mut params = Vec::new();
    for parameter in parameters {
        if config.is_poisoned() {
            break;
        }
        if !seen.insert(parameter.name.clone()) {
            config.submit_error(
                CompileError::IllegalUse(IllegalUseError::DuplicateVariable {
                    name: parameter.name,
                }),
                parameter.name_span,
            );
            break;
        }
        let ty = match parameter.ty {
            Some(ty) => match resolve_type_with_bindings(config, ty, symbols, types) {
                Some(ty) => ty,
                None => break,
            },
            None if parameter.name == "..." => TypeIndex::empty(),
            None => {
                config.submit_error(
                    CompileError::Resolve(ResolveError::MissingType),
                    parameter.name_span,
                );
                break;
            }
        };
        params.push((ty, parameter.name));
    }
    let ret = returns
        .and_then(|ty| resolve_type_with_bindings(config, ty, symbols, types))
        .map(|ty| ty.into_value_type(crate::ast::types::ValueType::Flex, symbols));
    (params, ret)
}

pub(super) fn context(
    config: &mut Config,
    belong: crate::ast::symbols::EnumBool<
        crate::ast::arena::InterfaceIndex,
        crate::ast::arena::FuncIndex,
    >,
    parameters: &[(TypeIndex, String)],
    generic_map: &HashMap<String, GenericIndex>,
    types: &HashMap<String, TypeIndex>,
    receiver: Option<TypeIndex>,
    span: Span,
) -> StatementContext {
    let mut context = StatementContext::new(belong);
    if let Some(ty) = receiver {
        context.insert_self(
            config,
            Rc::new(Variable {
                name: "self".into(),
                var_type: ty,
                init_val: Box::new(Expression::null()),
            }),
            span,
        );
    }
    for (name, index) in generic_map {
        if context.insert_generic_parameter(config, name.clone(), *index, span) {
            if let Some(ty) = types.get(name) {
                context.bind_generic_type(name, *ty);
            }
        }
    }
    for (ty, name) in parameters {
        if name == "..." {
            continue;
        }
        context.insert_parameter(
            config,
            Rc::new(Variable {
                name: name.clone(),
                var_type: *ty,
                init_val: Box::new(Expression::null()),
            }),
            span,
        );
    }
    context
}
