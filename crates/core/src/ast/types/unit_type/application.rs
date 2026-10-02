use crate::ast::symbols::Resolution;
use std::collections::HashMap;

use super::UnitType;
use crate::ast::{
    GenericIndex, TypeIndex, config::Config, function::InstantiationActives, generic::UnitIndex,
    symbol_name::SymbolName, symbols::SymbolTable, types::CompileType,
};
use crate::parser::out::Span;

/// A type application whose arguments are still symbolic. Keep the template
/// identity and arguments even when the Unit is empty or ignores an argument.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitApplication {
    pub template: UnitIndex,
    pub template_name: String,
    pub arguments: Vec<TypeIndex>,
}

impl UnitType {
    pub fn pending_application(
        template: UnitIndex,
        arguments: Vec<TypeIndex>,
        symbols: &mut dyn Resolution,
    ) -> TypeIndex {
        let source = symbols.get_generic_unit(template);
        let name = SymbolName::generic_instance(
            &source.name,
            &arguments
                .iter()
                .map(|ty| ty.format(symbols))
                .collect::<Vec<_>>(),
        );
        if let Some(index) = symbols.local().types.get_by_name(&name) {
            return index;
        }
        let mut unit = UnitType::empty();
        unit.name = name.clone();
        unit.attributes = source.attributes.clone();
        unit.exported = source.exported;
        unit.application = Some(UnitApplication {
            template,
            template_name: source.name.clone(),
            arguments,
        });
        symbols
            .local_mut()
            .types
            .insert(name, CompileType::Unit(unit))
    }
}

impl UnitApplication {
    pub fn instantiation(
        self,
        config: &mut Config,
        params: &HashMap<GenericIndex, TypeIndex>,
        symbols: &mut dyn Resolution,
        actives: Option<&InstantiationActives>,
        span: Span,
    ) -> Option<TypeIndex> {
        let arguments = self
            .arguments
            .into_iter()
            .map(|ty| ty.instantiation(config, params, symbols, actives, span))
            .collect::<Option<Vec<_>>>()?;
        if arguments.iter().any(|ty| ty.is_generic(symbols)) {
            return Some(UnitType::pending_application(
                self.template,
                arguments,
                symbols,
            ));
        }
        let unit = symbols.get_generic_unit(self.template).clone();
        // Retain outer bindings alongside this Unit's own parameter bindings.
        let mut merged = params.clone();
        for (name, ty) in unit.generics.iter().zip(arguments) {
            merged.insert(unit.generic_map[name], ty);
        }
        unit.instantiation(config, &merged, symbols, actives, span)
    }
}
