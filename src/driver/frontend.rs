//! Package analysis stays local; only finished IR crosses build worker boundaries.
use mlc_builder::{
    artifacts,
    plan::{BuildPlan, TargetInput},
    project::TargetKind,
};
use mlc_codegen::gens::{
    IrGenerator,
    func::SymbolIr,
    instruction::{IrValue, LlvmType},
};
use mlc_core::{
    ast::{
        AbstractSyntaxTree, Function,
        config::{Config, FileId},
        symbols::PackageSymbolTable,
    },
    diagnostic::{error::ErrorHandle, warning::WarningHandle},
};
mod names;

pub struct Generated {
    pub modules: Vec<(usize, String)>,
    pub supplement: Option<String>,
}

pub fn generate(plan: &BuildPlan, triplet: &str, kind: TargetKind) -> Result<Generated, String> {
    let mut package = PackageSymbolTable::new();
    let mut asts = vec![];
    for id in &plan.order {
        let target = &plan.targets[id.0];
        let module = match &target.input {
            TargetInput::Source { module, .. } => module.clone(),
            TargetInput::Declaration(_) => {
                let artifact = artifacts::load(&target.file)?;
                artifact
                    .templates
                    .map(|bundle| bundle.module)
                    .unwrap_or_else(|| artifact.manifest.symbols.declarations())
            }
        };
        let file = target.file.to_string_lossy().into_owned();
        let config = Config::new(
            FileId::new(id.0),
            vec![],
            target.module_name.clone(),
            file.clone(),
            ErrorHandle::new(file.clone()),
            WarningHandle::new(file),
        );
        package.set_imports(
            FileId::new(id.0),
            target.requires.iter().map(|id| FileId::new(id.0)),
        );
        let mut ast = AbstractSyntaxTree::new(config, module);
        ast.export(&mut package);
        checked(&ast)?;
        asts.push((*id, ast));
    }
    for (id, ast) in &mut asts {
        if matches!(plan.targets[id.0].input, TargetInput::Source { .. }) {
            ast.analysis(&mut package);
            checked(ast)?;
        }
    }
    names::qualify(&mut package, plan);
    let mut modules = vec![];
    let mut initializers = vec![];
    let mut entry = None;
    for (id, ast) in asts {
        if !matches!(plan.targets[id.0].input, TargetInput::Source { .. }) {
            continue;
        }
        let mut generator = IrGenerator::new(triplet.into());
        generator.generate_globals(ast.config.file_id(), &package);
        let file_initializers = generator.global_initializers().to_vec();
        for function in ast.body {
            if id == plan.entry {
                if let Function::Func(body) = &function {
                    let symbol = package.get_function(body.symbol, false);
                    if kind == TargetKind::Bin
                        && (symbol.name == "main" || symbol.name == ast.config.symbol_name("main"))
                    {
                        let abi = symbol.llvm_func(&package).map_err(|e| e.to_string())?;
                        if !symbol.params.is_empty()
                            || abi.is_variadic()
                            || !matches!(abi.result().ty, LlvmType::Void | LlvmType::Int(32))
                        {
                            return Err(
                                "Executable main must have no parameters and return i32 or void"
                                    .into(),
                            );
                        }
                        entry = Some((abi.name().to_owned(), abi.result().ty.clone()));
                    }
                }
            }
            generator.generate(&package, function);
        }
        let mut ir = generator.finish().map_err(|e| format!("{e:?}"))?;
        for init in file_initializers {
            let entry = format!("{init}.entry");
            ir.push_str(&format!(
                "define hidden void {}() {{\nentry:\n  call void {}()\n  ret void\n}}\n",
                IrValue::Global(entry.clone()),
                IrValue::Global(init)
            ));
            initializers.push(entry);
        }
        modules.push((id.0, ir));
    }
    let mut supplement = IrGenerator::new(triplet.into());
    let mut instances = vec![];
    for (_, file) in package.arenas().iter() {
        instances.extend(
            file.function_instances
                .values()
                .cloned()
                .map(Function::Func),
        );
        instances.extend(
            file.interface_instances
                .values()
                .cloned()
                .map(Function::Interface),
        );
    }
    // A single supplementary object owns all newly instantiated bodies.
    let has_instances = !instances.is_empty();
    for function in instances {
        supplement.generate(&package, function);
    }
    let mut ir = supplement.finish().map_err(|e| format!("{e:?}"))?;
    let mut extra = has_instances;
    if kind == TargetKind::Bin {
        let (name, result) = entry.ok_or("Executable entry file must define main")?;
        if name != "main" {
            for init in &initializers {
                ir.push_str(&format!(
                    "declare void {}()\n",
                    IrValue::Global(init.clone())
                ));
            }
            ir.push_str(&format!(
                "declare {result} {}()\ndefine i32 @main() {{\nentry:\n",
                IrValue::Global(name.clone())
            ));
            for init in &initializers {
                ir.push_str(&format!(
                    "  call void {}()\n",
                    IrValue::Global(init.clone())
                ));
            }
            if result == LlvmType::Void {
                ir.push_str(&format!(
                    "  call void {}()\n  ret i32 0\n}}\n",
                    IrValue::Global(name)
                ));
            } else {
                ir.push_str(&format!(
                    "  %result = call i32 {}()\n  ret i32 %result\n}}\n",
                    IrValue::Global(name)
                ));
            }
            extra = true;
        } else if !initializers.is_empty() {
            append_startup(&mut ir, &initializers);
            extra = true;
        }
    } else if !initializers.is_empty() {
        if kind == TargetKind::Static {
            return Err("Static library global startup needs archive entry orchestration; not supported yet".into());
        }
        append_startup(&mut ir, &initializers);
        extra = true;
    }
    Ok(Generated {
        modules,
        supplement: extra.then_some(ir),
    })
}

fn append_startup(ir: &mut String, initializers: &[String]) {
    for name in initializers {
        ir.push_str(&format!(
            "declare void {}()\n",
            IrValue::Global(name.clone())
        ));
    }
    ir.push_str("@llvm.global_ctors = appending global [1 x { i32, ptr, ptr }] [{ i32, ptr, ptr } { i32 65535, ptr @\".mlc.startup\", ptr null }]\ndefine internal void @\".mlc.startup\"() {\nentry:\n");
    for name in initializers {
        ir.push_str(&format!(
            "  call void {}()\n",
            IrValue::Global(name.clone())
        ));
    }
    ir.push_str("  ret void\n}\n");
}

fn checked(ast: &AbstractSyntaxTree) -> Result<(), String> {
    if ast.config.is_poisoned() {
        Err(format!("{:?}", ast.config.error_handle()))
    } else {
        Ok(())
    }
}
