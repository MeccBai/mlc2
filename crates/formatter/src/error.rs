use std::{fmt, ops::Range};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatIssue {
    pub span: Range<usize>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormatError {
    Lexical(FormatIssue),
    Syntax(Vec<FormatIssue>),
    /// An internal formatting invariant failed; the caller must not write output.
    InvalidOutput,
}

impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Lexical(issue) => write!(f, "{} at {:?}", issue.message, issue.span),
            Self::Syntax(issues) => {
                for (index, issue) in issues.iter().enumerate() {
                    if index > 0 {
                        writeln!(f)?;
                    }
                    write!(f, "{} at {:?}", issue.message, issue.span)?;
                }
                if issues.is_empty() {
                    write!(f, "Parser returned no syntax tree")?;
                }
                Ok(())
            }
            Self::InvalidOutput => write!(f, "Formatter changed source tokens or comments"),
        }
    }
}

impl std::error::Error for FormatError {}
