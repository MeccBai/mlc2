use super::IrGenerator;
use crate::ast::{AbstractSyntaxTree, symbols::PackageSymbolTable};

/// AST generation lives in codegen, keeping core independent of its consumers.
pub trait AstIr {
    fn generate(
        &self,
        package: &PackageSymbolTable,
        generator: &mut IrGenerator,
    ) -> (String, String);
}
impl AstIr for AbstractSyntaxTree {
    fn generate(
        &self,
        _package: &PackageSymbolTable,
        _generator: &mut IrGenerator,
    ) -> (String, String) {
        if !self.is_analyzed() || self.config.is_poisoned() {
            return (String::new(), String::new());
        }
        todo!("entry orchestration remains deferred")
    }
}
