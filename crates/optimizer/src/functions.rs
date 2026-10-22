//! Zero-reference elimination (not a whole-program reachability analysis).
use crate::{
    ast::{
        AbstractSyntaxTree, Function,
        expression::{Expression, UnaryExpr},
        symbols::{EnumBool, PackageSymbolTable},
    },
    walk,
};
use std::collections::HashSet;

pub(crate) fn eliminate(
    modules: &mut [AbstractSyntaxTree],
    package: &mut PackageSymbolTable,
    enabled: impl Fn(crate::ast::config::FileId) -> bool,
) {
    let mut functions = HashSet::new();
    let mut interfaces = HashSet::new();
    let mut reference = |value: &mut Expression| match value {
        Expression::FuncCallE(call, ..) => match call.func {
            EnumBool::False(index) => {
                functions.insert(index);
            }
            EnumBool::True(index) => {
                interfaces.insert(index);
            }
        },
        Expression::UnaryExprE(UnaryExpr::Function { function, .. }, ..) => {
            functions.insert(*function);
        }
        _ => {}
    };
    for module in modules.iter_mut() {
        for function in &mut module.body {
            walk::expressions(crate::body_mut(function), &mut reference);
        }
    }
    for (_, file) in package.arenas().iter() {
        // Instances imported from declaration-only units may have no source AST.
        for body in file.function_instances.values() {
            walk::expressions(&mut body.body.clone(), &mut reference);
        }
        for body in file.interface_instances.values() {
            walk::expressions(&mut body.body.clone(), &mut reference);
        }
        for (_, variable) in file.globals.values() {
            walk::expression(&mut (*variable.init_val).clone(), &mut reference);
        }
    }
    for module in modules.iter_mut() {
        if !enabled(module.config.file_id()) {
            continue;
        }
        module.body.retain(|function| match function {
            Function::Func(body) => {
                let symbol = package.get_function(body.symbol, false);
                functions.contains(&body.symbol)
                    || symbol.exported
                    || symbol
                        .attributes
                        .contains(&crate::ast::attribute::FuncAttibute::Cabi)
                    || symbol.name == "main"
                    || symbol.name.ends_with("::main")
                    || symbol.name.ends_with("::deconstruct")
            }
            Function::Interface(body) => {
                let symbol = package.get_interface(body.symbol, false);
                interfaces.contains(&body.symbol) || symbol.exported
            }
        });
    }
}
