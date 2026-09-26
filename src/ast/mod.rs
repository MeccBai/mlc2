pub mod arena;
pub mod config;
pub mod expr;
pub mod func;
mod generic;
pub mod stmt;
pub(crate) mod types;

use crate::ast::arena::{InterfaceArena, InterfaceIndex, get_ident};
use crate::ast::func::FuncSymbol;
use crate::ast::func::{Interface, InterfaceSymbol};
use crate::ast::generic::GenericTable;
use crate::ast::stmt::Variable;
use crate::ast::types::CompileType::{Base, Enum, List, Ref, Unit};
use crate::ast::types::{BaseType, CompileType};
use crate::ast::types::{EnumType, UnitType};
use crate::error::ice::ice;
use crate::parser::TempGlobalStmt;
use crate::parser::out::{TempEnum, TempFunc, TempGeneric, TempUnit, TempUsing, TempVar};
use arena::{FuncArena, FuncIndex, GenericIndex, TypeArena, TypeIndex};
use config::Config;
use func::FuncBody;
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
}

pub struct SymbolTable {
    pub types: TypeArena,
    pub functions: FuncArena,
    pub interfaces: InterfaceArena,
    pub generics: GenericTable,
    pub globals: Vec<Variable>,
}

impl SymbolTable {
    pub fn new() -> Self {
        let mut temp = Self {
            types: TypeArena::new(),
            functions: FuncArena::empty(),
            interfaces: InterfaceArena::empty(),
            generics: GenericTable::new(),
            globals: Vec::new(),
        };
        let base_types = BaseType::base_types();
        base_types
            .into_iter()
            .for_each(|base_type| match base_type {
                Base(base) => {
                    let name = base.name();
                    let ident = arena::get_ident(&name);
                    let ret_type = temp.types.get_by_ident(ident).unwrap();
                    let func = FuncSymbol {
                        name: base.name(),
                        params: Vec::new(),
                        ret_type: Some(ret_type),
                        generics: Vec::new(),
                        attributes: Vec::new(),
                        exported: false,
                    };
                    let ident = get_ident(&name);
                    temp.functions.insert(ident, func);
                }
                _ => ice("Expected a Base type."),
            });
        temp
    }
}

enum Bool<T, F> {
    True(T),
    False(F),
}

impl AbstractSyntaxTree {
    fn split(
        temp_ast: Vec<TempGlobalStmt>,
    ) -> (
        Vec<TempEnum>,
        Vec<TempUnit>,
        Vec<TempFunc>,
        Vec<TempGeneric>,
        Vec<ImportModule>,
        Vec<TempVar>,
        Vec<TempUsing>,
    ) {
        let mut enums = Vec::<TempEnum>::new();
        let mut units = Vec::<TempUnit>::new();
        let mut funcs = Vec::<TempFunc>::new();
        let mut generics = Vec::<TempGeneric>::new();
        let mut imports = Vec::<ImportModule>::new();
        let mut globals = Vec::<TempVar>::new();
        let mut usings = Vec::<TempUsing>::new();

        temp_ast.into_iter().for_each(|stmt| match stmt {
            TempGlobalStmt::Unit(temp_unit) => units.push(temp_unit),
            TempGlobalStmt::Func(temp_func) => funcs.push(temp_func),
            TempGlobalStmt::Using(temp_using) => usings.push(temp_using),
            TempGlobalStmt::Generic(temp_generic) => generics.push(temp_generic),
            TempGlobalStmt::Enum(temp_enum) => enums.push(temp_enum),
            TempGlobalStmt::Import(temp_import) => imports.push(temp_import),
            TempGlobalStmt::Variable(temp_variable) => globals.push(temp_variable),
        });

        (enums, units, funcs, generics, imports, globals, usings)
    }

    pub fn new(config: Config, temp_ast: Vec<TempGlobalStmt>) -> Self {
        let mut symbols = SymbolTable::new();
        let mut generics = GenericTable::new();
        let mut body = Vec::<Function>::new();

        let (enums, mut units, funcs, temp_generics, imports, globals, usings) =
            Self::split(temp_ast);

        imports.into_iter().for_each(|import| todo!());

        temp_generics.into_iter().for_each(|temp_generic| {
            let generic_type = GenericRequire::new(&config, temp_generic);
            let ident = arena::get_ident(&generic_type.name);
            generics.requires.insert(ident, generic_type);
        });

        enums.into_iter().for_each(|temp_enum| {
            let name = temp_enum.name;
            let variants = temp_enum.variants;
            let enum_type = EnumType::new(&config, name, variants);
            let ident = get_ident(&enum_type.name);
            symbols.types.insert(ident, CompileType::Enum(enum_type));
        });

        usings.into_iter().for_each(|temp_using| todo!());

        let unit_indexs = units
            .iter_mut()
            .map(|temp_unit| {
                temp_unit.name = config.symbol_name(&temp_unit.name);
                let ident = get_ident(&temp_unit.name);
                let temp = UnitType::empty();
                if temp_unit.generics.is_empty() {
                    Bool::True(symbols.types.insert(ident, CompileType::Unit(temp)))
                } else {
                    Bool::False(symbols.generics.units.insert(ident, temp))
                }
            })
            .collect::<Vec<_>>();

        unit_indexs
            .into_iter()
            .zip(units.into_iter())
            .for_each(|(index, temp)| match index {
                Bool::True(unit_index) => {
                    let unit = UnitType::new(&config, temp, &mut symbols).unwrap();
                    symbols.types.set(&unit_index, Unit(unit));
                }
                Bool::False(unit_index) => {
                    let unit = UnitType::new(&config, temp, &mut symbols).unwrap();
                    symbols.generics.units.set(&unit_index, unit);
                }
            });

        symbols.globals = globals
            .into_iter()
            .map(|temp_variable| Variable::new(temp_variable, &mut symbols))
            .collect::<Vec<Variable>>();

        let temp_funcs: Vec<_> = funcs
            .into_iter()
            .map(|temp_func| {
                let (symbol, body) = temp_func.split();
                let is_interface = symbol.owner.is_some();
                if is_interface {
                    let symbol = InterfaceSymbol::new(&config, symbol, &symbols);
                    let ident = get_ident(&symbol.name);
                    let index = symbols.interfaces.insert(ident, symbol);
                    (Bool::True(index), body)
                } else {
                    let symbol = FuncSymbol::new(&config, symbol, &symbols);
                    let ident = get_ident(&symbol.name);
                    let index = symbols.functions.insert(ident, symbol);
                    (Bool::False(index), body)
                }
            })
            .collect();

        temp_funcs
            .into_iter()
            .for_each(|(index, temp_body)| match index {
                Bool::False(func_index) => {
                    let func_body =
                        FuncBody::new(func_index, temp_body, &mut generics, &mut symbols);
                    body.push(Function::Func(func_body));
                }
                Bool::True(interface_index) => {
                    let interface =
                        Interface::new(interface_index, temp_body, &mut generics, &mut symbols);
                    body.push(Function::Interface(interface));
                }
            });

        Self { symbols, body }
    }

    pub fn export(config: Config, temp_ast: Vec<TempGlobalStmt>) -> SymbolTable {
        // Implementation for exporting the AST
        todo!()
    }
}
