use mlc_core::ast::{AbstractSyntaxTree, ImportModule, config::GlobalConfig};

/// Import loading belongs to the build plan, not the syntax declaration.
pub trait ImportFetch {
    fn fetch(&self, global: &GlobalConfig) -> AbstractSyntaxTree;
}
impl ImportFetch for ImportModule {
    fn fetch(&self, _global: &GlobalConfig) -> AbstractSyntaxTree {
        todo!("load imported source and create its Config using the shared GlobalConfig")
    }
}
