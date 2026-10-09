use std::{fs, process::Command};

#[test]
fn converts_handwritten_c_declarations_without_building() {
    let root = tempfile::tempdir().unwrap();
    let input = root.path().join("io.toml");
    fs::write(&input, include_str!("../lib/universal/c_std/io.toml")).unwrap();
    let run = |extra: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_mlc"))
            .current_dir(root.path())
            .args(["symbols", "io.toml"])
            .args(extra)
            .output()
            .unwrap()
    };
    let output = run(&["--config", "nonexistent.toml"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let artifact = mlc_builder::artifacts::load(&root.path().join("io.sym")).unwrap();
    let functions = &artifact.manifest.symbols.functions;
    assert_eq!(
        functions
            .iter()
            .map(|function| function.name.as_str())
            .collect::<Vec<_>>(),
        ["printf", "scanf", "puts", "putchar", "getchar"]
    );
    assert_eq!(functions[0].params.len(), 2);
    assert_eq!(functions[1].params.len(), 2);
    assert_eq!(functions[1].params[1].name, "...");
    assert!(functions[1].params[1].ty.is_none());
    assert_eq!(functions[2].params.len(), 1);
    assert_eq!(functions[3].params.len(), 1);
    assert!(functions[4].params.is_empty());
    assert!(!root.path().join("build").exists());
    let output = run(&["-o", "nested/custom.sym"]);
    assert!(output.status.success());
    assert!(root.path().join("nested/custom.sym").is_file());
    let output = run(&["-o", "wrong.toml"]);
    assert!(!output.status.success());
    assert!(!root.path().join("wrong.toml").exists());
    fs::write(&input, "invalid = [[[ ").unwrap();
    let output = run(&["-o", "broken.sym"]);
    assert!(!output.status.success());
    assert!(!root.path().join("broken.sym").exists());
}
