use std::collections::HashSet;

use crate::lexer::Span;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Warning {
    UnusedVariable { name: String },
    AttributeNotFound,
    ArrayInitializerZeroFilled { supplied: usize, length: usize },
}

impl std::fmt::Display for Warning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnusedVariable { name } => write!(f, "Variable `{name}` is never used"),
            Self::AttributeNotFound => write!(f, "Unknown attribute"),
            Self::ArrayInitializerZeroFilled { supplied, length } => write!(
                f,
                "Array initializer supplies {supplied} of {length} elements; remaining elements are zero-initialized"
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WarningInfo {
    pub warning: Warning,
    pub span: Span,
}

impl WarningInfo {
    pub fn new(warning: Warning, span: Span) -> Self {
        Self {
            warning: warning,
            span,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WarningHandle {
    pub file: String,
    pub warnings: HashSet<WarningInfo>,
}

impl WarningHandle {
    pub fn new(file: String) -> Self {
        Self {
            file,
            warnings: HashSet::new(),
        }
    }

    pub fn submit_warning(&mut self, warning: Warning, span: Span) {
        self.warnings.insert(WarningInfo::new(warning, span));
    }
}
