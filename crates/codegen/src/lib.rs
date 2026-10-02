#![allow(unused)]
//! AST-to-IR generation, independent of the LLVM runtime.
pub mod gens;
pub mod manifest;
pub use gens::IrGenerator;
pub use gens::ast::AstIr;
pub use gens::func::SymbolIr;
#[cfg(test)]
pub use mlc_builder::llvm;
pub use mlc_core::{ast, diagnostic};
pub use mlc_syntax::{lexer, parser};
