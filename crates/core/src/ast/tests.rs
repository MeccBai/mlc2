//! Source-to-AST integration coverage for this binary crate.
//! Tests never fabricate Temp nodes or pre-populate symbol arenas.
mod cross_file;
mod generic_diagnostics;
mod generic_inference;
mod numeric_initialization;
mod resources;
mod function_pointer;
mod union;
mod usage;

use std::panic::{AssertUnwindSafe, catch_unwind};

use mlc_syntax::manifest::SOURCE_SUFFIX;

use super::{AnalyzedAst, Function, config::Config};
use crate::diagnostic::{
    error::{CompileError, ErrorHandle, ErrorInfo},
    warning::WarningHandle,
};

enum Outcome {
    LexError,
    SyntaxError,
    Ast(AnalyzedAst),
}

fn pipeline(source: &str) -> Outcome {
    let result = catch_unwind(AssertUnwindSafe(|| {
        let Ok(lexed) = crate::lexer::tokenize(source) else {
            return Outcome::LexError;
        };
        let (module, errors) = crate::parser::parse(&lexed.tokens, source.len());
        if !errors.is_empty() {
            return Outcome::SyntaxError;
        }
        let module = module.expect("successful parsing must produce a module");
        Outcome::Ast(AnalyzedAst::new(
            Config::new(
                crate::ast::config::FileId::new(0),
                Vec::new(),
                String::new(),
                format!("integration{}", SOURCE_SUFFIX),
                ErrorHandle::new(format!("integration{}", SOURCE_SUFFIX)),
                WarningHandle::new(format!("integration{}", SOURCE_SUFFIX)),
            ),
            module,
        ))
    }));
    match result {
        Ok(outcome) => outcome,
        Err(payload) => {
            let message = payload
                .downcast_ref::<String>()
                .map(String::as_str)
                .or_else(|| payload.downcast_ref::<&str>().copied())
                .unwrap_or("non-string panic");
            panic!("source-to-AST pipeline panicked: {message}\nsource:\n{source}");
        }
    }
}

fn build(source: &str) -> AnalyzedAst {
    match pipeline(source) {
        Outcome::Ast(ast) => ast,
        Outcome::LexError => panic!("unexpected lexical error:\n{source}"),
        Outcome::SyntaxError => panic!("unexpected syntax error:\n{source}"),
    }
}

fn valid(source: &str) -> AnalyzedAst {
    let ast = build(source);
    assert!(
        !ast.config.is_poisoned(),
        "unexpected semantic error: {:?}\nsource:\n{source}",
        ast.config.error_handle().errors
    );
    assert!(ast.config.error_handle().errors.is_empty());
    ast
}

fn invalid_semantics(source: &str) -> AnalyzedAst {
    let ast = build(source);
    assert!(
        ast.config.is_poisoned(),
        "expected semantic rejection:\n{source}"
    );
    assert_eq!(
        ast.config.error_handle().errors.len(),
        1,
        "fail-fast must submit exactly one diagnostic:\n{source}"
    );
    ast
}

fn invalid_syntax(source: &str) {
    assert!(
        matches!(pipeline(source), Outcome::SyntaxError),
        "expected a parser diagnostic:\n{source}"
    );
}

fn assert_error(source: &str, needle: &str, error: CompileError) {
    let ast = invalid_semantics(source);
    let start = source.find(needle).expect("diagnostic needle must exist");
    assert!(
        ast.config
            .error_handle()
            .errors
            .contains(&ErrorInfo::new(error, (start..start + needle.len()).into())),
        "unexpected diagnostic/span: {:?}",
        ast.config.error_handle().errors
    );
}

fn main_body(ast: &AnalyzedAst) -> &[super::statement::Statement] {
    ast.body
        .iter()
        .find_map(|function| match function {
            Function::Func(body) if ast.symbols.functions.get(body.symbol).name == "main" => {
                Some(body.body.as_slice())
            }
            _ => None,
        })
        .expect("main body must be emitted")
}

macro_rules! valid_case {
    ($name:ident, $source:expr) => {
        #[test]
        fn $name() {
            valid($source);
        }
    };
}
macro_rules! semantic_case {
    ($name:ident, $source:expr) => {
        #[test]
        fn $name() {
            invalid_semantics($source);
        }
    };
}
macro_rules! syntax_case {
    ($name:ident, $source:expr) => {
        #[test]
        fn $name() {
            invalid_syntax($source);
        }
    };
}

mod c_abi;
mod control_flow;
mod declarations;
mod diagnostics;
mod expressions;
mod function_calls;
mod generics;
mod known_gaps;
mod path_symbols;
mod stages;
mod statements;
pub(crate) mod support;
mod symbol_names;
mod unit_applications;
