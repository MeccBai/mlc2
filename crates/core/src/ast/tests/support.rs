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
    pub(crate) fn with_iterator_fixture(mut module: TempModule, source: &str) -> TempModule {
        if source.contains("test_range(") {
            let fixture = include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../tests/fixtures/iterator.m2"
            ));
            let lexed = crate::lexer::tokenize(fixture).unwrap();
            module.extend(
                crate::parser::parse(&lexed.tokens, fixture.len())
                    .0
                    .unwrap(),
            );
        }
        module
    }

    pub fn new(config: Config, module: TempModule) -> Self {
        let id = config.file_id();
        let mut package = PackageSymbolTable::new();
        let mut imports = Vec::new();
        for (item, _) in &module {
            let crate::parser::TempGlobalStmt::Import(import) = item else {
                continue;
            };
            if let Some(builtin) = crate::ast::builtins::BuiltinModule::for_import(import.path()) {
                let builtin_id = super::super::config::FileId::new(id.index() + imports.len() + 1);
                let mut builtin_config = Config::new(
                    builtin_id,
                    vec![],
                    builtin.name().into(),
                    builtin.name().into(),
                    crate::diagnostic::error::ErrorHandle::new(builtin.name().into()),
                    crate::diagnostic::warning::WarningHandle::new(builtin.name().into()),
                );
                builtin_config.builtin_module = Some(builtin);
                AbstractSyntaxTree::new(builtin_config, builtin.declarations())
                    .export(&mut package);
                imports.push(builtin_id);
            }
        }
        package.set_imports(id, imports);
        let mut ast = AbstractSyntaxTree::new(config, module);
        ast.analysis(&mut package);
        Self {
            config: ast.config,
            body: ast.body,
            symbols: package.take_file(id),
        }
    }
}
