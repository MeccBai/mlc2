use std::{fs, process::Command};

#[test]
fn ll_exports_source_modules_and_generic_instances_without_linking() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("config.toml"), "").unwrap();
    fs::write(
        root.path().join("leaf.m2"),
        "export func<T> id(x:T) -> T { return x; }",
    )
    .unwrap();
    fs::write(
        root.path().join("main.m2"),
        "import leaf; func main() -> i32 { return leaf::id(1); }",
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_mlc"))
        .current_dir(root.path())
        .args([
            "ll",
            "main.m2",
            "--config",
            "config.toml",
            "-o",
            "ir",
            "--linker",
            "missing-linker",
            "--archiver",
            "missing-archiver",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        fs::read_to_string(root.path().join("ir/main.ll"))
            .unwrap()
            .contains("define")
    );
    assert!(root.path().join("ir/leaf.ll").is_file());
    assert!(
        fs::read_to_string(root.path().join("ir/__mlc_instances.ll"))
            .unwrap()
            .contains("leaf::id<i32>")
    );
    assert!(!root.path().join("ir/objects").exists());
    assert!(!root.path().join("build").exists());
}

#[test]
fn ll_defaults_to_build_ll_and_emits_nothing_on_frontend_failure() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("config.toml"), "").unwrap();
    fs::write(
        root.path().join("main.m2"),
        "func main() -> i32 { return 0; }",
    )
    .unwrap();
    let run = |extra: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_mlc"))
            .current_dir(root.path())
            .args(["ll", "main.m2", "--config", "config.toml"])
            .args(extra)
            .output()
            .unwrap()
    };
    let output = run(&[]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(root.path().join("build/ll/main.ll").is_file());
    fs::write(root.path().join("main.m2"), "func main() { missing(); }").unwrap();
    assert!(!run(&["-o", "invalid-ir"]).status.success());
    assert!(!root.path().join("invalid-ir").exists());
}
