#![allow(unused)]
//! Semantic analysis and package symbol ownership. No LLVM dependency.
pub mod ast;
pub mod diagnostic;
pub mod visibility;
pub use mlc_syntax::{lexer, parser};
