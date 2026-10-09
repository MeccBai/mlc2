//! Conservative ownership analysis over checked statements, using branch snapshots.
use super::Statement;
use crate::ast::{
    TypeIndex,
    config::Config,
    expression::{Access, Expression, InitialList, UnaryExpr},
    symbols::{EnumBool, Resolution},
};
use crate::diagnostic::error::{CompileError, IllegalUseError};
use crate::parser::out::Span;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResourceState {
    moved: HashSet<String>,
    borrows: HashMap<String, HashSet<String>>,
    loop_outer: HashSet<String>,
    owners: HashSet<String>,
}

impl ResourceState {
    pub(crate) fn parameters(&mut self, params: &[(TypeIndex, String)], symbols: &dyn Resolution) {
        self.owners.extend(
            params
                .iter()
                .filter(|(ty, _)| ty.has_res(symbols))
                .map(|(_, name)| name.clone()),
        );
    }
    fn error(config: &mut Config, error: IllegalUseError, span: Span) {
        config.submit_error(CompileError::IllegalUse(error), span);
    }

    fn owner_names(&self, expression: &Expression) -> HashSet<String> {
        match expression {
            Expression::VarValueE(variable) => self
                .borrows
                .get(&variable.name)
                .cloned()
                .unwrap_or_else(|| HashSet::from([variable.name.clone()])),
            Expression::UnaryExprE(UnaryExpr::Access(Access::Index { base, .. })) => {
                self.owner_names(base)
            }
            Expression::UnaryExprE(UnaryExpr::Access(Access::Member { base, .. }))
            | Expression::UnaryExprE(UnaryExpr::Operator { value: base, .. }) => {
                self.owner_names(&base.clone().to_expression())
            }
            Expression::FuncCallE(call) => call
                .args
                .iter()
                .flat_map(|argument| self.owner_names(argument))
                .collect(),
            _ => HashSet::new(),
        }
    }

    pub(crate) fn statement(
        &mut self,
        statement: &Statement,
        result: Option<TypeIndex>,
        config: &mut Config,
        symbols: &mut dyn Resolution,
        span: Span,
    ) {
        match statement {
            Statement::VariantMatch(block) => {
                self.expression(&block.value, None, config, symbols, span);
                let owners = self.owner_names(&block.value);
                let before = self.clone();
                for (_, binding, body) in &block.branches {
                    let mut branch = before.clone();
                    branch.owners.extend(owners.iter().cloned());
                    branch.borrows.insert(binding.name.clone(), owners.clone());
                    branch.scope(body, result, config, symbols, span, false);
                    self.moved.extend(branch.moved);
                }
            }
            Statement::VariableDecl(variable) => {
                let ty = variable.var_type;
                self.expression(
                    &variable.init_val,
                    Some(ty),
                    config,
                    symbols,
                    variable.declaration_span,
                );
                if ty.has_res(symbols)
                    || matches!(symbols.get_type(ty).unqualified(), crate::ast::types::CompileType::Unit(unit) if unit.variant.is_some())
                {
                    self.owners.insert(variable.name.clone());
                } else if ty.is_ref(symbols) {
                    let owners = self
                        .owner_names(&variable.init_val)
                        .into_iter()
                        .filter(|name| self.owners.contains(name))
                        .collect::<HashSet<_>>();
                    if !owners.is_empty() {
                        self.borrows.insert(variable.name.clone(), owners);
                    }
                }
                self.moved.remove(&variable.name);
            }
            Statement::Assignment(assign) => {
                let ty = assign.variable.type_inference(config, symbols);
                if matches!(symbols.get_type(ty).unqualified(), crate::ast::types::CompileType::Unit(unit) if unit.variant.is_some())
                {
                    let targets = self.owner_names(&assign.variable);
                    if let Some(name) = self
                        .borrows
                        .values()
                        .flat_map(|owners| owners.iter())
                        .find(|name| targets.contains(*name))
                    {
                        Self::error(
                            config,
                            IllegalUseError::ResourceMoveWhileBorrowed { name: name.clone() },
                            span,
                        );
                        return;
                    }
                }
                if let Expression::VarValueE(target) = &*assign.variable {
                    if self
                        .borrows
                        .values()
                        .any(|owners| owners.contains(&target.name))
                    {
                        Self::error(
                            config,
                            IllegalUseError::ResourceMoveWhileBorrowed {
                                name: target.name.clone(),
                            },
                            span,
                        );
                        return;
                    }
                }
                if ty.has_res(symbols) {
                    let Expression::VarValueE(target) = &*assign.variable else {
                        Self::error(config, IllegalUseError::UnsupportedResourceOperation, span);
                        return;
                    };
                    if self
                        .borrows
                        .values()
                        .any(|owners| owners.contains(&target.name))
                    {
                        Self::error(
                            config,
                            IllegalUseError::ResourceMoveWhileBorrowed {
                                name: target.name.clone(),
                            },
                            span,
                        );
                        return;
                    }
                    self.expression(&assign.value, Some(ty), config, symbols, span);
                    if self.owner_names(&assign.value).contains(&target.name) {
                        Self::error(config, IllegalUseError::UnsupportedResourceOperation, span);
                    }
                    self.moved.remove(&target.name);
                } else {
                    self.expression(&assign.variable, None, config, symbols, span);
                    self.expression(&assign.value, Some(ty), config, symbols, span);
                    if ty.is_ref(symbols)
                        && !self.owner_names(&assign.value).is_disjoint(&self.owners)
                    {
                        // Rebinding borrowed references needs a separate lifetime-aware assignment model.
                        Self::error(config, IllegalUseError::ResourceEscapesScope, span);
                    }
                }
            }
            Statement::Expression(expr) => {
                let ty = expr.type_inference(config, symbols);
                if ty.has_res(symbols) && !matches!(expr, Expression::VarValueE(_)) {
                    Self::error(config, IllegalUseError::ResourceRequiresOwner, span);
                }
                self.expression(expr, None, config, symbols, span);
            }
            Statement::FuncCall(call) => self.expression(
                &Expression::FuncCallE(call.clone()),
                None,
                config,
                symbols,
                span,
            ),
            Statement::ReturnBlock(ret) => {
                if let Some(expr) = &ret.value {
                    if result.is_some_and(|ty| ty.is_ref(symbols) && !ty.has_res(symbols))
                        && !self.owner_names(expr).is_disjoint(&self.owners)
                    {
                        Self::error(config, IllegalUseError::ResourceEscapesScope, span);
                    }
                    self.expression(expr, result, config, symbols, span);
                }
            }
            Statement::IfBlock(block) => {
                self.expression(&block.condition, None, config, symbols, span);
                let mut then_state = self.clone();
                then_state.scope(&block.then_branch, result, config, symbols, span, false);
                let mut else_state = self.clone();
                if let Some(body) = &block.else_branch {
                    else_state.scope(body, result, config, symbols, span, false);
                }
                self.moved.extend(then_state.moved);
                self.moved.extend(else_state.moved);
            }
            Statement::MatchBlock(block) => {
                self.expression(&block.value, None, config, symbols, span);
                let before = self.clone();
                for (_, body) in &block.branches {
                    let mut branch = before.clone();
                    branch.scope(body, result, config, symbols, span, false);
                    self.moved.extend(branch.moved);
                }
            }
            Statement::WhileBlock(block) => {
                self.expression(&block.condition, None, config, symbols, span);
                self.scope(&block.stmts, result, config, symbols, span, true);
            }
            Statement::ForBlock(block) => {
                if let Some(init) = &block.init {
                    self.statement(init, result, config, symbols, span);
                }
                self.expression(&block.condition, None, config, symbols, span);
                self.scope(&block.stmts, result, config, symbols, span, true);
            }
            Statement::AnonymousBlock(block) => {
                self.scope(&block.statements, result, config, symbols, span, false)
            }
            _ => {}
        }
    }

    fn scope(
        &mut self,
        body: &[Statement],
        result: Option<TypeIndex>,
        config: &mut Config,
        symbols: &mut dyn Resolution,
        span: Span,
        looping: bool,
    ) {
        let before = self.clone();
        if looping {
            self.loop_outer.extend(self.owners.iter().cloned());
        }
        let mut locals = Vec::new();
        for statement in body {
            if config.is_poisoned() {
                break;
            }
            if let Statement::VariableDecl(variable) = statement {
                locals.push(variable.name.clone());
            }
            self.statement(statement, result, config, symbols, span);
        }
        for name in locals {
            self.borrows.remove(&name);
            self.owners.remove(&name);
            self.moved.remove(&name);
        }
        self.loop_outer = before.loop_outer;
    }

    fn expression(
        &mut self,
        expr: &Expression,
        expected: Option<TypeIndex>,
        config: &mut Config,
        symbols: &mut dyn Resolution,
        span: Span,
    ) {
        if config.is_poisoned() {
            return;
        }
        let transfer = expected.is_some_and(|ty| ty.has_res(symbols));
        if transfer && matches!(expr, Expression::UnaryExprE(_) | Expression::CompositeE(_)) {
            Self::error(config, IllegalUseError::UnsupportedResourceOperation, span);
            return;
        }
        match expr {
            Expression::VarValueE(variable) => {
                let names = self.owner_names(expr);
                if let Some(name) = names.iter().find(|name| self.moved.contains(*name)) {
                    Self::error(
                        config,
                        IllegalUseError::ResourceUseAfterMove { name: name.clone() },
                        span,
                    );
                    return;
                }
                if variable.var_type.has_res(symbols) {
                    self.owners.insert(variable.name.clone());
                    if transfer {
                        if self.loop_outer.contains(&variable.name) {
                            Self::error(config, IllegalUseError::ResourceLoopMove, span);
                        } else if self
                            .borrows
                            .values()
                            .any(|owners| owners.contains(&variable.name))
                        {
                            Self::error(
                                config,
                                IllegalUseError::ResourceMoveWhileBorrowed {
                                    name: variable.name.clone(),
                                },
                                span,
                            );
                        } else {
                            self.moved.insert(variable.name.clone());
                        }
                    }
                }
            }
            Expression::FuncCallE(call) => {
                if let Some(callee) = &call.callee {
                    self.expression(callee, None, config, symbols, span);
                }
                let (params, builtin) = match call.func {
                    EnumBool::False(index) => {
                        let symbol = symbols.get_function_regular(index);
                        (symbol.params.clone(), symbol.builtin())
                    }
                    EnumBool::True(index) => {
                        (symbols.get_interface_regular(index).params.clone(), None)
                    }
                };
                if builtin == Some(crate::ast::builtins::Builtin::Cast) {
                    if call
                        .args
                        .iter()
                        .any(|arg| arg.clone().type_inference(config, symbols).has_res(symbols))
                    {
                        Self::error(config, IllegalUseError::UnsupportedResourceOperation, span);
                        return;
                    }
                }
                let offset = usize::from(matches!(call.func, EnumBool::True(_)));
                let mut argument_borrows = HashSet::new();
                for (index, arg) in call.args.iter().enumerate() {
                    let ty = index
                        .checked_sub(offset)
                        .and_then(|i| params.get(i))
                        .map(|p| p.0);
                    let owners = self.owner_names(arg);
                    if ty.is_some_and(|ty| ty.has_res(symbols)) {
                        if let Some(name) = owners.intersection(&argument_borrows).next() {
                            Self::error(
                                config,
                                IllegalUseError::ResourceMoveWhileBorrowed { name: name.clone() },
                                span,
                            );
                            return;
                        }
                    } else if ty.is_some_and(|ty| !ty.is_empty() && ty.is_ref(symbols)) {
                        argument_borrows.extend(owners);
                    }
                    self.expression(arg, ty, config, symbols, span);
                }
                if expr
                    .clone()
                    .type_inference(config, symbols)
                    .has_res(symbols)
                    && !transfer
                {
                    Self::error(config, IllegalUseError::ResourceRequiresOwner, span);
                }
            }
            Expression::CompositeE(composite) => {
                for atom in &composite.members {
                    self.expression(&atom.clone().to_expression(), None, config, symbols, span);
                }
            }
            Expression::UnaryExprE(UnaryExpr::Operator { op, value }) => {
                if matches!(
                    op,
                    crate::ast::expression::operators::Operator::AddressOf
                        | crate::ast::expression::operators::Operator::MutOf
                ) && value
                    .clone()
                    .to_expression()
                    .type_inference(config, symbols)
                    .is_resource(symbols)
                {
                    Self::error(config, IllegalUseError::UnsupportedResourceOperation, span);
                    return;
                }
                self.expression(&value.clone().to_expression(), None, config, symbols, span)
            }
            Expression::UnaryExprE(UnaryExpr::Access(Access::Index { base, index })) => {
                self.expression(base, None, config, symbols, span);
                self.expression(index, None, config, symbols, span);
            }
            Expression::UnaryExprE(UnaryExpr::Access(Access::Member { base, .. })) => {
                self.expression(&base.clone().to_expression(), None, config, symbols, span)
            }
            Expression::InitListE(list) => {
                let (owner, values) = match list {
                    InitialList::Array { ty, values } => (Some(*ty), values),
                    InitialList::List { onwer, values } => (onwer.or(expected), values),
                    InitialList::String { .. } => return,
                };
                let actual = values
                    .first()
                    .map(|value| value.type_inference(config, symbols));
                let fields = owner
                    .map(|ty| match symbols.get_type(ty).unqualified() {
                        crate::ast::types::CompileType::Unit(unit) if unit.variant.is_some() => {
                            unit.variant
                                .as_ref()
                                .unwrap()
                                .candidates
                                .iter()
                                .copied()
                                .filter(|candidate| {
                                    actual.is_some_and(|actual| {
                                        !actual.is_empty()
                                            && symbols
                                                .get_type(*candidate)
                                                .unqualified()
                                                .format(symbols)
                                                == symbols
                                                    .get_type(actual)
                                                    .unqualified()
                                                    .format(symbols)
                                    })
                                })
                                .take(1)
                                .collect()
                        }
                        crate::ast::types::CompileType::Unit(unit) => unit
                            .members
                            .iter()
                            .map(|m| m.member_type)
                            .collect::<Vec<_>>(),
                        crate::ast::types::CompileType::List(list) => {
                            vec![list.element_type; values.len()]
                        }
                        _ => Vec::new(),
                    })
                    .unwrap_or_default();
                for (index, value) in values.iter().enumerate() {
                    self.expression(value, fields.get(index).copied(), config, symbols, span);
                }
            }
            _ => {}
        }
    }
}
