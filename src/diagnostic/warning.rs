use std::collections::HashSet;

use crate::lexer::Span;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Warning {
    AttributeNotFound,
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
