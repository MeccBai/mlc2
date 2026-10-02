//! File-owned storage and ordered runtime initialization. The build entry calls
//! each listed initializer once, in its chosen dependency order, before user code.
use super::{IrGenerator, error::fail, function::FunctionGenerator, instruction::IrValue};
use crate::ast::{config::FileId, symbol_name::SymbolName, symbols::PackageSymbolTable};
use std::{collections::HashSet, fmt::Write};

impl IrGenerator {
    pub fn global_initializers(&self) -> &[String] {
        &self.initializers
    }

    pub fn generate_globals(&mut self, file: FileId, package: &PackageSymbolTable) {
        let name = SymbolName::global_initializer(file);
        self.collect(name.clone(), |generator| {
            let symbols = package
                .file(file)
                .unwrap_or_else(|| fail("Missing global variable arena"));
            if symbols.globals.is_empty()
                || !generator.remember_symbol(format!("definition::{name}"))
            {
                return;
            }
            let mut visited = HashSet::new();
            for variable in symbols.ordered_globals() {
                generator.ensure_type(variable.var_type, package, &mut visited);
                writeln!(
                    generator.header,
                    "{} = internal global {} zeroinitializer, align {}",
                    IrValue::Global(variable.name.clone()),
                    Self::type_lowering(variable.var_type, package),
                    variable.var_type.align(package.arenas())
                )
                .expect("String write");
            }
            let generated = FunctionGenerator::globals(file, package);
            generator.add_resources(&generated.resources, &generated.intrinsics);
            for call in &generated.calls {
                if let Some(ty) = call.result().source {
                    generator.ensure_type(ty, package, &mut visited);
                }
                for mapping in call.receiver().into_iter().chain(call.params()) {
                    generator.ensure_type(mapping.source, package, &mut visited);
                }
                generator.add_declaration(call);
            }
            generator.bodys.push_str(&generated.ir);
            generator.initializers.push(name);
        });
    }
}

#[cfg(test)]
mod tests;
