#![allow(unused)]

use crate::ast::config::Config;
use crate::ast::types::CompileType::Generic;
use crate::lexer::tokenize;

mod ast;
mod build;
mod error;
mod lexer;
mod llvm;
mod parser;

fn main() {
    let code = include_str!("..\\example\\main.vl");

    let err_h = error::ErrorHandle::new("test".to_string());

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

    let config = Config::new(vec!["".to_string()], "".to_string(), "".to_string(), err_h);

    let time_now = std::time::Instant::now();
    let ast = ast::AbstractSyntaxTree::new(config, module);
    let elapsed = time_now.elapsed();
    
    std::println!("AST construction took: {:.2?}", elapsed);

    //if let Some(module) = module {
    //    for (item, _) in &module {
    //        println!("{}", item.dump());
    //    }
    //}
}
