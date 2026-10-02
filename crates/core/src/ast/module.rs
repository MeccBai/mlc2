use super::{ExportTable, Function, ImportModule};
use crate::ast::{
    arena::{self, FuncIndex},
    config::Config,
    symbols::{EnumBool, PackageSymbolTable, SymbolTable, export},
};
use crate::diagnostic::error::CompileError;
use crate::parser::out::{
    TempEnum, TempFunc, TempGeneric, TempInterface, TempUnit, TempUsing, TempVar,
};
use crate::parser::{self, TempGlobalStmt, TempModule};
use std::collections::HashSet;
mod analysis;
mod declarations;
mod deferred;
use deferred::DeferredDeclarations;

type PendingFunction = (
    EnumBool<FuncIndex, FuncIndex>,
    Option<parser::out::TempScope>,
);
type PendingInterface = (
    EnumBool<arena::InterfaceIndex, arena::InterfaceIndex>,
    Option<parser::out::TempScope>,
);

#[derive(Debug, Clone, PartialEq)]
pub(super) struct PendingBodies {
    globals: Vec<TempVar>,
    functions: Vec<PendingFunction>,
    interfaces: Vec<PendingInterface>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AbstractSyntaxTree {
    pub body: Vec<Function>,
    pub config: Config,
    declarations: Option<DeferredDeclarations>,
    imports: Vec<ImportModule>,
    pending: Option<PendingBodies>,
    prepared: bool,
    analyzed: bool,
    exported_types: HashSet<String>,
}

impl AbstractSyntaxTree {
    /// Construction only groups and retains Temp declarations.
    pub fn new(config: Config, temp_ast: TempModule) -> Self {
        let imports = temp_ast
            .iter()
            .filter_map(|(stmt, _)| match stmt {
                TempGlobalStmt::Import(import) => Some(import.clone()),
                _ => None,
            })
            .collect();
        let exported_types = temp_ast
            .iter()
            .filter_map(|(stmt, _)| match stmt {
                TempGlobalStmt::Enum(v) if v.visibility == parser::out::TempVisibility::Export => {
                    Some(config.symbol_name(&v.name))
                }
                TempGlobalStmt::Unit(v) if v.visibility == parser::out::TempVisibility::Export => {
                    Some(config.symbol_name(&v.name))
                }
                _ => None,
            })
            .collect();
        Self {
            body: Vec::new(),
            config,
            declarations: Some(DeferredDeclarations::new(temp_ast)),
            imports,
            pending: None,
            prepared: false,
            analyzed: false,
            exported_types,
        }
    }

    pub fn requires(&self) -> &[ImportModule] {
        &self.imports
    }

    fn prepare(&mut self, package: &mut PackageSymbolTable) {
        if self.prepared || self.config.is_poisoned() {
            return;
        }
        self.prepared = true;
        let id = self.config.file_id();
        if package.register(id).is_err() {
            self.config.submit_error(
                CompileError::IllegalUse(
                    crate::diagnostic::error::IllegalUseError::DuplicateFileId,
                ),
                (0..0).into(),
            );
            return;
        }
        let declarations = self.declarations.take().expect("unprepared declarations");
        let symbols = package.file_mut(id).expect("registered file");
        symbols.collect_names(&mut self.config, declarations.source_order().into_iter());
        if self.config.is_poisoned() {
            return;
        }
        if let Err((name, span)) = package.claim_names(id) {
            self.config.submit_error(
                CompileError::IllegalUse(
                    crate::diagnostic::error::IllegalUseError::DuplicateSymbol { name },
                ),
                span,
            );
            return;
        }
        let symbols = package.file_mut(id).expect("registered file");
        self.pending = Some(declarations::parse(
            &mut self.config,
            symbols,
            declarations.into_module(),
        ));
        package.update_config(&self.config);
    }

    pub fn export(&mut self, package: &mut PackageSymbolTable) -> ExportTable {
        self.prepare(package);
        if self.config.is_poisoned() {
            return ExportTable::empty(self.config.file_id());
        }
        let table = export::collect(&self.config, package, &self.exported_types);
        if let Err(error) = package.concat(table.clone()) {
            let error = match error {
                crate::ast::symbols::ConcatError::DuplicateSymbol(name) => {
                    crate::diagnostic::error::IllegalUseError::DuplicateSymbol { name }
                }
                _ => crate::diagnostic::error::IllegalUseError::InvalidExportTable,
            };
            self.config
                .submit_error(CompileError::IllegalUse(error), (0..0).into());
            return ExportTable::empty(self.config.file_id());
        }
        table
    }

    pub fn analysis(&mut self, package: &mut PackageSymbolTable) {
        self.prepare(package);
        if self.analyzed || self.config.is_poisoned() {
            return;
        }
        self.analyzed = true;
        let Some(pending) = self.pending.take() else {
            return;
        };
        let symbols = package
            .file_mut(self.config.file_id())
            .expect("prepared file");
        self.body = analysis::parse(&mut self.config, symbols, pending);
        package.update_config(&self.config);
    }

    pub fn is_analyzed(&self) -> bool {
        self.analyzed
    }
}
