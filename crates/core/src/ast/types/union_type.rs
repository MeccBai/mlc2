use crate::ast::{TypeIndex, config::Config, symbols::Resolution};
use crate::diagnostic::error::{CompileError, IllegalUseError};
use crate::parser::out::Span;

/// 专属 Variant 语义；声明、泛型和符号注册复用 Unit 的容器。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnionType {
    pub candidates: Vec<TypeIndex>,
}

impl UnionType {
    pub fn validate(&self, config: &mut Config, symbols: &dyn Resolution, span: Span) {
        let mut seen = std::collections::HashSet::new();
        for candidate in &self.candidates {
            if !candidate.is_generic(symbols) && !seen.insert(candidate.format(symbols)) {
                config.submit_error(
                    CompileError::IllegalUse(IllegalUseError::InvalidUnion {
                        reason: format!("Duplicate union candidate: {}", candidate.format(symbols)),
                    }),
                    span,
                );
                return;
            }
        }
    }

    pub fn align(&self, types: &(impl super::TypeLookup + ?Sized)) -> usize {
        self.candidates
            .iter()
            .map(|ty| ty.align(types))
            .max()
            .unwrap_or(1)
            .max(4)
    }

    pub fn payload_size(&self, types: &(impl super::TypeLookup + ?Sized)) -> usize {
        let align = self.align(types);
        let size = self
            .candidates
            .iter()
            .map(|ty| ty.size(types))
            .max()
            .unwrap_or(0);
        size.div_ceil(align) * align
    }

    pub fn size(&self, types: &(impl super::TypeLookup + ?Sized)) -> usize {
        self.align(types) + self.payload_size(types)
    }
}
