#![allow(unused)]
//! Semantic analysis and package symbol ownership. No LLVM dependency.
pub mod ast;
pub mod diagnostic;
pub mod infer;
pub use infer::TypeInfer;
pub mod visibility;
pub use mlc_syntax::{lexer, parser};
