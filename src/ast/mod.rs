pub mod arena;
pub mod expr;
pub mod func;
mod generic;
pub mod stmt;
pub(crate) mod types;

use crate::ast::func::FuncSymbol;
use crate::ast::generic::GenericTable;
use crate::ast::stmt::Variable;
use crate::ast::types::CompileType;
use crate::ast::types::{EnumType, UnitType};
use crate::parser::TempGlobalStmt;
use crate::parser::out::{TempEnum, TempFunc, TempGeneric, TempUnit, TempUsing, TempVar};
use arena::{FuncArena, FuncIndex, GenericArena, GenericIndex, TypeArena, TypeIndex};
use chumsky::prelude::todo;
use func::FuncBody;
use generic::GenericRequire;
use std::collections::HashMap;

pub struct Config {
    system_path: Vec<String>,
    project_path: String,
}
impl TypeIndex {
    pub fn format(&self, arena: &TypeArena) -> String {
        let ty = arena.get(*self);
        match ty {
            CompileType::Base(base) => base.name(),
            CompileType::Ref(ref_type) => ref_type.format(arena),
            CompileType::Unit(unit) => format!("Unit Type: {}", unit.name),
            CompileType::List(list) => format!("List Type: {:?}", list),
            CompileType::Enum(enm) => format!("Enum Type: {:?}", enm),
            _ => {
                panic!("Type Index cannot contain a Generic type. This is a bug in the compiler.");
            }
        }
    }

    pub fn dump(&self) {}
}

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

pub struct AbstractSyntaxTree {
    pub symbols: SymbolTable,
    pub generics: GenericTable,
}

pub struct SymbolTable {
    pub types: TypeArena,
    pub generics: GenericArena,
    pub functions: HashMap<String, FuncSymbol>,
    pub globals: Vec<Variable>,
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
        let mut types = TypeArena::empty();
        let mut generic_table = GenericTable::new();
        let mut functions = FuncArena::empty();

        let (enums, units, funcs, generics_temp, imports, globals, usings) = Self::split(temp_ast);

        imports.into_iter().for_each(|import| todo!());

        generics_temp.into_iter().for_each(|temp_generic| {
            let generic_type = GenericRequire::new(temp_generic);
            let ident = arena::get_ident(&generic_type.name);
            generic_table.generics.insert(ident, generic_type);
        });

        enums.into_iter().for_each(|temp_enum| {
            let name = temp_enum.name;
            let variants = temp_enum.variants;
            let enum_type = EnumType::new(name, variants);
            let ident = arena::get_ident(&enum_type.name);
            types.insert(ident, CompileType::Enum(enum_type));
        });

        usings.into_iter().for_each(|temp_using| todo!());

        units.into_iter().for_each(|temp_unit| {
            let result = UnitType::new(temp_unit, &types, &generic_table.generics);
        });

        globals.into_iter().for_each(|temp_variable| todo!());

        funcs.into_iter().for_each(|temp_func| todo!());

        todo!()
    }
    pub fn export(config: Config, temp_ast: Vec<TempGlobalStmt>) -> SymbolTable {
        // Implementation for exporting the AST
        todo!()
    }
}
