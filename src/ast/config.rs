use crate::ast::symbol_name::SymbolName;
use crate::diagnostic::error::ErrorHandle;
use crate::diagnostic::warning::WarningHandle;
use crate::parser::out::Span;

/// Assigned by the entry unit; also identifies the file's future arena.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileId(usize);

impl FileId {
    pub const fn new(value: usize) -> Self {
        Self(value)
    }
}

/// Share one allocator across the entry unit and all recursively loaded files.
#[derive(Debug, Clone, Default)]
pub struct GlobalConfig {
    next_file_id: std::rc::Rc<std::cell::Cell<usize>>,
}

impl GlobalConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn next_file_id(&self) -> FileId {
        let next = self.next_file_id.get();
        assert!(next < usize::MAX, "file identity space exhausted");
        self.next_file_id.set(next + 1);
        FileId::new(next)
    }

    pub fn config(
        &self,
        system_path: Vec<String>,
        project_path: String,
        current_file: String,
        err_h: ErrorHandle,
        warning_h: WarningHandle,
    ) -> Config {
        Config::new(
            self.next_file_id(),
            system_path,
            project_path,
            current_file,
            err_h,
            warning_h,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    file_id: FileId,
    system_path: Vec<String>,
    project_path: String,
    current_file: String,
    err_h: ErrorHandle,
    warning_h: WarningHandle,
    poisoned: bool,
}

impl Config {
    pub fn new(
        file_id: FileId,
        system_path: Vec<String>,
        project_path: String,
        current_file: String,
        err_h: ErrorHandle,
        warning_h: WarningHandle,
    ) -> Self {
        let poisoned = !err_h.errors.is_empty();
        Self {
            file_id,
            system_path,
            project_path,
            current_file,
            err_h,
            warning_h,
            poisoned,
        }
    }

    pub fn symbol_prefix(&self) -> String {
        SymbolName::prefix(&self.system_path, &self.project_path)
    }

    pub fn file_id(&self) -> FileId {
        self.file_id
    }

    pub fn symbol_name(&self, name: &str) -> String {
        SymbolName::qualified(&self.system_path, &self.project_path, name)
    }

    pub fn current_file(&self) -> &String {
        &self.current_file
    }

    pub fn submit_error(&mut self, error: crate::diagnostic::error::CompileError, span: Span) {
        if self.poisoned {
            return;
        }
        self.err_h.submit_error(error, span);
        self.poisoned = true;
    }

    pub fn submit_warning(&mut self, warning: crate::diagnostic::warning::Warning, span: Span) {
        self.warning_h.submit_warning(warning, span);
    }

    /// Semantic parsing stops after the first diagnostic; recovery belongs to the token parser.
    pub fn is_poisoned(&self) -> bool {
        self.poisoned
    }

    pub fn error_handle(&self) -> &ErrorHandle {
        &self.err_h
    }

    pub fn warning_handle(&self) -> &WarningHandle {
        &self.warning_h
    }

    pub fn into_error_handle(self) -> ErrorHandle {
        self.err_h
    }
}

#[cfg(test)]
mod tests;
