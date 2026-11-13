use super::*;
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
        mlc_manifest::Target::default().triplet(),
        mlc_manifest::Target::default()
            .with_system(mlc_manifest::System::Windows)
            .with_arch(mlc_manifest::Arch::X86_64)
            .with_abi(mlc_manifest::Abi::Gnu)
            .triplet(),
        mlc_manifest::Target::default()
            .with_system(mlc_manifest::System::Linux)
            .with_arch(mlc_manifest::Arch::X86_64)
            .triplet(),
        mlc_manifest::Target::default()
            .with_system(mlc_manifest::System::Linux)
            .with_arch(mlc_manifest::Arch::Aarch64)
            .triplet(),
        mlc_manifest::Target::default()
            .with_arch(mlc_manifest::Arch::X86_64)
            .with_system(mlc_manifest::System::Macos)
            .triplet(),
        mlc_manifest::Target::default()
            .with_arch(mlc_manifest::Arch::Aarch64)
            .with_system(mlc_manifest::System::Macos)
            .triplet(),
    ] {
        for _ in 0..2 {
            IrCompiler::init(triplet).unwrap();
        }
    }
}

#[test]
fn linux_and_macos_targets_emit_objects() {
    for triplet in [
        mlc_manifest::Target::default()
            .with_system(mlc_manifest::System::Linux)
            .with_arch(mlc_manifest::Arch::X86_64)
            .triplet(),
        mlc_manifest::Target::default()
            .with_system(mlc_manifest::System::Linux)
            .with_arch(mlc_manifest::Arch::Aarch64)
            .triplet(),
        mlc_manifest::Target::default()
            .with_arch(mlc_manifest::Arch::X86_64)
            .with_system(mlc_manifest::System::Macos)
            .triplet(),
        mlc_manifest::Target::default()
            .with_arch(mlc_manifest::Arch::Aarch64)
            .with_system(mlc_manifest::System::Macos)
            .triplet(),
    ] {
        let root = tempfile::tempdir().unwrap();
        let output = root.path().join("sample.o");
        IrCompiler::init(triplet)
            .unwrap()
            .emit("define i32 @sample() { ret i32 42 }".into(), &output)
            .unwrap();
        assert!(std::fs::metadata(output).unwrap().len() > 0);
    }
}

#[test]
fn verifier_failure_is_an_ice_result_not_a_panic() {
    let compiler = IrCompiler::init(mlc_manifest::Target::default().triplet()).unwrap();
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
    let compiler = IrCompiler::init(mlc_manifest::Target::default().triplet()).unwrap();
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
        program: mlc_manifest::Target::default().linker().into(),
        exit_code: Some(1),
        stdout: vec![],
        stderr: format!(
            "{}: error: undefined symbol\n>>> referenced by a.obj\nwarning: detail\n",
            mlc_manifest::Target::default().linker()
        )
        .into_bytes(),
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
        link::completion(Path::new(mlc_manifest::Target::default().linker()), output)
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
            Path::new(mlc_manifest::Target::default().linker()),
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
        .join(format!(
            "{}{}",
            mlc_manifest::Target::default().linker(),
            mlc_manifest::EXE_SUFFIX
        ));
    assert!(
        matches!(link::link(&program, &[]), Err(BackendError::LinkerLaunch { source, .. }) if source.kind() == std::io::ErrorKind::NotFound)
    );
}
