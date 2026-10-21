//! Source positions are retained independently of semantic expression equality.
use super::{CompAtom, Composite, Expression};
use crate::parser::out::Span;

impl Expression {
    pub fn span(&self) -> Span {
        match self {
            Self::Poison(span)
            | Self::VarValueE(_, span)
            | Self::InitListE(_, span)
            | Self::CompositeE(_, span)
            | Self::FuncCallE(_, span)
            | Self::ConstValueE(_, span)
            | Self::UnaryExprE(_, span) => *span,
        }
    }

    /// Use the originating source range; 0..0 is reserved for synthetic nodes.
    pub fn with_span(mut self, source: Span) -> Self {
        match &mut self {
            Self::Poison(span)
            | Self::VarValueE(_, span)
            | Self::InitListE(_, span)
            | Self::CompositeE(_, span)
            | Self::FuncCallE(_, span)
            | Self::ConstValueE(_, span)
            | Self::UnaryExprE(_, span) => *span = source,
        }
        self
    }
}

impl PartialEq for Expression {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Poison(_), Self::Poison(_)) => true,
            (Self::VarValueE(a, _), Self::VarValueE(b, _)) => a == b,
            (Self::InitListE(a, _), Self::InitListE(b, _)) => a == b,
            (Self::CompositeE(a, _), Self::CompositeE(b, _)) => a == b,
            (Self::FuncCallE(a, _), Self::FuncCallE(b, _)) => a == b,
            (Self::ConstValueE(a, _), Self::ConstValueE(b, _)) => a == b,
            (Self::UnaryExprE(a, _), Self::UnaryExprE(b, _)) => a == b,
            _ => false,
        }
    }
}
impl Eq for Expression {}

impl CompAtom {
    pub fn span(&self) -> Span {
        match self {
            Self::Poison(span)
            | Self::VarValueA(_, span)
            | Self::CompositeA(_, span)
            | Self::FuncCallA(_, span)
            | Self::ConstValueA(_, span)
            | Self::UnaryExprA(_, span) => *span,
        }
    }

    /// Use the originating source range; 0..0 is reserved for synthetic nodes.
    pub fn with_span(mut self, source: Span) -> Self {
        match &mut self {
            Self::Poison(span)
            | Self::VarValueA(_, span)
            | Self::CompositeA(_, span)
            | Self::FuncCallA(_, span)
            | Self::ConstValueA(_, span)
            | Self::UnaryExprA(_, span) => *span = source,
        }
        self
    }
}

impl PartialEq for CompAtom {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Poison(_), Self::Poison(_)) => true,
            (Self::VarValueA(a, _), Self::VarValueA(b, _)) => a == b,
            (Self::CompositeA(a, _), Self::CompositeA(b, _)) => a == b,
            (Self::FuncCallA(a, _), Self::FuncCallA(b, _)) => a == b,
            (Self::ConstValueA(a, _), Self::ConstValueA(b, _)) => a == b,
            (Self::UnaryExprA(a, _), Self::UnaryExprA(b, _)) => a == b,
            _ => false,
        }
    }
}
impl Eq for CompAtom {}

impl Composite {
    pub(crate) fn span(&self) -> Span {
        match (self.members.first(), self.members.last()) {
            (Some(first), Some(last)) => (first.span().start..last.span().end).into(),
            _ => (0..0).into(),
        }
    }
}
