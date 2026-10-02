//! Append-only construction history with a restorable active stack head.
//! Sibling scopes never reuse positions referenced by previously emitted exits.
use super::{LlvmValue, error::fail};
use crate::ast::arena::TypeIndex;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StackPosition(Option<usize>);

impl StackPosition {
    pub fn index(self) -> Option<usize> {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackVariable {
    pub name: String,
    pub ty: TypeIndex,
    pub storage: LlvmValue,
    pub previous: StackPosition,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct VariableStack {
    variables: Vec<StackVariable>,
    current: StackPosition,
}

impl VariableStack {
    pub fn position(&self) -> StackPosition {
        self.current
    }
    pub fn variables(&self) -> &[StackVariable] {
        &self.variables
    }

    /// Call only after initialization has completed successfully.
    pub fn push(&mut self, name: String, ty: TypeIndex, mut storage: LlvmValue) -> StackPosition {
        if storage.in_reg || storage.reg.is_none() {
            fail("VariableStack requires initialized storage");
        }
        storage.code.clear();
        let previous = self.current;
        self.current = StackPosition(Some(self.variables.len()));
        self.variables.push(StackVariable {
            name,
            ty,
            storage,
            previous,
        });
        self.current
    }

    /// Reverse construction order, excluding the surviving outer scope.
    pub fn between(&self, from: StackPosition, to: StackPosition) -> Vec<&StackVariable> {
        let mut result = Vec::new();
        let mut cursor = from;
        while cursor != to {
            let index = cursor
                .0
                .unwrap_or_else(|| fail("Exit target is not an ancestor of the variable stack"));
            let variable = self
                .variables
                .get(index)
                .unwrap_or_else(|| fail("Invalid variable stack position"));
            result.push(variable);
            cursor = variable.previous;
        }
        result
    }

    pub fn restore(&mut self, position: StackPosition) {
        self.between(self.current, position);
        self.current = position;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitKind {
    Scope,
    Return,
    Break,
    Continue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeExit {
    pub kind: ExitKind,
    pub from: StackPosition,
    pub to: StackPosition,
    pub target: String,
}

#[cfg(test)]
mod tests;
