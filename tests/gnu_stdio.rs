//! Verify that LLVM-MinGW/UCRT supplies externally callable scanf.
#![cfg(windows)]

use mlc_builder::{
    llvm::IrCompiler,
    manifest::{DEFAULT_WINDOWS_LINKER, X86_64_PC_WINDOWS_GNU},
};
use std::{
    io::Write,
    process::{Command, Stdio},
};

#[test]
fn gnu_scanf_links_and_reads_stdin() {
    let root = tempfile::tempdir().unwrap();
    let object = root.path().join("scanf.obj");
    let executable = root.path().join("scanf.exe");
    let ir = r#"
@format = private constant [3 x i8] c"%d\00"
declare i32 @scanf(ptr, ...)
define i32 @main() {
entry:
  %value = alloca i32
  store i32 0, ptr %value
  %count = call i32 (ptr, ...) @scanf(ptr @format, ptr %value)
  %read = load i32, ptr %value
  %correct = icmp eq i32 %read, 42
  %success = icmp eq i32 %count, 1
  %ok = and i1 %correct, %success
  %exit = select i1 %ok, i32 0, i32 1
  ret i32 %exit
}
"#;
    IrCompiler::init(X86_64_PC_WINDOWS_GNU)
        .unwrap()
        .emit(ir.into(), &object)
        .unwrap();
    let clang = std::env::var_os("MLC_TEST_CLANG").unwrap_or_else(|| DEFAULT_WINDOWS_LINKER.into());
    let link = Command::new(clang)
        .args(["--target=x86_64-pc-windows-gnu", "-fuse-ld=lld"])
        .arg(&object)
        .arg("-o")
        .arg(&executable)
        .output()
        .unwrap();
    assert!(
        link.status.success(),
        "{}",
        String::from_utf8_lossy(&link.stderr)
    );
    let mut child = Command::new(executable)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(b"42\n").unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
