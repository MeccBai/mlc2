pub mod arena;
pub mod config;
pub mod expression;
pub mod function;
pub mod generic;
pub mod statement;
pub mod symbol_name;
pub mod symbols;
pub mod types;
pub mod attribute;

#[cfg(test)]
mod tests;

use arena::{FuncIndex, GenericIndex, TypeArena, TypeIndex};
use config::Config;
use function::{FuncBody, Interface};
use generic::GenericRequire;
use statement::Variable;
use symbols::{EnumBool, SymbolTable};
use types::{EnumType, UnitType};

#[derive(Debug, Clone, PartialEq)]
pub enum GlobalStatement {
    EnumDef(EnumType),
    UnitDef(UnitType),
    FuncDef(FuncBody),
    GenericDef(GenericRequire),
    Import(ImportModule),
    GlobalVar(Variable),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Function {
    Func(FuncBody),
    Interface(Interface),
}

mod module;
pub use module::AbstractSyntaxTree;
pub use symbols::{ExportSymbol, ExportTable, ImportModule};

#[cfg(test)]
pub(crate) use tests::support::AnalyzedAst;
