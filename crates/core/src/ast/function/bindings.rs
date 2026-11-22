use crate::ast::{
    arena,
    symbols::{self, Resolution},
    types,
};
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
    for parameter in &parameters {
        if indices.contains_key(&parameter.name) {
            config.submit_error(
                CompileError::IllegalUse(IllegalUseError::DuplicateVariable {
                    name: parameter.name.clone(),
                }),
                parameter.name_span,
            );
            break;
        }
        let identity = SymbolName::generic_param(kind, owner, &parameter.name);
        let index = symbols
            .local_mut()
            .generics
            .requires
            .insert(identity.clone(), GenericRequire::empty(identity));
        let ty = symbols
            .local_mut()
            .types
            .insert(SymbolName::generic_type(index), CompileType::Generic(index));
        types.insert(parameter.name.clone(), ty);
        indices.insert(parameter.name.clone(), index);
        names.push(parameter.name.clone());
    }
    for parameter in parameters {
        if config.is_poisoned() {
            break;
        }
        let Some((path, span)) = parameter.constraint else {
            continue;
        };
        let Some(template) = GenericRequire::find(config, &path, symbols) else {
            config.submit_error(CompileError::Resolve(ResolveError::UnknownConstraint), span);
            break;
        };
        let arguments = parameter
            .constraint_args
            .into_iter()
            .map(|ty| resolve_type_with_bindings(config, ty, symbols, &types))
            .collect::<Option<Vec<_>>>();
        let Some(arguments) = arguments else { break };
        let requirement = symbols.get_generic(template);
        if requirement.parameters.len() != arguments.len() {
            config.submit_error(
                CompileError::IllegalUse(IllegalUseError::GenericCountMismatch),
                span,
            );
            break;
        }
        let index = indices[&parameter.name];
        let mut require = if arguments.is_empty() {
            requirement.clone()
        } else {
            let mut require = GenericRequire::empty(parameter.name);
            require.application = Some((template, arguments));
            require
        };
        require.name = symbols.get_generic(index).name.clone();
        require.exported = false;
        symbols.local_mut().generics.requires.set(&index, require);
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
        .map(|ty| ty.with_immutability(false, symbols));
    (params, ret)
}

pub(super) fn context(
    config: &mut Config,
    belong: symbols::EnumBool<arena::InterfaceIndex, arena::FuncIndex>,
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
                constant: false,
                read_count: Default::default(),
                declaration_span: (0..0).into(),
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
                constant: false,
                read_count: Default::default(),
                declaration_span: (0..0).into(),
                name: name.clone(),
                var_type: *ty,
                init_val: Box::new(Expression::null()),
            }),
            span,
        );
    }
    context
}
