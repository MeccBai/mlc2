use super::{Statement, Variable};
use crate::ast::arena::FuncIndex;
use crate::ast::config::Config;
use crate::ast::expression::Expression;
use crate::ast::statement::ReturnStatement;
use crate::ast::symbols::{EnumBool, StatementContext, SymbolTable};
use crate::error::CompileError;
use crate::error::IllegalUseError;
use crate::parser::out::{Spanned, TempStmt};

impl Statement {
    pub fn new(
        config: &mut Config,
        prototype: Spanned<TempStmt>,
        symbols: &mut SymbolTable,
        context: Option<&mut StatementContext>,
    ) -> Self {
        if config.is_poisoned() {
            return Self::Poison;
        }
        let statement = Self::create(config, prototype, symbols, context);
        if config.is_poisoned() {
            Self::Poison
        } else {
            statement
        }
    }

    pub(crate) fn parse_scope(
        config: &mut Config,
        scope: crate::parser::Scope,
        symbols: &mut SymbolTable,
        context: &mut StatementContext,
    ) -> Vec<Self> {
        let mut statements = Vec::new();
        for statement in scope.statements {
            if config.is_poisoned() {
                break;
            }
            statements.push(Self::new(config, statement, symbols, Some(context)));
        }
        statements
    }

    fn create(
        config: &mut Config,
        prototype: Spanned<TempStmt>,
        symbols: &mut SymbolTable,
        mut context: Option<&mut StatementContext>,
    ) -> Self {
        let (stmt, span) = prototype;

        match stmt {
            TempStmt::Variable(temp_var) => {
                let span = temp_var.name_span;
                let variable = Variable::new(config, temp_var, symbols, context.as_deref_mut());
                if variable.is_poisoned() {
                    return Self::Poison;
                }
                if let Some(context) = context {
                    if !context.declare(config, variable.clone(), span) {
                        return Self::Poison;
                    }
                }
                Statement::VariableDecl(variable)
            }
            TempStmt::Assignment { target, value } => {
                let ctxt = context.map(|ctxt| &*ctxt);
                let right_expr = Expression::new(config, value, symbols, ctxt);
                if right_expr.is_poisoned() {
                    return Self::Poison;
                }
                let left_expr = Expression::new(config, target, symbols, ctxt);
                if left_expr.is_poisoned() {
                    return Self::Poison;
                }
                if left_expr.assignable(config, symbols) == false {
                    config.submit_error(
                        CompileError::IllegalUse(IllegalUseError::InvalidAssignment),
                        span,
                    );
                    return Self::Poison;
                }
                Self::Assignment(super::Assignment {
                    variable: Box::new(left_expr),
                    value: Box::new(right_expr),
                })
            }
            TempStmt::Expression(expr) => {
                let immut = context.map(|m| &*m);
                Self::Expression(Expression::new(config, expr, symbols, immut))
            }
            TempStmt::Return(expr) => {
                Self::create_return(config, expr, symbols, context.as_deref(), span)
            }
            TempStmt::If {
                condition,
                then_scope,
                else_scope,
            } => Self::create_if(
                config,
                condition,
                then_scope,
                else_scope,
                symbols,
                context.as_deref(),
            ),
            TempStmt::While { condition, scope } => {
                Self::create_while(config, condition, scope, symbols, context.as_deref())
            }
            TempStmt::Match { value, branches } => {
                Self::create_match(config, value, branches, symbols, context.as_deref())
            }
            TempStmt::For {
                binding,
                start,
                end,
                scope,
            } => Self::create_for(
                config,
                binding,
                start,
                end,
                scope,
                symbols,
                context.as_deref(),
            ),
            TempStmt::Anonymous(scope) => {
                let mut child = context
                    .as_deref()
                    .cloned()
                    .unwrap_or_else(|| StatementContext::new(EnumBool::False(FuncIndex::empty())));
                child.push_frame();
                Self::AnonymousBlock(super::AnonymousBlock {
                    statements: Self::parse_scope(config, scope, symbols, &mut child),
                })
            }
            TempStmt::Break | TempStmt::Continue => {
                if context
                    .as_deref()
                    .and_then(|context| context.loop_scope())
                    .is_none()
                {
                    config.submit_error(
                        CompileError::IllegalUse(IllegalUseError::LoopControlOutsideLoop),
                        span,
                    );
                    Self::Poison
                } else if matches!(stmt, TempStmt::Break) {
                    Self::Break
                } else {
                    Self::Continue
                }
            }
        }
    }
}
