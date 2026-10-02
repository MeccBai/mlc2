use super::*;
use crate::lexer::tokenize;
use crate::parser::out::{TempEnum, TempExpr, TempMatchPattern, TempPath, TempStmt, TempType};

fn parse_ok(source: &str) -> TempModule {
    let lexed = tokenize(source).unwrap();
    let (module, errors) = parse(&lexed.tokens, source.len());
    assert!(errors.is_empty(), "{errors:#?}");
    module.expect("valid source should produce a module")
}

mod declarations;
mod expressions;
mod statements;
