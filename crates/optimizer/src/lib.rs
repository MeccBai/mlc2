//! Analyzed AST -> pass pipeline -> analyzed AST. No LLVM or build dependencies.
use ast::{AbstractSyntaxTree, Function, statement::Statement, symbols::PackageSymbolTable};
pub use mlc_core::ast;
mod config;
mod constant;
mod functions;
pub mod passes;
#[cfg(test)]
mod tests;
mod walk;
pub use config::OptimizeConfig;
use passes::{AstPass, ConstantFolding, DeadCodeElimination, UnusedVariableElimination};

pub struct Optimizer {
    pub config: OptimizeConfig,
    extra_passes: Vec<Box<dyn AstPass>>,
}

impl Optimizer {
    pub fn new(config: OptimizeConfig) -> Self {
        Self {
            config,
            extra_passes: vec![],
        }
    }

    pub fn add_pass(&mut self, pass: impl AstPass + 'static) {
        self.extra_passes.push(Box::new(pass));
    }

    pub fn optimize(
        &self,
        ast: AbstractSyntaxTree,
        package: &mut PackageSymbolTable,
    ) -> AbstractSyntaxTree {
        self.optimize_all(vec![ast], package).pop().unwrap()
    }

    /// Optimize the complete analyzed package together so cross-file references survive.
    pub fn optimize_all(
        &self,
        modules: Vec<AbstractSyntaxTree>,
        package: &mut PackageSymbolTable,
    ) -> Vec<AbstractSyntaxTree> {
        self.optimize_all_with_configs(modules, package, |_| self.config)
    }

    pub fn optimize_all_with_configs(
        &self,
        mut modules: Vec<AbstractSyntaxTree>,
        package: &mut PackageSymbolTable,
        config_for: impl Fn(ast::config::FileId) -> OptimizeConfig,
    ) -> Vec<AbstractSyntaxTree> {
        let mut globals = std::collections::HashMap::new();
        // Late cross-file instantiations may not yet be in their owner's AST body.
        for module in &mut modules {
            let config = config_for(module.config.file_id());
            assert!(
                module.is_analyzed() && !module.config.is_poisoned(),
                "Optimizer requires a valid analyzed AST"
            );
            let file = package
                .file(module.config.file_id())
                .expect("analyzed file");
            for body in file.function_instances.values() {
                if !module.body.iter().any(
                    |function| matches!(function, Function::Func(f) if f.symbol == body.symbol),
                ) {
                    module.body.push(Function::Func(body.clone()));
                }
            }
            for body in file.interface_instances.values() {
                if !module.body.iter().any(|function| matches!(function, Function::Interface(f) if f.symbol == body.symbol)) {
                    module.body.push(Function::Interface(body.clone()));
                }
            }
            let mut pipeline: Vec<&dyn AstPass> = vec![];
            if config.constant_folding {
                pipeline.push(&ConstantFolding);
            }
            if config.dead_code_elimination {
                pipeline.push(&DeadCodeElimination);
            }
            if config.unused_variable_elimination {
                pipeline.push(&UnusedVariableElimination);
            }
            pipeline.extend(self.extra_passes.iter().map(|pass| &**pass));
            for pass in pipeline {
                for function in &mut module.body {
                    pass.run(body_mut(function), &mut module.config, package);
                }
            }
            if config.constant_folding {
                let mut declarations = package
                    .file(module.config.file_id())
                    .unwrap()
                    .globals
                    .clone()
                    .into_iter()
                    .collect::<Vec<_>>();
                declarations.sort_by_key(|(_, (order, _))| *order);
                for (name, (order, mut variable)) in declarations {
                    let context = package.resolve_context(module.config.file_id());
                    let mut init = variable
                        .init_val
                        .clone()
                        .const_fold(&mut module.config, &context)
                        .normalize_initializer(variable.var_type, &context);
                    walk::expression(&mut init, &mut |expression| {
                        rebind_global(expression, &globals)
                    });
                    if init != *variable.init_val {
                        let original = std::rc::Rc::as_ptr(&variable) as usize;
                        let previous = variable.clone();
                        std::rc::Rc::make_mut(&mut variable).init_val = Box::new(init);
                        // Keep the old allocation alive until rebinding finishes;
                        // pointer keys must not be reused by later Rc allocations.
                        globals.insert(original, (previous, variable.clone()));
                    }
                    package
                        .file_mut(module.config.file_id())
                        .unwrap()
                        .globals
                        .insert(name, (order, variable));
                }
            }
        }
        // Globals are identified by Rc identity during lowering. Rewriting an
        // initializer must retarget every reference, not only the symbol table.
        if !globals.is_empty() {
            for module in &mut modules {
                for function in &mut module.body {
                    walk::expressions(body_mut(function), &mut |expression| {
                        rebind_global(expression, &globals)
                    });
                }
            }
        }
        functions::eliminate(&mut modules, package, |file| {
            config_for(file).unused_function_elimination
        });
        // The supplement generator reads these maps, not the AST body itself.
        for module in &modules {
            let file = package.file_mut(module.config.file_id()).unwrap();
            file.function_instances.retain(|index, _| {
                module
                    .body
                    .iter()
                    .any(|function| matches!(function, Function::Func(f) if f.symbol == *index))
            });
            file.interface_instances.retain(|index, _| {
                module.body.iter().any(
                    |function| matches!(function, Function::Interface(f) if f.symbol == *index),
                )
            });
            for function in &module.body {
                match function {
                    Function::Func(body) if file.function_instances.contains_key(&body.symbol) => {
                        file.function_instances.insert(body.symbol, body.clone());
                    }
                    Function::Interface(body)
                        if file.interface_instances.contains_key(&body.symbol) =>
                    {
                        file.interface_instances.insert(body.symbol, body.clone());
                    }
                    _ => {}
                }
            }
        }
        modules
    }
}

fn rebind_global(
    expression: &mut ast::expression::Expression,
    replacements: &std::collections::HashMap<
        usize,
        (
            std::rc::Rc<ast::statement::Variable>,
            std::rc::Rc<ast::statement::Variable>,
        ),
    >,
) {
    if let ast::expression::Expression::VarValueE(variable, ..) = expression {
        if let Some((_, replacement)) = replacements.get(&(std::rc::Rc::as_ptr(variable) as usize))
        {
            *variable = replacement.clone();
        }
    }
}

pub(crate) fn body_mut(function: &mut Function) -> &mut Vec<Statement> {
    match function {
        Function::Func(body) => &mut body.body,
        Function::Interface(body) => &mut body.body,
    }
}
