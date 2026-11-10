//! Diagnostics are data; rendering belongs to the command's output boundary.
use std::{ops::Range, sync::Arc};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}
impl Severity {
    pub fn name(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
        }
    }
}
#[derive(Debug, Clone)]
pub struct Source {
    pub file: String,
    pub text: Arc<str>,
    pub span: Range<usize>,
}
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    pub source: Option<Source>,
}
impl Diagnostic {
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            message: message.into(),
            source: None,
        }
    }
    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            message: message.into(),
            source: None,
        }
    }
    pub fn at(
        mut self,
        file: impl Into<String>,
        text: impl Into<Arc<str>>,
        span: Range<usize>,
    ) -> Self {
        self.source = Some(Source {
            file: file.into(),
            text: text.into(),
            span,
        });
        self
    }
    pub fn position(&self) -> Option<(usize, usize)> {
        let source = self.source.as_ref()?;
        let mut start = source.span.start.min(source.text.len());
        while !source.text.is_char_boundary(start) {
            start -= 1;
        }
        let prefix = &source.text[..start];
        Some((
            prefix.bytes().filter(|b| *b == b'\n').count() + 1,
            prefix[prefix.rfind('\n').map_or(0, |n| n + 1)..]
                .chars()
                .count()
                + 1,
        ))
    }
    pub fn render(&self) -> String {
        if let Some(source) = &self.source {
            match self.severity {
                Severity::Error => super::error::render_diagnostic(
                    &source.file,
                    &source.text,
                    source.span.clone(),
                    &self.message,
                ),
                Severity::Warning => super::error::render_warning(
                    &source.file,
                    &source.text,
                    source.span.clone(),
                    &self.message,
                ),
            }
        } else {
            use colored::Colorize;
            let label = match self.severity {
                Severity::Error => "error".red().bold(),
                Severity::Warning => "warning".yellow().bold(),
            };
            format!("{label}: {}", self.message)
        }
    }
}
#[derive(Debug, Clone)]
pub struct Report {
    pub diagnostics: Vec<Diagnostic>,
}
impl Report {
    pub fn new(diagnostics: Vec<Diagnostic>) -> Self {
        Self { diagnostics }
    }
    pub fn contains(&self, text: &str) -> bool {
        self.diagnostics.iter().any(|d| d.message.contains(text))
    }
}
impl From<String> for Report {
    fn from(message: String) -> Self {
        Self::new(vec![Diagnostic::error(message)])
    }
}
impl From<&str> for Report {
    fn from(message: &str) -> Self {
        message.to_owned().into()
    }
}
impl std::fmt::Display for Report {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, d) in self.diagnostics.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            write!(f, "{}", d.message)?;
        }
        Ok(())
    }
}
impl std::error::Error for Report {}
