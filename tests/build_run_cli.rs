#![cfg(windows)]
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

fn invoke(root: &Path, arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_mlc"))
        .current_dir(root)
        .args(arguments)
        .args(["--config", "config.toml"])
        .output()
        .unwrap()
}

#[test]
fn run_requires_target_and_legacy_entry_is_rejected() {
    let root = tempfile::tempdir().unwrap();
    for args in [vec!["run"], vec!["main.m2"], vec![]] {
        assert!(!invoke(root.path(), &args).status.success());
    }
}

#[test]
fn standalone_source_ignores_project_and_run_rejects_libraries() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("Project.toml"), "invalid [").unwrap();
    fs::write(root.path().join("main.m2"), "func main(){ return 1; }").unwrap();
    let output = invoke(root.path(), &["build", "main.m2"]);
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("Cannot return a value"), "{error}");
    let output = invoke(root.path(), &["run", "main.m2", "--type", "shared"]);
    assert!(String::from_utf8_lossy(&output.stderr).contains("requires a bin target"));
}

#[test]
#[ignore = "Requires bundled ld.lld.exe and runtime beside the test compiler (MLC installation)"]
fn build_and_run_sources_named_targets_and_all_targets() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("main.m2"),
        "func main()->i32 { return 7; }",
    )
    .unwrap();
    fs::write(
        root.path().join("other.m2"),
        "func main()->i32 { return 0; }",
    )
    .unwrap();
    fs::write(root.path().join("project.toml"), "[Project]\nName='demo'\nVersion='1'\nOutputDir='project-out'\n[[Project.Targets]]\nName='main'\nEntry='main.m2'\nType='bin'\n[[Project.Targets]]\nName='other'\nEntry='other.m2'\nType='bin'\n").unwrap();
    for args in [
        vec!["build", "main.m2"],
        vec!["build", "main"],
        vec!["build"],
    ] {
        let result = invoke(root.path(), &args);
        assert!(result.status.success(), "{result:?}");
    }
    assert!(root.path().join("build/main.exe").is_file());
    assert!(root.path().join("project-out/main.exe").is_file());
    assert!(root.path().join("project-out/other.exe").is_file());
    for target in ["main.m2", "main"] {
        let result = invoke(root.path(), &["run", target]);
        assert_eq!(result.status.code(), Some(7), "{result:?}");
    }
    let result = invoke(root.path(), &["run", "missing"]);
    assert!(!result.status.success());
    fs::write(root.path().join("project.toml"), "broken [").unwrap();
    assert_eq!(
        invoke(root.path(), &["run", "main.m2"]).status.code(),
        Some(7)
    );
    // A failed rebuild must not execute the existing program returning 7.
    fs::write(
        root.path().join("main.m2"),
        "func main()->i32 { return missing; }",
    )
    .unwrap();
    let result = invoke(root.path(), &["run", "main.m2"]);
    assert_eq!(result.status.code(), Some(1));
    assert!(!result.stderr.is_empty());
}
