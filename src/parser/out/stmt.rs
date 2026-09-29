use super::{Spanned, TempExpr, TempVar};

#[derive(Debug, Clone, PartialEq)]
pub enum TempMatchPattern {
    Default,
    Expression(Spanned<TempExpr>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct TempScope {
    pub statements: Vec<Spanned<TempStmt>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TempStmt {
    Variable(TempVar),
    Assignment {
        target: Spanned<TempExpr>,
        value: Spanned<TempExpr>,
    },
    Expression(Spanned<TempExpr>),
    Return(Option<Spanned<TempExpr>>),
    If {
        condition: Spanned<TempExpr>,
        then_scope: TempScope,
        else_scope: Option<TempScope>,
    },
    While {
        condition: Spanned<TempExpr>,
        scope: TempScope,
    },
    Match {
        value: Spanned<TempExpr>,
        branches: Vec<(TempMatchPattern, TempScope)>,
    },
    For {
        binding: Spanned<String>,
        /// Inclusive lower bound of the `[start, end)` iteration range.
        start: Spanned<TempExpr>,
        /// Exclusive upper bound of the `[start, end)` iteration range.
        end: Spanned<TempExpr>,
        scope: TempScope,
    },
    Anonymous(TempScope),
    Continue,
    Break,
}
