use super::*;
use crate::ast::symbols::Resolution;
use crate::ast::{
    function::{FuncBody, Interface},
    statement::Variable,
};

pub(super) fn parse(
    mut config: &mut Config,
    mut symbols: &mut dyn Resolution,
    pending: PendingBodies,
) -> Vec<Function> {
    let PendingBodies {
        globals,
        functions: temp_funcs,
        interfaces: temp_interfaces,
    } = pending;
    let mut body = Vec::new();
    globals
        .into_iter()
        .enumerate()
        .for_each(|(order, mut temp_variable)| {
            if config.is_poisoned() {
                return;
            }
            let name = temp_variable.name.clone();
            temp_variable.name = config.symbol_name(&name);
            let var = Variable::new(&mut config, temp_variable, &mut symbols, None);
            symbols.local_mut().globals.insert(name, (order, var));
        });

    temp_funcs.into_iter().for_each(|(index, temp_body)| {
        if config.is_poisoned() {
            return;
        }
        match index {
            EnumBool::True(func_index) => {
                let func_body = FuncBody::new(&mut config, func_index, temp_body, &mut symbols);
                body.push(Function::Func(func_body));
            }
            EnumBool::False(_) => {}
        }
    });

    temp_interfaces.into_iter().for_each(|(index, temp_body)| {
        if config.is_poisoned() {
            return;
        }
        match index {
            EnumBool::True(interface_index) => {
                let interface_body =
                    Interface::new(&mut config, interface_index, temp_body, &mut symbols);
                body.push(Function::Interface(interface_body));
            }
            EnumBool::False(_) => {}
        }
    });

    let mut functions = symbols
        .local()
        .function_instances
        .values()
        .cloned()
        .collect::<Vec<_>>();
    functions.sort_by_key(|body| symbols.get_function_regular(body.symbol).name.clone());
    body.extend(functions.into_iter().map(Function::Func));
    let mut interfaces = symbols
        .local()
        .interface_instances
        .values()
        .cloned()
        .collect::<Vec<_>>();
    interfaces.sort_by_key(|body| symbols.get_interface_regular(body.symbol).name.clone());
    body.extend(interfaces.into_iter().map(Function::Interface));

    body
}
