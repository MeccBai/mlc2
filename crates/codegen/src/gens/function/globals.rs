use super::*;
use crate::ast::{config::FileId, symbol_name::SymbolName};

impl<'a> FunctionGenerator<'a> {
    /// Global initialization is not a language-level assignment: val/const
    /// storage may be initialized here, but cannot be reassigned by user code.
    pub(in crate::gens) fn globals(
        file: FileId,
        package: &'a PackageSymbolTable,
    ) -> GeneratedFunction {
        let name = SymbolName::global_initializer(file);
        let mut generator = Self::new(LlvmFunc::initializer(name), package);
        let symbols = package
            .file(file)
            .unwrap_or_else(|| fail("Missing global initialization arena"));
        for variable in symbols.ordered_globals() {
            let target = generator.expression(&Expression::VarValueE(variable.clone()));
            let signed = generator.source_signed(&variable.init_val);
            let value = generator.expression_expected(&variable.init_val, Some(variable.var_type));
            generator.store(&target, value, signed);
        }
        generator.exit(
            ExitKind::Return,
            StackPosition::default(),
            "function.return".into(),
        );
        let mut generated = generator.finish();
        generated.ir = generated.ir.replacen("define ", "define internal ", 1);
        generated
    }
}
