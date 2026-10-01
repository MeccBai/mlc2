//! Single-file test adapter. Production ASTs do not own symbol tables.
use crate::ast::symbols::PackageSymbolTable;
use crate::ast::{AbstractSyntaxTree, Config, Function, SymbolTable};
use crate::parser::out::TempModule;

pub struct AnalyzedAst {
    pub config: Config,
    pub body: Vec<Function>,
    pub symbols: SymbolTable,
}

impl AnalyzedAst {
    pub fn new(config: Config, module: TempModule) -> Self {
        let id = config.file_id();
        let mut package = PackageSymbolTable::new();
        let mut ast = AbstractSyntaxTree::new(config, module);
        ast.analysis(&mut package);
        Self {
            config: ast.config,
            body: ast.body,
            symbols: package.take_file(id),
        }
    }
}
