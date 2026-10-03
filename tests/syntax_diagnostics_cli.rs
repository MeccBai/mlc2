use std::process::Command;

#[test]
fn cli_syntax_errors_use_source_tokens_and_keep_snippets_and_count() {
    let root = tempfile::tempdir().unwrap();
    let config = root.path().join("config.toml");
    std::fs::write(&config, "").unwrap();
    for (source, found, expected, internal) in [
        ("enum Color { Red Green }", "Green", ",", "Comma"),
        ("func main() { var a; }", ";", "=", "Semicolon"),
        ("api func main() {}", "api", "module item", "Api"),
    ] {
        let entry = root.path().join("main.m2");
        std::fs::write(&entry, source).unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_mlc"))
            .current_dir(root.path())
            .arg(&entry)
            .arg("--config")
            .arg(&config)
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        let diagnostic = String::from_utf8(output.stderr).unwrap();
        assert!(
            diagnostic.contains(&format!("found `{found}`")),
            "{diagnostic}"
        );
        assert!(diagnostic.contains(expected), "{diagnostic}");
        assert!(!diagnostic.contains(internal), "{diagnostic}");
        assert!(diagnostic.contains(source), "{diagnostic}");
        assert!(diagnostic.contains('^'), "{diagnostic}");
        assert!(diagnostic.contains("1 error generated."), "{diagnostic}");
    }
}
