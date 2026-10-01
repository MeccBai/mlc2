#[derive(Debug, Clone, PartialEq)]
pub struct ImportModule {
    path: Vec<String>,
    export: bool,
}

impl ImportModule {
    pub fn fetch(
        &self,
        _global: &crate::ast::config::GlobalConfig,
    ) -> crate::ast::AbstractSyntaxTree {
        todo!("load imported source and create its Config using the shared GlobalConfig")
    }
    pub fn new(path: Vec<String>, export: bool) -> Self {
        Self { path, export }
    }

    pub fn path(&self) -> &[String] {
        &self.path
    }

    pub fn exported(&self) -> bool {
        self.export
    }
}
