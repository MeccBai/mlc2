use super::{BackendError, LinkDiagnostic};
use std::{
    ffi::OsString,
    path::Path,
    process::{Command, Output},
};

/// Invoke the caller-selected lld driver (lld-link, ld.lld, or wasm-ld), without a shell.
/// llvm-sys does not expose lld's C++ driver; preserve diagnostics rather than guessing categories.
pub fn link(program: &Path, arguments: &[OsString]) -> Result<LinkDiagnostic, BackendError> {
    let output = Command::new(program)
        .args(arguments)
        .output()
        .map_err(|source| BackendError::LinkerLaunch {
            program: program.into(),
            source,
        })?;
    completion(program, output)
}

pub(super) fn completion(program: &Path, output: Output) -> Result<LinkDiagnostic, BackendError> {
    let success = output.status.success();
    let diagnostic = LinkDiagnostic {
        program: program.into(),
        exit_code: output.status.code(),
        stdout: output.stdout,
        stderr: output.stderr,
    };
    if success {
        Ok(diagnostic)
    } else {
        Err(BackendError::LinkerFailed(diagnostic))
    }
}
