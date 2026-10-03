pub mod arena;
pub mod attribute;
pub mod builtins;
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

pub use arena::{FuncIndex, GenericIndex, TypeArena, TypeIndex};
pub use config::Config;
use function::{FuncBody, Interface};
use generic::GenericRequire;
use statement::Variable;
pub use symbols::{EnumBool, SymbolTable};
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
