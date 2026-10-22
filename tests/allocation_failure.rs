#![cfg(windows)]

mod support;
use mlc_builder::{
    llvm::IrCompiler,
    manifest::{DEFAULT_WINDOWS_LINKER, X86_64_PC_WINDOWS_GNU},
};
use std::{fs, path::Path, process::Command};

#[test]
fn allocation_failure_aborts_without_resource_cleanup() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("main.m2"),
        "import std::mem; func main() -> i32 { \
         var first:res = std::mem::alloc<i32>(1); \
         var second:res = std::mem::alloc<i32>(1); return 0; }",
    )
    .unwrap();
    let compile = Command::new(env!("CARGO_BIN_EXE_mlc"))
        .current_dir(root.path())
        .args(["ll", "main.m2", "--lib-dir"])
        .arg(support::library())
        .output()
        .unwrap();
    assert!(
        compile.status.success(),
        "{}",
        String::from_utf8_lossy(&compile.stderr)
    );

    let compiler = IrCompiler::init(X86_64_PC_WINDOWS_GNU).unwrap();
    let mut objects = Vec::new();
    for file in fs::read_dir(
        root.path()
            .join("build/ll/default")
            .join(mlc_builder::paths::HOST_TRIPLET),
    )
    .unwrap()
    {
        let path = file.unwrap().path();
        if !path.extension().is_some_and(|extension| extension == "ll") {
            continue;
        }
        let mut ir = fs::read_to_string(&path).unwrap();
        for name in ["malloc", "free", "abort"] {
            ir = ir
                .replace(&format!("@\"{name}\""), &format!("@\"test_{name}\""))
                .replace(&format!("@{name}("), &format!("@test_{name}("));
        }
        let object = path.with_extension("obj");
        compiler.emit(ir, &object).unwrap();
        objects.push(object);
    }
    let executable = root.path().join("failure.exe");
    let link = Command::new(DEFAULT_WINDOWS_LINKER)
        .arg("-fuse-ld=lld")
        .args(objects)
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/allocation_failure.c"))
        .arg("-o")
        .arg(&executable)
        .output()
        .unwrap();
    assert!(
        link.status.success(),
        "{}",
        String::from_utf8_lossy(&link.stderr)
    );
    // The first allocation succeeds; the second returns null. Cleanup is forbidden.
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(73));
}
