use crate::ast::symbols::Resolution;
use std::collections::HashMap;

use super::InstantiationActives;
use crate::ast::{
    GenericIndex, TypeIndex,
    arena::{FuncIndex, InterfaceIndex},
    config::Config,
    function::{FuncBody, Interface, bindings},
    statement::Statement,
    symbols::{EnumBool, SymbolTable},
    types::ValueType,
};
use crate::parser::{Scope, out::Span};

fn build_body(
    config: &mut Config,
    scope: Option<Scope>,
    belong: EnumBool<InterfaceIndex, FuncIndex>,
    parameters: &[(TypeIndex, String)],
    generic_map: &HashMap<String, GenericIndex>,
    generics: &HashMap<GenericIndex, TypeIndex>,
    receiver: Option<TypeIndex>,
    symbols: &mut dyn Resolution,
    actives: &InstantiationActives,
    span: Span,
) -> Vec<Statement> {
    let Some(scope) = scope else {
        return Vec::new();
    };
    let types = generic_map
        .iter()
        .filter_map(|(name, index)| generics.get(index).map(|ty| (name.clone(), *ty)))
        .collect();
    let mut context = bindings::context(
        config,
        belong,
        parameters,
        generic_map,
        &types,
        receiver,
        span,
    );
    context.set_instantiation_actives(actives.clone());
    // Each statement is built once, with concrete parameter variables and the
    // newly registered symbol as its owner. No post-build substitution pass.
    Statement::parse_scope(config, scope, symbols, &mut context)
}

impl FuncBody {
    pub(super) fn instantiation(
        config: &mut Config,
        symbol: FuncIndex,
        template: FuncIndex,
        generics: &HashMap<GenericIndex, TypeIndex>,
        symbols: &mut dyn Resolution,
        actives: &InstantiationActives,
        span: Span,
    ) -> Self {
        let parameters = symbols.get_function_regular(symbol).params.clone();
        let generic_map = symbols.get_function(template, true).generic_map.clone();
        let scope = symbols.function_template(template);
        let body = build_body(
            config,
            scope,
            EnumBool::False(symbol),
            &parameters,
            &generic_map,
            generics,
            None,
            symbols,
            actives,
            span,
        );
        Self { symbol, body }
    }
}

impl Interface {
    pub(super) fn instantiation(
        config: &mut Config,
        symbol: InterfaceIndex,
        template: InterfaceIndex,
        generics: &HashMap<GenericIndex, TypeIndex>,
        symbols: &mut dyn Resolution,
        actives: &InstantiationActives,
        span: Span,
    ) -> Self {
        let concrete = symbols.get_interface_regular(symbol).clone();
        let generic_map = symbols.get_interface(template, true).generic_map.clone();
        let scope = symbols.interface_template(template);
        let receiver = concrete.has_self.then(|| {
            let value = if concrete.mutable {
                ValueType::Flex
            } else {
                ValueType::Final
            };
            concrete.owner.into_value_type(value, symbols)
        });
        let body = build_body(
            config,
            scope,
            EnumBool::True(symbol),
            &concrete.params,
            &generic_map,
            generics,
            receiver,
            symbols,
            actives,
            span,
        );
        Self { symbol, body }
    }
}
