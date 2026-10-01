use crate::ast::symbol_name::SymbolName;
use crate::error::ErrorHandle;
use crate::parser::out::Span;

pub struct Config {
    system_path: Vec<String>,
    project_path: String,
    current_file: String,
    err_h: ErrorHandle,
    poisoned: bool,
}

impl Config {
    pub fn new(
        system_path: Vec<String>,
        project_path: String,
        current_file: String,
        err_h: ErrorHandle,
    ) -> Self {
        let poisoned = !err_h.errors.is_empty();
        Self {
            system_path,
            project_path,
            current_file,
            err_h,
            poisoned,
        }
    }

    pub fn symbol_prefix(&self) -> String {
        SymbolName::prefix(&self.system_path, &self.project_path)
    }

    pub fn symbol_name(&self, name: &str) -> String {
        SymbolName::qualified(&self.system_path, &self.project_path, name)
    }

    pub fn current_file(&self) -> &String {
        &self.current_file
    }

    pub fn submit_error(&mut self, error: crate::error::CompileError, span: Span) {
        if self.poisoned {
            return;
        }
        self.err_h.submit_error(error, span);
        self.poisoned = true;
    }

    /// Semantic parsing stops after the first diagnostic; recovery belongs to the token parser.
    pub fn is_poisoned(&self) -> bool {
        self.poisoned
    }

    pub fn error_handle(&self) -> &ErrorHandle {
        &self.err_h
    }

    pub fn into_error_handle(self) -> ErrorHandle {
        self.err_h
    }
}
