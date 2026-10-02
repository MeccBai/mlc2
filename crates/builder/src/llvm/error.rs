use std::{fmt, io, path::PathBuf};

#[derive(Debug, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Note,
}

#[derive(Debug)]
pub struct DiagnosticLine {
    pub severity: Option<DiagnosticSeverity>,
    pub text: String,
}

impl LinkDiagnostic {
    /// A textual view, not a native lld diagnostic protocol. Continuations are retained.
    pub fn diagnostics(&self) -> Vec<DiagnosticLine> {
        String::from_utf8_lossy(&self.stderr)
            .lines()
            .map(|text| {
                let severity = [
                    ("error:", DiagnosticSeverity::Error),
                    ("warning:", DiagnosticSeverity::Warning),
                    ("note:", DiagnosticSeverity::Note),
                ]
                .into_iter()
                .find_map(|(marker, severity)| {
                    (text.trim_start().starts_with(marker) || text.contains(&format!(": {marker}")))
                        .then_some(severity)
                });
                DiagnosticLine {
                    severity,
                    text: text.into(),
                }
            })
            .collect()
    }
}

#[derive(Debug)]
pub struct LinkDiagnostic {
    pub program: PathBuf,
    pub exit_code: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

#[derive(Debug)]
pub enum BackendError {
    Config(String),
    Target(String),
    /// Generated IR/codegen failure: presented as ICE, but propagated as Result.
    Codegen(String),
    LinkerLaunch {
        program: PathBuf,
        source: io::Error,
    },
    LinkerFailed(LinkDiagnostic),
}

impl fmt::Display for BackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(message) => write!(f, "backend configuration error: {message}"),
            Self::Target(message) => write!(f, "LLVM target error: {message}"),
            Self::Codegen(message) => write!(f, "internal compiler error (LLVM): {message}"),
            Self::LinkerLaunch { program, source } => {
                write!(f, "cannot launch {}: {source}", program.display())
            }
            Self::LinkerFailed(diagnostic) => write!(
                f,
                "{} failed (exit {:?}):\n{}",
                diagnostic.program.display(),
                diagnostic.exit_code,
                String::from_utf8_lossy(&diagnostic.stderr)
            ),
        }
    }
}

impl std::error::Error for BackendError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::LinkerLaunch { source, .. } => Some(source),
            _ => None,
        }
    }
}
