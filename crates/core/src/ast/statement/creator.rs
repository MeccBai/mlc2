use super::{Statement, Variable};
use crate::ast::arena::FuncIndex;
use crate::ast::config::Config;
use crate::ast::expression::Expression;
use crate::ast::statement::ReturnStatement;
use crate::ast::symbols::Resolution;
use crate::ast::symbols::{EnumBool, StatementContext, SymbolTable};
use crate::diagnostic::error::CompileError;
use crate::diagnostic::error::IllegalUseError;
use crate::parser::out::{Spanned, TempStmt};

impl Statement {
    pub fn new(
        config: &mut Config,
        prototype: Spanned<TempStmt>,
        symbols: &mut dyn Resolution,
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
        symbols: &mut dyn Resolution,
        context: &mut StatementContext,
    ) -> Vec<Self> {
        let mut statements = Vec::new();
        let mut spans = Vec::new();
        for statement in scope.statements {
            if config.is_poisoned() {
                break;
            }
            spans.push(statement.1);
            statements.push(Self::new(config, statement, symbols, Some(context)));
        }
        if context.frames().len() == 1 && !config.is_poisoned() {
            let params = match context.belong() {
                EnumBool::False(index) if !index.is_empty() => {
                    symbols.get_function_regular(*index).params.clone()
                }
                EnumBool::True(index) if !index.is_empty() => {
                    symbols.get_interface_regular(*index).params.clone()
                }
                _ => Vec::new(),
            };
            context.resources.parameters(&params, symbols);
            let result = match context.belong() {
                EnumBool::False(index) if !index.is_empty() => {
                    symbols.get_function_regular(*index).ret_type
                }
                EnumBool::True(index) if !index.is_empty() => {
                    symbols.get_interface_regular(*index).ret_type
                }
                _ => None,
            };
            for (statement, span) in statements.iter().zip(spans) {
                context
                    .resources
                    .statement(statement, result, config, symbols, span);
                if config.is_poisoned() {
                    break;
                }
            }
        }
        statements
    }

    fn create(
        config: &mut Config,
        prototype: Spanned<TempStmt>,
        symbols: &mut dyn Resolution,
        mut context: Option<&mut StatementContext>,
    ) -> Self {
        let (stmt, span) = prototype;

        match stmt {
            TempStmt::VariantMatch { binding, value, branches } => Self::create_variant_match(config, binding, value, branches, symbols, context.as_deref()),
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
                let target_type = left_expr.type_inference(config, symbols);
                let source_type = right_expr.type_inference(config, symbols);
                let function_pointer = [target_type, source_type].iter().any(|ty| {
                    !ty.is_empty()
                        && matches!(
                            symbols.get_type(*ty).unqualified(),
                            crate::ast::types::CompileType::Function(_)
                        )
                });
                if function_pointer && !right_expr.type_check(&target_type, config, symbols) {
                    config.submit_error(
                        CompileError::IllegalUse(IllegalUseError::TypeMismatched {
                            expected: target_type.format(symbols),
                            found: source_type.format(symbols),
                        }),
                        span,
                    );
                    return Self::Poison;
                }
                right_expr.check_constant_range(target_type, config, symbols, span);
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
