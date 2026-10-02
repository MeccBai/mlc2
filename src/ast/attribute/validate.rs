use super::{FuncAttibute, UnitAttribute};
use crate::ast::{Config, TypeIndex, symbols::SymbolTable, types::CompileType};
use crate::diagnostic::error::{CAbiError, CompileError, IllegalUseError};
use crate::lexer::Span;
use std::collections::{HashMap, HashSet};

pub(crate) fn reject(config: &mut Config, error: CAbiError, span: Span) {
    config.submit_error(CompileError::IllegalUse(IllegalUseError::CAbi(error)), span);
}

pub(crate) fn callable(
    config: &mut Config,
    attributes: &HashSet<FuncAttibute>,
    generic: bool,
    span: Span,
) {
    if generic && attributes.contains(&FuncAttibute::Cabi) {
        reject(config, CAbiError::GenericFunction, span);
    }
}

pub(crate) fn parameters(
    config: &mut Config,
    attributes: &HashSet<FuncAttibute>,
    params: &[(TypeIndex, String)],
    symbols: &SymbolTable,
    span: Span,
) {
    if config.is_poisoned() || !attributes.contains(&FuncAttibute::Cabi) {
        return;
    }
    if params
        .iter()
        .any(|(ty, _)| contains_non_c_unit(*ty, symbols, &mut HashSet::new()))
    {
        reject(config, CAbiError::NonCAbiUnitParameter, span);
    }
}

fn contains_non_c_unit(
    ty: TypeIndex,
    symbols: &SymbolTable,
    visited: &mut HashSet<TypeIndex>,
) -> bool {
    if ty.is_empty() || !visited.insert(ty) {
        return false;
    }
    match symbols.types.get(ty).unqualified() {
        CompileType::Unit(unit) => !unit.attributes.contains(&UnitAttribute::Cabi),
        CompileType::Ref(reference) => contains_non_c_unit(reference.base, symbols, visited),
        CompileType::List(list) => contains_non_c_unit(list.element_type, symbols, visited),
        _ => false,
    }
}
