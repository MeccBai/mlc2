use super::FuncBody;
use crate::ast::{
    Config, TypeIndex,
    expression::{Access, CompAtom, Expression, FuncCall, InitialList, UnaryExpr},
    statement::{MatchPattern, Statement, Variable},
    symbols::{EnumBool, Resolution, StatementContext},
    types::CompileType,
};
use crate::diagnostic::error::{CompileError, IllegalUseError};
use crate::parser::out::Span;
use std::{collections::HashSet, rc::Rc};

fn pointers(ty: TypeIndex, symbols: &dyn Resolution, seen: &mut HashSet<TypeIndex>) -> bool {
    if ty.is_empty() || !seen.insert(ty) {
        return false;
    }
    match symbols.get_type(ty).unqualified() {
        CompileType::Ref(_) | CompileType::Function(_) => true,
        CompileType::List(list) => pointers(list.element_type(), symbols, seen),
        CompileType::Unit(unit) => {
            unit.members
                .iter()
                .any(|m| pointers(m.member_type, symbols, seen))
                || unit.variant.as_ref().is_some_and(|variant| {
                    variant
                        .candidates
                        .iter()
                        .any(|ty| pointers(*ty, symbols, seen))
                })
        }
        _ => false,
    }
}
type Failure = (String, Span);
fn call(
    call: &FuncCall,
    symbols: &dyn Resolution,
    local: &dyn Fn(&Rc<Variable>) -> bool,
    span: Span,
) -> Result<(), Failure> {
    validate_call(call, symbols, span)?;
    for arg in &call.args {
        expression(arg, symbols, local)?;
    }
    Ok(())
}
pub(crate) fn validate_call(
    call: &FuncCall,
    symbols: &dyn Resolution,
    span: Span,
) -> Result<(), Failure> {
    if let EnumBool::False(index) = call.func {
        if symbols
            .get_function_regular(index)
            .attributes
            .contains(&crate::ast::attribute::FuncAttibute::Cabi)
        {
            return Err(("const functions cannot call C functions".into(), span));
        }
    }
    let allowed = call.callee.is_none()
        && matches!(call.func,EnumBool::False(i) if {let f=symbols.get_function_regular(i);f.constant||f.builtin().is_some()});
    if !allowed {
        return Err((
            "const functions can only call const functions; indirect calls are not allowed".into(),
            span,
        ));
    }
    Ok(())
}
pub(crate) fn check_call_boundary(
    call: &FuncCall,
    symbols: &dyn Resolution,
    config: &mut Config,
    context: Option<&StatementContext>,
    span: Span,
) {
    let parent_is_const = context.is_some_and(|context| match *context.belong() {
        EnumBool::False(index) if !index.is_empty() => symbols.get_function_regular(index).constant,
        _ => false,
    });
    if parent_is_const {
        return;
    }
    if let EnumBool::False(index) = call.func {
        let function = symbols.get_function_regular(index);
        if function.constant && function.builtin().is_none() {
            report(validate_inputs(function, symbols, span), config);
        }
    }
}
fn validate_inputs(
    symbol: &super::FuncSymbol,
    symbols: &dyn Resolution,
    span: Span,
) -> Result<(), Failure> {
    if let Some((ty, name)) = symbol
        .params
        .iter()
        .find(|(ty, _)| pointers(*ty, symbols, &mut HashSet::new()))
    {
        return Err((
            format!(
                "const function input parameter {name} cannot contain pointers when called outside a const function: {}",
                ty.format(symbols)
            ),
            span,
        ));
    }
    Ok(())
}
fn expression(
    value: &Expression,
    symbols: &dyn Resolution,
    local: &dyn Fn(&Rc<Variable>) -> bool,
) -> Result<(), Failure> {
    let atom = |value: &CompAtom| expression(&value.clone().to_expression(), symbols, local);
    match value {
        Expression::VarValueE(variable, span) if !local(variable) && !variable.constant => {
            return Err((
                "const functions cannot access global var or val bindings".into(),
                *span,
            ));
        }
        Expression::FuncCallE(value, span) => call(value, symbols, local, *span)?,
        Expression::CompositeE(value, ..) => {
            for value in &value.members {
                atom(value)?;
            }
        }
        Expression::UnaryExprE(UnaryExpr::Operator { value, .. }, ..) => atom(value)?,
        Expression::UnaryExprE(UnaryExpr::Access(Access::Member { base, .. }), ..) => atom(base)?,
        Expression::UnaryExprE(UnaryExpr::Access(Access::Index { base, index }), ..) => {
            expression(base, symbols, local)?;
            expression(index, symbols, local)?;
        }
        Expression::InitListE(
            InitialList::Array { values, .. } | InitialList::List { values, .. },
            ..,
        ) => {
            for value in values {
                expression(value, symbols, local)?;
            }
        }
        _ => (),
    }
    Ok(())
}
fn report(result: Result<(), Failure>, config: &mut Config) -> bool {
    if let Err((reason, span)) = result {
        config.submit_error(
            CompileError::IllegalUse(IllegalUseError::ConstantEvaluation { reason }),
            span,
        );
        false
    } else {
        true
    }
}
impl Expression {
    pub(crate) fn check_constant_context(
        &self,
        symbols: &mut dyn Resolution,
        config: &mut Config,
        context: &StatementContext,
    ) {
        let EnumBool::False(index) = *context.belong() else {
            return;
        };
        if index.is_empty() || !symbols.get_function_regular(index).constant {
            return;
        }
        // Check global var and val references before folding.
        if !report(
            expression(self, symbols, &|variable| {
                context
                    .lookup(&variable.name)
                    .is_some_and(|v| Rc::ptr_eq(v, variable))
            }),
            config,
        ) {
            return;
        }
    }
}
fn local_names(body: &[Statement], names: &mut HashSet<String>) {
    for statement in body {
        match statement {
            Statement::VariableDecl(v) => {
                names.insert(v.name.clone());
            }
            Statement::IfBlock(s) => {
                local_names(&s.then_branch, names);
                if let Some(body) = &s.else_branch {
                    local_names(body, names);
                }
            }
            Statement::WhileBlock(s) => local_names(&s.stmts, names),
            Statement::ForBlock(s) => {
                if let Some(init) = &s.init {
                    local_names(std::slice::from_ref(init.as_ref()), names);
                }
                local_names(&s.stmts, names);
            }
            Statement::MatchBlock(s) => {
                for (_, body) in &s.branches {
                    local_names(body, names);
                }
            }
            Statement::VariantMatch(s) => {
                for (_, binding, body) in &s.branches {
                    names.insert(binding.name.clone());
                    local_names(body, names);
                }
            }
            Statement::AnonymousBlock(s) => local_names(&s.statements, names),
            _ => (),
        }
    }
}
fn statements(
    body: &[Statement],
    symbols: &dyn Resolution,
    local: &dyn Fn(&Rc<Variable>) -> bool,
    span: Span,
) -> Result<(), Failure> {
    for statement in body {
        match statement {
            Statement::VariableDecl(v) => expression(&v.init_val, symbols, local)?,
            Statement::Assignment(s) => {
                expression(&s.variable, symbols, local)?;
                expression(&s.value, symbols, local)?;
            }
            Statement::FuncCall(s) => call(s, symbols, local, span)?,
            Statement::Expression(s) => expression(s, symbols, local)?,
            Statement::ReturnBlock(s) => {
                if let Some(value) = &s.value {
                    expression(value, symbols, local)?;
                }
            }
            Statement::IfBlock(s) => {
                expression(&s.condition, symbols, local)?;
                statements(&s.then_branch, symbols, local, span)?;
                if let Some(body) = &s.else_branch {
                    statements(body, symbols, local, span)?;
                }
            }
            Statement::WhileBlock(s) => {
                expression(&s.condition, symbols, local)?;
                statements(&s.stmts, symbols, local, span)?;
            }
            Statement::ForBlock(s) => {
                if let Some(init) = &s.init {
                    statements(std::slice::from_ref(init.as_ref()), symbols, local, span)?;
                }
                expression(&s.condition, symbols, local)?;
                if let Some(step) = &s.execution {
                    statements(std::slice::from_ref(step.as_ref()), symbols, local, span)?;
                }
                statements(&s.stmts, symbols, local, span)?;
            }
            Statement::MatchBlock(s) => {
                expression(&s.value, symbols, local)?;
                for (pattern, body) in &s.branches {
                    if let MatchPattern::Value(value) = pattern {
                        expression(value, symbols, local)?;
                    }
                    statements(body, symbols, local, span)?;
                }
            }
            Statement::VariantMatch(s) => {
                expression(&s.value, symbols, local)?;
                for (_, _, body) in &s.branches {
                    statements(body, symbols, local, span)?;
                }
            }
            Statement::AnonymousBlock(s) => statements(&s.statements, symbols, local, span)?,
            _ => (),
        }
    }
    Ok(())
}
impl FuncBody {
    pub fn constant_check(&self, symbols: &dyn Resolution, config: &mut Config) -> bool {
        let symbol = symbols.get_function_regular(self.symbol);
        if !symbol.constant {
            return true;
        }
        let mut names: HashSet<_> = symbol.params.iter().map(|p| p.1.clone()).collect();
        local_names(&self.body, &mut names);
        if !report(
            statements(&self.body, symbols, &|v| names.contains(&v.name), self.span),
            config,
        ) {
            return false;
        }
        !config.is_poisoned()
    }
}
