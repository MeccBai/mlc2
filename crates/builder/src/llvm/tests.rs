use super::*;
use crate::manifest::{
    AARCH64_NONE_ELF, RISCV32_NONE_ELF, THUMBV7EM_NONE_EABI, X86_64_PC_WINDOWS_MSVC,
};
#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;
#[cfg(windows)]
use std::os::windows::process::ExitStatusExt;
use std::{
    path::Path,
    process::{ExitStatus, Output},
};

#[test]
fn configuration_uses_executable_siblings() {
    let root = std::env::temp_dir().join("mlc2-layout");
    let config = BackendConfig::from_executable(root.join("mlc.exe")).unwrap();
    assert_eq!(config.lib, root.join("lib"));
    assert_eq!(config.tools, root.join("tools"));
    assert!(BackendConfig::from_executable("mlc.exe".into()).is_err());
}

#[test]
fn unsupported_triplet_returns_an_error() {
    assert!(matches!(
        IrCompiler::init("unsupported-target"),
        Err(BackendError::Target(_))
    ));
}

#[test]
fn preset_targets_initialize_repeatedly() {
    for triplet in [
        X86_64_PC_WINDOWS_MSVC,
        THUMBV7EM_NONE_EABI,
        AARCH64_NONE_ELF,
        RISCV32_NONE_ELF,
    ] {
        for _ in 0..2 {
            IrCompiler::init(triplet).unwrap();
        }
    }
}

#[test]
fn verifier_failure_is_an_ice_result_not_a_panic() {
    let compiler = IrCompiler::init(X86_64_PC_WINDOWS_MSVC).unwrap();
    let error = compiler
        .emit(
            "define i32 @main() { entry: br label %entry }".into(),
            Path::new("unused.obj"),
        )
        .unwrap_err();
    assert!(matches!(error, BackendError::Codegen(_)));
    assert!(error.to_string().contains("internal compiler error"));
}

#[test]
fn invalid_ir_is_recoverable_and_next_module_can_emit() {
    let compiler = IrCompiler::init(X86_64_PC_WINDOWS_MSVC).unwrap();
    let output = std::env::temp_dir().join(format!("mlc2-backend-{}.obj", std::process::id()));
    assert!(matches!(
        compiler.emit("define broken".into(), &output),
        Err(BackendError::Codegen(_))
    ));
    compiler
        .emit("define i32 @main() { ret i32 0 }".into(), &output)
        .unwrap();
    assert!(std::fs::metadata(&output).unwrap().len() > 0);
    std::fs::remove_file(output).unwrap();
}

#[test]
fn diagnostic_text_keeps_continuations() {
    let diagnostic = LinkDiagnostic {
        program: "lld-link".into(),
        exit_code: Some(1),
        stdout: vec![],
        stderr: b"lld-link: error: undefined symbol\n>>> referenced by a.obj\nwarning: detail\n"
            .to_vec(),
    };
    let lines = diagnostic.diagnostics();
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0].severity, Some(DiagnosticSeverity::Error));
    assert_eq!(lines[1].severity, None);
    assert_eq!(lines[2].severity, Some(DiagnosticSeverity::Warning));
}

#[test]
fn linker_failure_preserves_status_and_both_raw_streams() {
    let output = Output {
        status: ExitStatus::from_raw(1),
        stdout: b"stdout\xff".to_vec(),
        stderr: b"lld: undefined symbol: missing\n".to_vec(),
    };
    let Err(BackendError::LinkerFailed(diagnostic)) =
        link::completion(Path::new("lld-link"), output)
    else {
        panic!("link error expected")
    };
    assert!(diagnostic.exit_code.is_some());
    assert_eq!(diagnostic.stdout, b"stdout\xff");
    assert_eq!(diagnostic.stderr, b"lld: undefined symbol: missing\n");
}

#[test]
fn successful_linker_completion_is_not_an_error_even_with_warnings() {
    assert!(
        link::completion(
            Path::new("lld-link"),
            Output {
                status: ExitStatus::from_raw(0),
                stdout: Vec::new(),
                stderr: b"warning\n".to_vec()
            }
        )
        .is_ok()
    );
}

#[test]
fn linker_launch_error_preserves_io_cause() {
    let program = std::env::temp_dir()
        .join("mlc2-nonexistent-linker")
        .join("lld-link.exe");
    assert!(
        matches!(link::link(&program, &[]), Err(BackendError::LinkerLaunch { source, .. }) if source.kind() == std::io::ErrorKind::NotFound)
    );
}
