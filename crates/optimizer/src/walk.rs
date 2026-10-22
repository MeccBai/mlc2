//! Traversal shared by passes; variable references never recurse into declarations.
use crate::ast::{
    expression::{Access, CompAtom, Expression, InitialList, UnaryExpr},
    statement::Statement,
};
use std::rc::Rc;

pub(crate) fn expression(value: &mut Expression, visit: &mut impl FnMut(&mut Expression)) {
    match value {
        Expression::CompositeE(composite, ..) => {
            for atom in &mut composite.members {
                atom_expression(atom, visit);
            }
        }
        Expression::UnaryExprE(UnaryExpr::Operator { value, .. }, ..) => {
            atom_expression(value, visit)
        }
        Expression::UnaryExprE(UnaryExpr::Access(Access::Member { base, .. }), ..) => {
            atom_expression(base, visit)
        }
        Expression::UnaryExprE(UnaryExpr::Access(Access::Index { base, index }), ..) => {
            expression(base, visit);
            expression(index, visit);
        }
        Expression::FuncCallE(call, ..) => {
            if let Some(callee) = &mut call.callee {
                expression(callee, visit);
            }
            for argument in &mut call.args {
                expression(argument, visit);
            }
        }
        Expression::InitListE(
            InitialList::List { values, .. } | InitialList::Array { values, .. },
            ..,
        ) => {
            for value in values {
                expression(value, visit);
            }
        }
        _ => {}
    }
    visit(value);
}

fn atom_expression(atom: &mut CompAtom, visit: &mut impl FnMut(&mut Expression)) {
    let mut value = atom.clone().to_expression();
    expression(&mut value, visit);
    // Aggregate constants cannot occupy scalar atom slots. Keep their binding,
    // especially when it supplies storage for a borrowed native str.
    if !matches!(value, Expression::InitListE(_, ..)) {
        *atom = CompAtom::from_expr(value);
    }
}

pub(crate) fn expressions(scope: &mut [Statement], visit: &mut impl FnMut(&mut Expression)) {
    let mut replacements = std::collections::HashMap::new();
    statements(scope, &mut |statement| {
        let mut rewrite = |value: &mut Expression| {
            rebind(value, &replacements);
            visit(value);
        };
        match statement {
            Statement::VariableDecl(variable) => {
                let mut init = (*variable.init_val).clone();
                expression(&mut init, &mut rewrite);
                if init != *variable.init_val {
                    let previous = variable.clone();
                    let identity = Rc::as_ptr(&previous) as usize;
                    Rc::make_mut(variable).init_val = Box::new(init);
                    replacements.insert(identity, (previous, variable.clone()));
                }
            }
            Statement::Expression(value) => expression(value, &mut rewrite),
            Statement::FuncCall(call) => {
                let mut value = Expression::FuncCallE(call.clone(), (0..0).into());
                expression(&mut value, &mut rewrite);
                *statement = Statement::Expression(value);
            }
            Statement::Assignment(assignment) => {
                expression(&mut assignment.variable, &mut rewrite);
                expression(&mut assignment.value, &mut rewrite);
            }
            Statement::ReturnBlock(ret) => {
                if let Some(value) = &mut ret.value {
                    expression(value, &mut rewrite);
                }
            }
            Statement::IfBlock(block) => expression(&mut block.condition, &mut rewrite),
            Statement::WhileBlock(block) => expression(&mut block.condition, &mut rewrite),
            Statement::ForBlock(block) => expression(&mut block.condition, &mut rewrite),
            Statement::MatchBlock(block) => {
                expression(&mut block.value, &mut rewrite);
                for (pattern, _) in &mut block.branches {
                    if let crate::ast::statement::MatchPattern::Value(value) = pattern {
                        expression(value, &mut rewrite);
                    }
                }
            }
            Statement::VariantMatch(block) => expression(&mut block.value, &mut rewrite),
            _ => {}
        }
    });
}

pub(crate) type Replacements = std::collections::HashMap<
    usize,
    (
        Rc<crate::ast::statement::Variable>,
        Rc<crate::ast::statement::Variable>,
    ),
>;
pub(crate) fn rebind(value: &mut Expression, replacements: &Replacements) {
    if let Expression::VarValueE(variable, ..) = value {
        while let Some((_, replacement)) = replacements.get(&(Rc::as_ptr(variable) as usize)) {
            *variable = replacement.clone();
        }
    }
}

pub(crate) fn statements(scope: &mut [Statement], visit: &mut impl FnMut(&mut Statement)) {
    for statement in scope {
        match statement {
            Statement::IfBlock(block) => {
                statements(&mut block.then_branch, visit);
                if let Some(branch) = &mut block.else_branch {
                    statements(branch, visit);
                }
            }
            Statement::WhileBlock(block) => statements(&mut block.stmts, visit),
            Statement::ForBlock(block) => {
                if let Some(init) = &mut block.init {
                    statements(std::slice::from_mut(init.as_mut()), visit);
                }
                if let Some(step) = &mut block.execution {
                    statements(std::slice::from_mut(step.as_mut()), visit);
                }
                statements(&mut block.stmts, visit);
            }
            Statement::MatchBlock(block) => {
                for (_, branch) in &mut block.branches {
                    statements(branch, visit);
                }
            }
            Statement::VariantMatch(block) => {
                for (_, _, branch) in &mut block.branches {
                    statements(branch, visit);
                }
            }
            Statement::AnonymousBlock(block) => statements(&mut block.statements, visit),
            _ => {}
        }
        visit(statement);
    }
}
