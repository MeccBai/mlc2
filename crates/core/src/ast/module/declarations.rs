use super::*;
use crate::ast::symbols::Resolution;
use crate::ast::{
    arena::get_ident,
    function::{FuncSymbol, InterfaceSymbol},
    generic::GenericRequire,
    types::{CompileType, CompileType::Unit, EnumType, UnitType},
};
use crate::parser;

pub(super) fn parse(
    mut config: &mut Config,
    mut symbols: &mut dyn Resolution,
    temp_ast: TempModule,
) -> PendingBodies {
    let (enums, mut units, funcs, temp_generics, imports, globals, _usings, interfaces) =
        super::deferred::split(temp_ast);

    imports.into_iter().for_each(|_import| {});

    enums.into_iter().for_each(|temp_enum| {
        let (enum_type, _span) = EnumType::new(&config, temp_enum);
        let ident = get_ident(&enum_type.name);
        symbols
            .local_mut()
            .types
            .insert(ident, CompileType::Enum(enum_type));
    });

    let unit_indexs = units
        .iter_mut()
        .map(|temp_unit| {
            temp_unit.name = config.symbol_name(&temp_unit.name);
            let ident = get_ident(&temp_unit.name);
            let temp = UnitType::empty();
            if temp_unit.generics.is_empty() {
                EnumBool::True(
                    symbols
                        .local_mut()
                        .types
                        .insert(ident, CompileType::Unit(temp)),
                )
            } else {
                EnumBool::False(symbols.local_mut().generics.units.insert(ident, temp))
            }
        })
        .collect::<Vec<_>>();

    temp_generics.into_iter().for_each(|temp_generic| {
        if config.is_poisoned() {
            return;
        }
        let (generic_type, _span) = GenericRequire::new(&mut config, temp_generic, &mut symbols);
        let ident = arena::get_ident(&generic_type.name);
        symbols
            .local_mut()
            .generics
            .requires
            .insert(ident, generic_type);
    });

    unit_indexs
        .into_iter()
        .zip(units.into_iter())
        .for_each(|(index, temp)| {
            if config.is_poisoned() {
                return;
            }
            match index {
                EnumBool::True(unit_index) => {
                    let (unit, _span) = UnitType::finalize(&mut config, temp, &mut symbols);
                    symbols.local_mut().types.set(&unit_index, Unit(unit));
                }
                EnumBool::False(unit_index) => {
                    let (unit, _span) = UnitType::finalize(&mut config, temp, &mut symbols);
                    symbols.local_mut().generics.units.set(&unit_index, unit);
                }
            }
        });

    crate::ast::types::unit_type::prepare_deconstructs(symbols);

    let temp_funcs = funcs
        .into_iter()
        .filter_map(|temp_func| {
            if config.is_poisoned() {
                return None;
            }
            let (symbol, body) = temp_func.split();
            let (symbol, _span) = FuncSymbol::new(&mut config, symbol, &mut symbols);
            let ident = get_ident(&symbol.name);
            let index = if symbol.has_generics() {
                let index = symbols.local_mut().generics.functions.insert(ident, symbol);
                if let Some(scope) = body.clone() {
                    symbols
                        .local_mut()
                        .generics
                        .function_templates
                        .insert(index, scope);
                }
                EnumBool::False(index)
            } else {
                EnumBool::True(symbols.local_mut().functions.insert(ident, symbol))
            };
            Some((index, body))
        })
        .collect::<Vec<_>>();

    let temp_interfaces = interfaces
        .into_iter()
        .filter_map(|temp_interface| {
            if config.is_poisoned() {
                return None;
            }
            let (symbol, body) = temp_interface.split();
            if symbol.owner.is_some() && symbol.name == "deconstruct" {
                config.submit_error(
                    crate::diagnostic::error::CompileError::IllegalUse(
                        crate::diagnostic::error::IllegalUseError::ReservedDeconstruct,
                    ),
                    symbol.name_span,
                );
                return None;
            }
            let (symbol, _span) = InterfaceSymbol::new(&mut config, symbol, &mut symbols);
            let ident = get_ident(&symbol.name);
            let index = if symbol.has_generics() {
                let index = symbols
                    .local_mut()
                    .generics
                    .interfaces
                    .insert(ident, symbol);
                if let Some(scope) = body.clone() {
                    symbols
                        .local_mut()
                        .generics
                        .interface_templates
                        .insert(index, scope);
                }
                EnumBool::False(index)
            } else {
                EnumBool::True(symbols.local_mut().interfaces.insert(ident, symbol))
            };
            Some((index, body))
        })
        .collect::<Vec<_>>();

    PendingBodies {
        globals,
        functions: temp_funcs,
        interfaces: temp_interfaces,
    }
}
