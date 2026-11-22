use super::{Spanned, TempExpr, TempVar};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", content = "data")]
pub enum TempMatchPattern {
    Default,
    Expression(Spanned<TempExpr>),
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TempScope {
    pub statements: Vec<Spanned<TempStmt>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TempForBinding {
    Clone,
    Ref,
    MutRef,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", content = "data")]
pub enum TempStmt {
    MacroFor {
        binding: Spanned<String>,
        pack: Spanned<String>,
        scope: TempScope,
    },
    VariantMatch {
        binding: Spanned<String>,
        value: Spanned<TempExpr>,
        branches: Vec<(Option<Spanned<super::TempType>>, TempScope)>,
    },
    Variable(TempVar),
    CompoundAssignment {
        target: Spanned<TempExpr>,
        op: crate::operators::Operator,
        value: Spanned<TempExpr>,
    },
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
        mode: TempForBinding,
        iterator: Spanned<TempExpr>,
        scope: TempScope,
    },
    Anonymous(TempScope),
    Continue,
    Break,
}
