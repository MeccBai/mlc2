//! Linkage naming is a lowering concern; semantic arena keys remain unchanged.
use mlc_builder::plan::BuildPlan;
use mlc_core::ast::{attribute::FuncAttibute, config::FileId, symbols::PackageSymbolTable};

pub(super) fn qualify(package: &mut PackageSymbolTable, plan: &BuildPlan) {
    for target in &plan.targets {
        let file = package
            .file_mut(FileId::new(target.id.0))
            .expect("analyzed file");
        let functions: Vec<_> = file
            .functions
            .entries()
            .map(|(_, index, _)| index)
            .collect();
        for index in functions {
            let symbol = file.functions.get_mut(index);
            if !symbol.attributes.contains(&FuncAttibute::Cabi) {
                symbol.name = qualified(&target.module_name, &symbol.name);
            }
        }
        let interfaces: Vec<_> = file
            .interfaces
            .entries()
            .map(|(_, index, _)| index)
            .collect();
        for index in interfaces {
            let symbol = file.interfaces.get_mut(index);
            if !symbol.attributes.contains(&FuncAttibute::Cabi) {
                symbol.name = qualified(&target.module_name, &symbol.name);
            }
        }
    }
}

fn qualified(module: &str, name: &str) -> String {
    if name.starts_with(&format!("{module}::")) {
        name.into()
    } else {
        format!("{module}::{name}")
    }
}
