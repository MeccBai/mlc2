#![allow(unused)]

use crate::ast::config::Config;
use crate::ast::types::CompileType::Generic;
use crate::lexer::tokenize;

use mlc_core::{ast, diagnostic};
use mlc_syntax::{lexer, parser};
pub mod manifest;

fn main() {
    let code = include_str!("..\\example\\main.vl");

    let err_h = diagnostic::error::ErrorHandle::new("test".to_string());
    let warn_h = diagnostic::warning::WarningHandle::new("test".to_string());

    let lexed = tokenize(code).unwrap_or_else(|e| {
        err_h.token_error(e, code);
        std::process::exit(1);
    });

    let (module, errors) = parser::parse(&lexed.tokens, code.len());
    if !errors.is_empty() {
        for error in &errors {
            err_h.parse_error(error, code);
        }
        return;
    }
    let Some(module) = module else {
        return;
    };

    let global = crate::ast::config::GlobalConfig::new();
    let config = global.config(
        vec!["".to_string()],
        "".to_string(),
        "".to_string(),
        err_h,
        warn_h,
    );

    let time_now = std::time::Instant::now();
    let mut package = ast::symbols::PackageSymbolTable::new();
    let mut ast = ast::AbstractSyntaxTree::new(config, module);
    ast.analysis(&mut package);
    let elapsed = time_now.elapsed();

    std::println!("AST construction took: {:.2?}", elapsed);

    //if let Some(module) = module {
    //    for (item, _) in &module {
    //        println!("{}", item.dump());
    //    }
    //}
}
