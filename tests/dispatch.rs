//! The router and formatter must work without the compiler worker or LLVM DLL.
use std::{fs, path::Path, process::Command};

fn router(root: &Path) -> std::path::PathBuf {
    let path = root.join(format!("mlc{}", std::env::consts::EXE_SUFFIX));
    fs::copy(env!("CARGO_BIN_EXE_mlc"), &path).unwrap();
    path
}

#[test]
fn version_and_help_do_not_require_workers() {
    let root = tempfile::tempdir().unwrap();
    let executable = router(root.path());
    for flag in ["--version", "--help"] {
        let result = Command::new(&executable)
            .current_dir(root.path())
            .env("PATH", "")
            .arg(flag)
            .output()
            .unwrap();
        assert!(result.status.success(), "{result:?}");
    }
    let result = Command::new(&executable)
        .current_dir(root.path())
        .args(["build", "missing.m2"])
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("Command worker not found"));
}

#[test]
fn formatting_works_without_llvm_or_compiler_worker() {
    let root = tempfile::tempdir().unwrap();
    let executable = router(root.path());
    fs::copy(
        env!("CARGO_BIN_EXE_mlc-fmt"),
        root.path()
            .join(format!("mlc-fmt{}", std::env::consts::EXE_SUFFIX)),
    )
    .unwrap();
    let source = root.path().join("main.m2");
    fs::write(&source, "func main(){}").unwrap();
    let result = Command::new(&executable)
        .current_dir(root.path())
        .env("PATH", "")
        .args(["fmt", "main.m2"])
        .output()
        .unwrap();
    assert!(result.status.success(), "{result:?}");
    assert_eq!(fs::read_to_string(source).unwrap(), "func main() {\n}\n");
}
