pub mod arena;
pub mod config;
pub mod expression;
pub mod function;
pub mod generic;
pub mod statement;
pub mod symbol_name;
pub mod symbols;
pub mod types;

#[cfg(test)]
mod tests;

use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use crate::ast::function::{FuncSymbol, Interface, InterfaceSymbol};
use crate::ast::generic::GenericTable;
use crate::ast::statement::Variable;
use crate::ast::types::CompileType::{Base, Enum, List, Ref, Unit};
use crate::ast::types::base_type::DataType;
use crate::ast::types::{BaseType, CompileType};
use crate::ast::types::{EnumType, UnitType};
use crate::ast::{
    arena::{InterfaceArena, get_ident},
    symbols::{EnumBool, SymbolTable},
};
use crate::error::ice::ice;
use crate::error::{CompileError, ErrorHandle, ResolveError};
use crate::parser::out::{
    TempEnum, TempFunc, TempGeneric, TempInterface, TempUnit, TempUsing, TempVar,
};
use crate::parser::{TempGlobalStmt, TempModule};
use arena::{FuncArena, FuncIndex, GenericIndex, TypeArena, TypeIndex};
use config::Config;
use function::FuncBody;
use generic::GenericRequire;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportModule {
    path: Vec<String>,
    export: bool,
}

impl ImportModule {
    pub fn new(path: Vec<String>, export: bool) -> Self {
        Self { path, export }
    }

    pub fn path(&self) -> &[String] {
        &self.path
    }

    pub fn exported(&self) -> bool {
        self.export
    }
}

pub enum GlobalStatement {
    EnumDef(EnumType),
    UnitDef(UnitType),
    FuncDef(FuncBody),
    GenericDef(GenericRequire),
    Import(ImportModule),
    GlobalVar(Variable),
}

pub enum Function {
    Func(FuncBody),
    Interface(Interface),
}

pub struct AbstractSyntaxTree {
    pub symbols: SymbolTable,
    pub body: Vec<Function>,
    pub config: Config,
}

impl AbstractSyntaxTree {
    fn split(
        temp_ast: TempModule,
    ) -> (
        Vec<TempEnum>,
        Vec<TempUnit>,
        Vec<TempFunc>,
        Vec<TempGeneric>,
        Vec<ImportModule>,
        Vec<TempVar>,
        Vec<TempUsing>,
        Vec<TempInterface>,
    ) {
        let mut enums = Vec::<TempEnum>::new();
        let mut units = Vec::<TempUnit>::new();
        let mut funcs = Vec::<TempFunc>::new();
        let mut generics = Vec::<TempGeneric>::new();
        let mut imports = Vec::<ImportModule>::new();
        let mut globals = Vec::<TempVar>::new();
        let mut usings = Vec::<TempUsing>::new();
        let mut interfaces = Vec::<TempInterface>::new();

        temp_ast.into_iter().for_each(|(stmt, _span)| match stmt {
            TempGlobalStmt::Unit(temp_unit) => units.push(temp_unit),
            TempGlobalStmt::Func(temp_func) => funcs.push(temp_func),
            TempGlobalStmt::Using(temp_using) => usings.push(temp_using),
            TempGlobalStmt::Generic(temp_generic) => generics.push(temp_generic),
            TempGlobalStmt::Enum(temp_enum) => enums.push(temp_enum),
            TempGlobalStmt::Import(temp_import) => imports.push(temp_import),
            TempGlobalStmt::Variable(temp_variable) => globals.push(temp_variable),
            TempGlobalStmt::Interface(interface) => interfaces.push(interface),
        });

        (
            enums, units, funcs, generics, imports, globals, usings, interfaces,
        )
    }

    pub fn new(mut config: Config, temp_ast: TempModule) -> Self {
        let mut symbols = SymbolTable::new();
        let mut body = Vec::<Function>::new();
        symbols.collect_names(&mut config, &temp_ast);
        if config.is_poisoned() {
            return Self {
                symbols,
                body,
                config,
            };
        }

        let (enums, mut units, funcs, temp_generics, imports, globals, _usings, interfaces) =
            Self::split(temp_ast);

        imports.into_iter().for_each(|_import| {});

        enums.into_iter().for_each(|temp_enum| {
            let (enum_type, _span) = EnumType::new(&config, temp_enum);
            let ident = get_ident(&enum_type.name);
            symbols.types.insert(ident, CompileType::Enum(enum_type));
        });

        let unit_indexs = units
            .iter_mut()
            .map(|temp_unit| {
                temp_unit.name = config.symbol_name(&temp_unit.name);
                let ident = get_ident(&temp_unit.name);
                let temp = UnitType::empty();
                if temp_unit.generics.is_empty() {
                    EnumBool::True(symbols.types.insert(ident, CompileType::Unit(temp)))
                } else {
                    EnumBool::False(symbols.generics.units.insert(ident, temp))
                }
            })
            .collect::<Vec<_>>();

        temp_generics.into_iter().for_each(|temp_generic| {
            if config.is_poisoned() {
                return;
            }
            let (generic_type, _span) =
                GenericRequire::new(&mut config, temp_generic, &mut symbols);
            let ident = arena::get_ident(&generic_type.name);
            symbols.generics.requires.insert(ident, generic_type);
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
                        symbols.types.set(&unit_index, Unit(unit));
                    }
                    EnumBool::False(unit_index) => {
                        let (unit, _span) = UnitType::finalize(&mut config, temp, &mut symbols);
                        symbols.generics.units.set(&unit_index, unit);
                    }
                }
            });

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
                    let index = symbols.generics.functions.insert(ident, symbol);
                    if let Some(scope) = body.clone() {
                        symbols.generics.function_templates.insert(index, scope);
                    }
                    EnumBool::False(index)
                } else {
                    EnumBool::True(symbols.functions.insert(ident, symbol))
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
                let (symbol, _span) = InterfaceSymbol::new(&mut config, symbol, &mut symbols);
                let ident = get_ident(&symbol.name);
                let index = if symbol.has_generics() {
                    let index = symbols.generics.interfaces.insert(ident, symbol);
                    if let Some(scope) = body.clone() {
                        symbols.generics.interface_templates.insert(index, scope);
                    }
                    EnumBool::False(index)
                } else {
                    EnumBool::True(symbols.interfaces.insert(ident, symbol))
                };
                Some((index, body))
            })
            .collect::<Vec<_>>();

        globals.into_iter().for_each(|temp_variable| {
            if config.is_poisoned() {
                return;
            }
            let name = temp_variable.name.clone();
            let var = Variable::new(&mut config, temp_variable, &mut symbols, None);
            symbols.globals.insert(name, var);
        });

        temp_funcs.into_iter().for_each(|(index, temp_body)| {
            if config.is_poisoned() {
                return;
            }
            match index {
                EnumBool::True(func_index) => {
                    let func_body = FuncBody::new(&mut config, func_index, temp_body, &mut symbols);
                    body.push(Function::Func(func_body));
                }
                EnumBool::False(_) => {}
            }
        });

        temp_interfaces.into_iter().for_each(|(index, temp_body)| {
            if config.is_poisoned() {
                return;
            }
            match index {
                EnumBool::True(interface_index) => {
                    let interface_body =
                        Interface::new(&mut config, interface_index, temp_body, &mut symbols);
                    body.push(Function::Interface(interface_body));
                }
                EnumBool::False(_) => {}
            }
        });

        let mut functions = symbols
            .function_instances
            .values()
            .cloned()
            .collect::<Vec<_>>();
        functions.sort_by_key(|body| symbols.functions.get(body.symbol).name.clone());
        body.extend(functions.into_iter().map(Function::Func));
        let mut interfaces = symbols
            .interface_instances
            .values()
            .cloned()
            .collect::<Vec<_>>();
        interfaces.sort_by_key(|body| symbols.interfaces.get(body.symbol).name.clone());
        body.extend(interfaces.into_iter().map(Function::Interface));
        Self {
            symbols,
            body,
            config,
        }
    }

    pub fn export(config: Config, temp_ast: TempModule) -> SymbolTable {
        // Implementation for exporting the AST
        todo!()
    }
}
