use super::{Spanned, TempPath};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TempLiteralKind {
    Integer,
    Float,
    String,
    Boolean,
    Null,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TempUnaryOp {
    Negate,
    LogicalNot,
    BitNot,
    AddressOf,
    Dereference,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TempBinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    ShiftLeft,
    ShiftRight,
    BitAnd,
    BitOr,
    BitXor,
    Equal,
    NotEqual,
    Less,
    LessOrEqual,
    Greater,
    GreaterOrEqual,
    LogicalAnd,
    LogicalOr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TempMemberAccess {
    Dot,
    Arrow,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TempExpr {
    Literal {
        kind: TempLiteralKind,
        text: String,
    },
    Path(TempPath),
    Unary {
        op: TempUnaryOp,
        value: Box<Spanned<TempExpr>>,
    },
    Binary {
        lhs: Box<Spanned<TempExpr>>,
        op: TempBinaryOp,
        rhs: Box<Spanned<TempExpr>>,
    },
    Call {
        callee: Box<Spanned<TempExpr>>,
        args: Vec<Spanned<TempExpr>>,
    },
    Member {
        base: Box<Spanned<TempExpr>>,
        access: TempMemberAccess,
        name: String,
    },
    Init {
        target: Option<Box<Spanned<TempExpr>>>,
        values: Vec<Spanned<TempExpr>>,
    },
    Array(Vec<Spanned<TempExpr>>),
}
