use super::{Spanned, TempPath};
use crate::ast::expr::operators::Operator;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TempLiteralKind {
    Integer,
    Float,
    String,
    Boolean,
    Null,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TempExpr {
    Literal {
        kind: TempLiteralKind,
        text: String,
    },
    Path(TempPath),
    Group(Box<Spanned<TempExpr>>),
    Unary {
        op: Operator,
        value: Box<Spanned<TempExpr>>,
    },
    Binary {
        operands: Vec<Spanned<TempExpr>>,
        operators: Vec<Operator>,
    },
    Call {
        callee: Box<Spanned<TempExpr>>,
        args: Vec<Spanned<TempExpr>>,
    },
    Member {
        base: Box<Spanned<TempExpr>>,
        indirect: bool,
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
            Self::Group(inner) => format!("({})", inner.0.dump()),
            Self::Unary { op, value } => format!("{op:?}({})", value.0.dump()),
            Self::Binary {
                operands,
                operators,
            } => {
                if operators == &[Operator::Index] && operands.len() == 2 {
                    return format!("{}[{}]", operands[0].0.dump(), operands[1].0.dump());
                }
                let mut parts = operands.iter().map(|(expr, _)| expr.dump());
                let first = parts.next().unwrap_or_default();
                let rest = operators
                    .iter()
                    .zip(parts)
                    .map(|(op, expr)| format!(" {op:?} {expr}"))
                    .collect::<String>();
                format!("({first}{rest})")
            }
            Self::Call { callee, args } => format!(
                "{}({})",
                callee.0.dump(),
                args.iter()
                    .map(|(arg, _)| arg.dump())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Self::Member {
                base,
                indirect,
                name,
            } => {
                let access = if *indirect { "->" } else { "." };
                format!("{}{access}{name}", base.0.dump())
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
