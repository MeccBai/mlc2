//! Exercise real MSVC ABI objects, CRT startup and standard-library calls.
#![cfg(windows)]
mod support;
use std::{fs, process::Command};

#[test]
#[ignore = "Requires bundled tools/lld-link.exe and MSVC/UCRT libraries"]
fn all_examples_link_and_run_with_msvc_ucrt() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("config.toml"), "").unwrap();
    for example in mlc_examples::EXAMPLES {
        let input = format!("{}.m2", example.name);
        fs::write(root.path().join(&input), example.source).unwrap();
        let result = Command::new(env!("CARGO_BIN_EXE_mlc"))
            .current_dir(root.path())
            .args([
                "build",
                &input,
                "--triplet",
                "x86_64-pc-windows-msvc",
                "--config",
                "config.toml",
                "--lib-dir",
            ])
            .arg(support::library())
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}: {}",
            example.name,
            String::from_utf8_lossy(&result.stderr)
        );
        let executed = Command::new(root.path().join("build").join(format!("{}.exe", example.name))).output().unwrap();
        assert!(executed.status.success(), "{}", example.name);
        assert_eq!(
            String::from_utf8(executed.stdout)
                .unwrap()
                .replace("\r\n", "\n"),
            example.expected_stdout,
            "{}",
            example.name
        );
    }
}
