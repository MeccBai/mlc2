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

impl TempExpr {
    pub fn dump(&self) -> String {
        match self {
            Self::Literal { text, .. } => text.clone(),
            Self::Path(path) => path.segments.join("::"),
            Self::Unary { op, value } => format!("{op:?}({})", value.0.dump()),
            Self::Binary { lhs, op, rhs } => {
                format!("({} {op:?} {})", lhs.0.dump(), rhs.0.dump())
            }
            Self::Call { callee, args } => format!(
                "{}({})",
                callee.0.dump(),
                args.iter()
                    .map(|(arg, _)| arg.dump())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Self::Member { base, access, name } => {
                format!("{} {access:?} {name}", base.0.dump())
            }
            Self::Init { target, values } => format!(
                "{}{{{}}}",
                target
                    .as_ref()
                    .map(|target| target.0.dump())
                    .unwrap_or_default(),
                values
                    .iter()
                    .map(|(value, _)| value.dump())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Self::Array(values) => format!(
                "[{}]",
                values
                    .iter()
                    .map(|(value, _)| value.dump())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }
}
