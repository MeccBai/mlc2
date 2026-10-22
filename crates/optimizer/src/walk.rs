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
    statements(scope, &mut |statement| match statement {
        Statement::VariableDecl(variable) => {
            let mut init = (*variable.init_val).clone();
            expression(&mut init, visit);
            if init != *variable.init_val {
                Rc::make_mut(variable).init_val = Box::new(init);
            }
        }
        Statement::Expression(value) => expression(value, visit),
        Statement::FuncCall(call) => {
            let mut value = Expression::FuncCallE(call.clone(), (0..0).into());
            expression(&mut value, visit);
            *statement = Statement::Expression(value);
        }
        Statement::Assignment(assignment) => {
            expression(&mut assignment.variable, visit);
            expression(&mut assignment.value, visit);
        }
        Statement::ReturnBlock(ret) => {
            if let Some(value) = &mut ret.value {
                expression(value, visit);
            }
        }
        Statement::IfBlock(block) => expression(&mut block.condition, visit),
        Statement::WhileBlock(block) => expression(&mut block.condition, visit),
        Statement::ForBlock(block) => expression(&mut block.condition, visit),
        Statement::MatchBlock(block) => {
            expression(&mut block.value, visit);
            for (pattern, _) in &mut block.branches {
                if let crate::ast::statement::MatchPattern::Value(value) = pattern {
                    expression(value, visit);
                }
            }
        }
        Statement::VariantMatch(block) => expression(&mut block.value, visit),
        _ => {}
    });
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
