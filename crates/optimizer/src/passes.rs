use crate::{
    ast::{
        config::Config,
        expression::Expression,
        statement::{AnonymousBlock, Statement},
        symbols::PackageSymbolTable,
    },
    constant, walk,
};

/// Each pass mutates an analyzed body without changing symbol identities or types.
pub trait AstPass {
    fn name(&self) -> &'static str;
    fn run(&self, body: &mut [Statement], config: &mut Config, symbols: &mut PackageSymbolTable);
}

pub struct ConstantFolding;
pub struct DeadCodeElimination;
pub struct UnusedVariableElimination;

impl AstPass for ConstantFolding {
    fn name(&self) -> &'static str {
        "constant-folding"
    }
    fn run(&self, body: &mut [Statement], config: &mut Config, symbols: &mut PackageSymbolTable) {
        let context = symbols.resolve_context(config.file_id());
        walk::expressions(body, &mut |value| {
            *value = value.clone().const_fold(config, &context);
        });
        let mut replacements = walk::Replacements::new();
        walk::statements(body, &mut |statement| {
            if let Statement::VariableDecl(variable) = statement {
                let init = (*variable.init_val)
                    .clone()
                    .normalize_initializer(variable.var_type, &context);
                if init != *variable.init_val {
                    let previous = variable.clone();
                    let identity = std::rc::Rc::as_ptr(&previous) as usize;
                    std::rc::Rc::make_mut(variable).init_val = Box::new(init);
                    replacements.insert(identity, (previous, variable.clone()));
                }
            }
        });
        walk::expressions(body, &mut |value| walk::rebind(value, &replacements));
    }
}

impl AstPass for DeadCodeElimination {
    fn name(&self) -> &'static str {
        "dead-code-elimination"
    }
    fn run(&self, body: &mut [Statement], _: &mut Config, symbols: &mut PackageSymbolTable) {
        walk::statements(body, &mut |statement| {
            let replacement = match statement {
                Statement::IfBlock(block) => constant::value(&block.condition, symbols)
                    .and_then(constant::Scalar::boolean)
                    .map(|condition| {
                        if condition {
                            block.then_branch.clone()
                        } else {
                            block.else_branch.clone().unwrap_or_default()
                        }
                    }),
                Statement::WhileBlock(block)
                    if constant::value(&block.condition, symbols)
                        .and_then(constant::Scalar::boolean)
                        == Some(false) =>
                {
                    Some(vec![])
                }
                Statement::ForBlock(block) if constant::empty_for(block, symbols) => Some(vec![]),
                Statement::MatchBlock(block) => match constant::select_match(block, symbols) {
                    constant::MatchSelection::Scope(body) => Some(body.to_vec()),
                    constant::MatchSelection::Skip => Some(vec![]),
                    constant::MatchSelection::Dynamic => None,
                },
                _ => None,
            };
            if let Some(statements) = replacement {
                *statement = Statement::AnonymousBlock(AnonymousBlock { statements });
            }
        });
    }
}

impl AstPass for UnusedVariableElimination {
    fn name(&self) -> &'static str {
        "unused-variable-elimination"
    }
    fn run(&self, body: &mut [Statement], _: &mut Config, symbols: &mut PackageSymbolTable) {
        // Dropping an unused declaration releases its initializer's references.
        // Repeat so bindings retained only by that initializer can also disappear.
        loop {
            let mut changed = false;
            walk::statements(body, &mut |statement| {
                let Statement::VariableDecl(variable) = statement else {
                    return;
                };
                // Remaining expression references hold an Rc. Owners and aggregates
                // retain storage because their cleanup or address may be observable.
                if std::rc::Rc::strong_count(variable) != 1
                    || variable.var_type.has_res(symbols)
                    || matches!(&*variable.init_val, Expression::InitListE(_, ..))
                {
                    return;
                }
                *statement = if constant::value(&variable.init_val, symbols).is_some() {
                    Statement::AnonymousBlock(AnonymousBlock { statements: vec![] })
                } else {
                    Statement::Expression((*variable.init_val).clone())
                };
                changed = true;
            });
            if !changed {
                break;
            }
        }
    }
}
