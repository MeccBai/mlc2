use std::collections::HashMap;
use std::rc::Rc;

use super::EnumBool;
use crate::ast::arena::{FuncIndex, GenericIndex, InterfaceIndex};
use crate::ast::config::Config;
use crate::ast::statement::Variable;
use crate::diagnostic::error::{CompileError, IllegalUseError};
use crate::parser::out::Span;

/// One lexical scope. Declaration order is retained for future cleanup lowering.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScopeFrame {
    names: HashMap<String, Rc<Variable>>,
    locals: Vec<Rc<Variable>>,
}

impl ScopeFrame {
    pub fn new() -> Self {
        Self::default()
    }

    /// Reject duplicate names in this scope without replacing the first binding.
    fn declare(&mut self, variable: Rc<Variable>) -> Result<(), Rc<Variable>> {
        if let Some(existing) = self.names.get(&variable.name) {
            return Err(Rc::clone(existing));
        }
        self.names
            .insert(variable.name.clone(), Rc::clone(&variable));
        self.locals.push(variable);
        Ok(())
    }

    pub fn lookup(&self, name: &str) -> Option<&Rc<Variable>> {
        self.names.get(name)
    }

    pub fn locals(&self) -> &[Rc<Variable>] {
        &self.locals
    }

    /// Compile-time cleanup candidates, not Rust Rc destruction or runtime drops.
    pub fn cleanup_order(&self) -> impl DoubleEndedIterator<Item = &Rc<Variable>> {
        self.locals.iter().rev()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatementContext {
    pub(crate) resources: crate::ast::statement::resources::ResourceState,
    /// Current lexical frame index and scope kind; parent scopes retain loop targets.
    pub supper_scope: (usize, SupperScopeType),
    enclosing_scopes: Vec<(usize, SupperScopeType)>,
    belong: EnumBool<InterfaceIndex, FuncIndex>,
    frames: Vec<ScopeFrame>,
    receiver: Option<Rc<Variable>>,
    generic_parameters: HashMap<String, GenericIndex>,
    parameters: HashMap<String, Rc<Variable>>,
    type_bindings: HashMap<String, crate::ast::TypeIndex>,
    instantiation_actives: Option<crate::ast::function::InstantiationActives>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupperScopeType {
    Function,
    If,
    While,
    For,
    Match,
    Anonymous,
}

#[derive(Debug, Clone, Copy)]
pub enum ContextBinding<'a> {
    Variable(&'a Rc<Variable>),
    Generic(GenericIndex),
}

impl StatementContext {
    pub fn new(belong: EnumBool<InterfaceIndex, FuncIndex>) -> Self {
        Self {
            resources: Default::default(),
            supper_scope: (0, SupperScopeType::Function),
            enclosing_scopes: Vec::new(),
            belong,
            frames: vec![ScopeFrame::new()],
            receiver: None,
            generic_parameters: HashMap::new(),
            parameters: HashMap::new(),
            type_bindings: HashMap::new(),
            instantiation_actives: None,
        }
    }

    pub fn belong(&self) -> &EnumBool<InterfaceIndex, FuncIndex> {
        &self.belong
    }

    pub fn type_bindings(&self) -> &HashMap<String, crate::ast::TypeIndex> {
        &self.type_bindings
    }

    pub(crate) fn set_instantiation_actives(
        &mut self,
        actives: crate::ast::function::InstantiationActives,
    ) {
        self.instantiation_actives = Some(actives);
    }

    pub(crate) fn instantiation_actives(
        &self,
    ) -> Option<&crate::ast::function::InstantiationActives> {
        self.instantiation_actives.as_ref()
    }

    /// Bind the concrete type of an already registered generic parameter.
    pub fn bind_generic_type(&mut self, name: &str, ty: crate::ast::TypeIndex) {
        assert!(
            self.generic_parameters.contains_key(name),
            "generic must be registered first"
        );
        self.type_bindings.insert(name.to_owned(), ty);
    }

    pub fn frames(&self) -> &[ScopeFrame] {
        &self.frames
    }

    pub fn push_frame(&mut self) {
        self.push_scope(SupperScopeType::Anonymous);
    }

    pub fn push_scope(&mut self, scope_type: SupperScopeType) {
        self.enclosing_scopes.push(self.supper_scope);
        self.supper_scope = (self.frames.len(), scope_type);
        self.frames.push(ScopeFrame::new());
    }

    pub fn loop_scope(&self) -> Option<(usize, SupperScopeType)> {
        std::iter::once(self.supper_scope)
            .chain(self.enclosing_scopes.iter().rev().copied())
            .find(|(_, kind)| matches!(kind, SupperScopeType::For | SupperScopeType::While))
    }

    /// Keep the function's root frame. The returned frame retains cleanup order.
    pub fn pop_frame(&mut self) -> Option<ScopeFrame> {
        if self.frames.len() == 1 {
            return None;
        }
        self.supper_scope = self
            .enclosing_scopes
            .pop()
            .expect("nested frame has parent scope");
        self.frames.pop()
    }

    pub fn declare(&mut self, config: &mut Config, variable: Rc<Variable>, span: Span) -> bool {
        if !self.check_insert(config, &variable, span) {
            return false;
        }
        self.frames
            .last_mut()
            .expect("statement context always has a root frame")
            .declare(variable)
            .expect("checked all visible bindings before insertion");
        true
    }

    fn check_insert(&self, config: &mut Config, variable: &Variable, span: Span) -> bool {
        if config.is_poisoned() || variable.is_poisoned() {
            return false;
        }
        if self.resolve(&variable.name).is_some() {
            config.submit_error(
                CompileError::IllegalUse(IllegalUseError::DuplicateVariable {
                    name: variable.name.clone(),
                }),
                span,
            );
            return false;
        }
        true
    }

    pub fn insert_self(&mut self, config: &mut Config, variable: Rc<Variable>, span: Span) -> bool {
        if !self.check_insert(config, &variable, span) {
            return false;
        }
        if variable.name != "self" || self.receiver.is_some() {
            config.submit_error(
                CompileError::IllegalUse(IllegalUseError::InvalidSelfBinding),
                span,
            );
            return false;
        }
        self.receiver = Some(variable);
        true
    }

    pub fn insert_generic_parameter(
        &mut self,
        config: &mut Config,
        name: String,
        index: GenericIndex,
        span: Span,
    ) -> bool {
        if config.is_poisoned() {
            return false;
        }
        if self.resolve(&name).is_some() {
            config.submit_error(
                CompileError::IllegalUse(IllegalUseError::DuplicateVariable { name }),
                span,
            );
            return false;
        }
        self.generic_parameters.insert(name, index);
        true
    }

    pub fn insert_parameter(
        &mut self,
        config: &mut Config,
        variable: Rc<Variable>,
        span: Span,
    ) -> bool {
        if !self.check_insert(config, &variable, span) {
            return false;
        }
        self.parameters.insert(variable.name.clone(), variable);
        true
    }

    pub fn lookup(&self, name: &str) -> Option<&Rc<Variable>> {
        match self.resolve(name)? {
            ContextBinding::Variable(variable) => Some(variable),
            ContextBinding::Generic(_) => None,
        }
    }

    pub fn generic_parameter(&self, name: &str) -> Option<GenericIndex> {
        self.generic_parameters.get(name).copied()
    }

    pub fn resolve(&self, name: &str) -> Option<ContextBinding<'_>> {
        self.receiver
            .as_ref()
            .filter(|receiver| receiver.name == name)
            .or_else(|| {
                self.frames
                    .iter()
                    .rev()
                    .find_map(|frame| frame.lookup(name))
            })
            .map(ContextBinding::Variable)
            .or_else(|| {
                self.generic_parameters
                    .get(name)
                    .copied()
                    .map(ContextBinding::Generic)
            })
            .or_else(|| self.parameters.get(name).map(ContextBinding::Variable))
    }
}

#[cfg(test)]
mod tests;
