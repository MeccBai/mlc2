use std::process::Command;

#[test]
fn example_commands_need_no_project_or_config_and_print_only_text() {
    let root = tempfile::tempdir().unwrap();
    let invalid_config = root.path().join("config.toml");
    std::fs::write(&invalid_config, "not valid toml [[[!").unwrap();
    let run = |name: &str| {
        Command::new(env!("CARGO_BIN_EXE_mlc"))
            .current_dir(root.path())
            .args(["example", name, "--config"])
            .arg(&invalid_config)
            .output()
            .unwrap()
    };
    let listing = run("list");
    assert!(
        listing.status.success(),
        "{}",
        String::from_utf8_lossy(&listing.stderr)
    );
    assert!(listing.stderr.is_empty());
    assert_eq!(
        String::from_utf8(listing.stdout).unwrap(),
        mlc_examples::render("list").unwrap()
    );
    for example in mlc_examples::EXAMPLES {
        let output = run(example.name);
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        assert_eq!(output.stdout, example.source.as_bytes());
    }
    let unknown = run("unknown");
    assert!(!unknown.status.success());
    assert!(unknown.stdout.is_empty());
    assert!(
        String::from_utf8(unknown.stderr)
            .unwrap()
            .contains("mlc example list")
    );
    assert!(!root.path().join("build").exists());
}
