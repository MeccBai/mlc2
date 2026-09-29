use super::Statement;
use super::Variable;
use crate::ast::arena::{FuncIndex, InterfaceIndex};
use crate::ast::config::Config;
use crate::ast::expression::{Expression, FuncCall};
use crate::ast::statement::ReturnStatement;
use crate::ast::types::resolve_type;
use crate::ast::{EnumBool, SymbolTable, TypeIndex};
use crate::error::CompileError;
use crate::error::IllegalUseError;
use crate::parser::out::{Spanned, TempStmt};
use std::collections::HashMap;
use std::hash::Hash;
use std::rc::Rc;

impl Statement {
    pub fn new(
        config: &mut Config,
        prototype: Spanned<TempStmt>,
        symbols: &mut SymbolTable,
        context: Option<&mut HashMap<String, Rc<Variable>>>,
    ) -> Self {
        let (stmt, span) = prototype;

        match stmt {
            TempStmt::Variable(temp_var) => {
                Statement::VariableDecl(Variable::new(config, temp_var, symbols, context))
            }
            TempStmt::Assignment { target, value } => {
                let expr = Expression::new(config, value, symbols, context.map(|m| &*m));
                todo!()
            }
            TempStmt::Expression(expr) => {
                let immut: Option<&HashMap<String, Rc<Variable>>> = context.map(|m| &*m);
                Self::Expression(Expression::new(config, expr, symbols, immut))
            }
            TempStmt::Return(expr) => {
                let immut: Option<&HashMap<String, Rc<Variable>>> = context.map(|m| &*m);
                match expr {
                    Some(e) => Self::ReturnBlock(ReturnStatement {
                        value: Some(Expression::new(config, e, symbols, immut)),
                    }),
                    None => Self::ReturnBlock(ReturnStatement { value: None }),
                }
            }
            TempStmt::If {
                condition,
                then_scope,
                else_scope,
            } => {
                todo!()
            }
            TempStmt::While { condition, scope } => {
                todo!()
            }
            TempStmt::Match { value, branches } => {
                todo!()
            }
            TempStmt::For {
                binding,
                start,
                end,
                scope,
            } => {
                todo!()
            }
            TempStmt::Anonymous(scope) => {
                todo!()
            }
            TempStmt::Break => Self::Break,
            TempStmt::Continue => Self::Continue,
        }
    }
}
