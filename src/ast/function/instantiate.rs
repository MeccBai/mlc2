use std::{cell::RefCell, collections::HashMap, rc::Rc};

use super::{FuncBody, Interface};
use crate::ast::{
    GenericIndex, TypeIndex,
    arena::{FuncIndex, InterfaceIndex},
    config::Config,
    symbol_name::SymbolName,
    symbols::SymbolTable,
};
use crate::error::{CompileError, IllegalUseError};
use crate::parser::out::Span;
mod body;
#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstanceIndex {
    Type(TypeIndex),
    Function(FuncIndex),
    Interface(InterfaceIndex),
}

/// A shared recursion registry. No borrow is held while parsing a nested body.
pub type InstantiationActives = Rc<RefCell<HashMap<String, InstanceIndex>>>;

fn arguments(
    config: &mut Config,
    names: &[String],
    indices: &HashMap<String, GenericIndex>,
    params: &HashMap<GenericIndex, TypeIndex>,
    symbols: &SymbolTable,
    span: Span,
) -> Option<Vec<String>> {
    if config.is_poisoned() {
        return None;
    }
    let mut arguments = Vec::with_capacity(names.len());
    for name in names {
        let Some(index) = indices.get(name) else {
            config.submit_error(
                CompileError::Resolve(crate::error::ResolveError::UnknownGeneric),
                span,
            );
            return None;
        };
        let Some(&ty) = params.get(index) else {
            config.submit_error(
                CompileError::IllegalUse(IllegalUseError::GenericCountMismatch),
                span,
            );
            return None;
        };
        if !index.check(ty, symbols) {
            config.submit_error(
                CompileError::IllegalUse(IllegalUseError::RequirementUnmet),
                span,
            );
            return None;
        }
        arguments.push(ty.format(&symbols.types));
    }
    Some(arguments)
}

fn substitute(
    config: &mut Config,
    ty: TypeIndex,
    params: &HashMap<GenericIndex, TypeIndex>,
    symbols: &mut SymbolTable,
    actives: &InstantiationActives,
    span: Span,
) -> Option<TypeIndex> {
    if ty.is_empty() {
        Some(ty)
    } else {
        ty.instantiation(config, params, symbols, Some(actives), span)
    }
}

fn signature(
    config: &mut Config,
    parameters: &mut [(TypeIndex, String)],
    returns: &mut Option<TypeIndex>,
    generics: &HashMap<GenericIndex, TypeIndex>,
    symbols: &mut SymbolTable,
    actives: &InstantiationActives,
    span: Span,
) -> Option<()> {
    for (ty, _) in parameters {
        *ty = substitute(config, *ty, generics, symbols, actives, span)?;
    }
    if let Some(ty) = returns {
        *ty = substitute(config, *ty, generics, symbols, actives, span)?;
    }
    Some(())
}

impl FuncIndex {
    pub fn instantiation(
        self,
        config: &mut Config,
        params: &HashMap<GenericIndex, TypeIndex>,
        symbols: &mut SymbolTable,
        actives: Option<&InstantiationActives>,
        span: Span,
    ) -> FuncIndex {
        let local = InstantiationActives::default();
        let actives = actives.unwrap_or(&local);
        let template = symbols.generics.functions.get(self).clone();
        let Some(args) = arguments(
            config,
            &template.generics,
            &template.generic_map,
            params,
            symbols,
            span,
        ) else {
            return FuncIndex::empty();
        };
        let name = SymbolName::generic_instance(&template.name, &args);
        if let Some(InstanceIndex::Function(index)) = actives.borrow().get(&name) {
            return *index;
        }
        if let Some(index) = symbols.functions.get_by_name(&name) {
            return index;
        }
        let mut symbol = template;
        symbol.name = name.clone();
        if signature(
            config,
            &mut symbol.params,
            &mut symbol.ret_type,
            params,
            symbols,
            actives,
            span,
        )
        .is_none()
        {
            return FuncIndex::empty();
        }

        symbol.generics.clear();
        symbol.generic_map.clear();
        let index = symbols.functions.insert(name.clone(), symbol);
        actives
            .borrow_mut()
            .insert(name.clone(), InstanceIndex::Function(index));
        let body = FuncBody::instantiation(config, index, self, params, symbols, actives, span);
        actives.borrow_mut().remove(&name);
        if config.is_poisoned() {
            return FuncIndex::empty();
        }
        symbols.function_instances.insert(index, body);
        index
    }
}

impl InterfaceIndex {
    pub fn instantiation(
        self,
        config: &mut Config,
        params: &HashMap<GenericIndex, TypeIndex>,
        symbols: &mut SymbolTable,
        actives: Option<&InstantiationActives>,
        span: Span,
    ) -> InterfaceIndex {
        let local = InstantiationActives::default();
        let actives = actives.unwrap_or(&local);
        let template = symbols.generics.interfaces.get(self).clone();
        let Some(args) = arguments(
            config,
            &template.generics,
            &template.generic_map,
            params,
            symbols,
            span,
        ) else {
            return InterfaceIndex::empty();
        };
        let name = SymbolName::generic_instance(&template.name, &args);
        if let Some(InstanceIndex::Interface(index)) = actives.borrow().get(&name) {
            return *index;
        }
        if let Some(index) = symbols.interfaces.get_by_name(&name) {
            return index;
        }
        let mut symbol = template;
        symbol.name = name.clone();
        if signature(
            config,
            &mut symbol.params,
            &mut symbol.ret_type,
            params,
            symbols,
            actives,
            span,
        )
        .is_none()
        {
            return InterfaceIndex::empty();
        }
        let Some(owner) = substitute(config, symbol.owner, params, symbols, actives, span) else {
            return InterfaceIndex::empty();
        };
        symbol.owner = owner;

        symbol.generics.clear();
        symbol.generic_map.clear();
        let index = symbols.interfaces.insert(name.clone(), symbol);
        actives
            .borrow_mut()
            .insert(name.clone(), InstanceIndex::Interface(index));
        let body = Interface::instantiation(config, index, self, params, symbols, actives, span);
        actives.borrow_mut().remove(&name);
        if config.is_poisoned() {
            return InterfaceIndex::empty();
        }
        symbols.interface_instances.insert(index, body);
        index
    }
}
