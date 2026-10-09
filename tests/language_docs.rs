use std::{fs, process::Command};

/// Every standalone language example must still pass the real CLI frontend
/// and LLVM text generation, including its imported declaration modules.
#[test]
fn standalone_language_documentation_examples_compile() {
    let document = include_str!("../docs/language.md");
    let lib = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("lib");
    let mut example = None;
    let mut examples = Vec::new();
    for line in document.lines() {
        if line == "```rust" {
            example = Some(String::new());
        } else if line == "```" {
            if let Some(source) = example.take() {
                examples.push(source);
            }
        } else if let Some(source) = &mut example {
            source.push_str(line);
            source.push('\n');
        }
    }
    assert!(examples.len() >= 20, "language reference lost its examples");
    for (index, source) in examples.iter().enumerate() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("main.m2"), source).unwrap();
        let result = Command::new(env!("CARGO_BIN_EXE_mlc"))
            .current_dir(root.path())
            .args(["ll", "main.m2", "--lib-dir"])
            .arg(&lib)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "Example {}:\n{source}\n{}",
            index + 1,
            String::from_utf8_lossy(&result.stderr)
        );
    }
}
