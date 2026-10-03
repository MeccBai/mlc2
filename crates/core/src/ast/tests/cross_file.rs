use crate::ast::types::TypeLookup;
use crate::ast::{
    AbstractSyntaxTree,
    config::{Config, FileId},
    expression::Expression,
    statement::Statement,
    symbols::{ExportSymbol, PackageSymbolTable, Resolution, ResolveContext, ResolveScope},
    types::{CompileType, resolve_type},
};
use crate::diagnostic::{
    error::{CompileError, ErrorHandle, ErrorInfo, IllegalUseError},
    warning::WarningHandle,
};
use crate::parser::out::{TempPath, TempType};

fn ast(id: usize, module: &str, source: &str) -> AbstractSyntaxTree {
    let tokens = crate::lexer::tokenize(source).unwrap();
    let (temp, errors) = crate::parser::parse(&tokens.tokens, source.len());
    assert!(errors.is_empty(), "{errors:?}\n{source}");
    let file = format!("{module}.m2");
    AbstractSyntaxTree::new(
        Config::new(
            FileId::new(id),
            vec![],
            module.into(),
            file.clone(),
            ErrorHandle::new(file.clone()),
            WarningHandle::new(file),
        ),
        temp.unwrap(),
    )
}
fn publish(package: &mut PackageSymbolTable, ast: &mut AbstractSyntaxTree) {
    ast.export(package);
    assert!(!ast.config.is_poisoned(), "{:?}", ast.config.error_handle());
}
fn analyze(package: &mut PackageSymbolTable, ast: &mut AbstractSyntaxTree) {
    ast.analysis(package);
    assert!(!ast.config.is_poisoned(), "{:?}", ast.config.error_handle());
}

mod access;
mod instantiation;
mod visibility;
