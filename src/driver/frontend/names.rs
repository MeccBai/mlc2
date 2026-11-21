//! Linkage naming is a lowering concern; semantic arena keys remain unchanged.
use mlc_builder::plan::BuildPlan;
use mlc_core::ast::{attribute::FuncAttibute, config::FileId, symbols::PackageSymbolTable};

pub(super) fn qualify(
    package: &mut PackageSymbolTable,
    plan: &BuildPlan,
    embedded: bool,
) -> Result<(), String> {
    let mut handlers = std::collections::HashSet::new();
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
            if embedded {
                let mut interrupts =
                    symbol
                        .attributes
                        .iter()
                        .filter_map(|attribute| match attribute {
                            FuncAttibute::Interrupt(name) => Some(name.clone()),
                            _ => None,
                        });
                if let Some(name) = interrupts.next() {
                    if interrupts.next().is_some()
                        || !symbol.params.is_empty()
                        || symbol.ret_type.is_some()
                    {
                        return Err(format!(
                            "Interrupt handler {} requires one interrupt attribute and a void signature with no parameters",
                            symbol.name
                        ));
                    }
                    if !handlers.insert(name.clone()) {
                        return Err(format!("Duplicate interrupt handler symbol: {name}"));
                    }
                    symbol.name = name;
                    symbol.attributes.insert(FuncAttibute::Cabi);
                }
            }
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
    Ok(())
}

fn qualified(module: &str, name: &str) -> String {
    if name.starts_with(&format!("{module}::")) {
        name.into()
    } else {
        format!("{module}::{name}")
    }
}
