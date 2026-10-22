#![cfg(windows)]
use std::{fs, path::Path, process::Command};
fn invoke(root: &Path, args: &[&str]) {
    let result = Command::new(env!("CARGO_BIN_EXE_mlc"))
        .current_dir(root)
        .args(args)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}
#[test]
#[ignore = "Requires bundled linker and MSVC/UCRT libraries"]
fn modes_isolate_outputs_and_share_only_the_selected_std_variant() {
    let root = tempfile::tempdir().unwrap();
    let triplet = "x86_64-pc-windows-msvc";
    fs::create_dir_all(root.path().join("lib").join(triplet)).unwrap();
    fs::write(
        root.path().join("lib").join(triplet).join("std.m2"),
        "export func value()->i32 {var x=1+2;return x;}",
    )
    .unwrap();
    fs::write(
        root.path().join("main.m2"),
        "import std;func main()->i32{return std::value()-3;}",
    )
    .unwrap();
    fs::write(root.path().join("config.toml"), "").unwrap();
    let project = "[Project]\nName='layout'\nVersion='1'\nOutputDir='build'\nLibDirs=['lib']\nLibCacheDir='cache'\n[[Project.Targets]]\nName='app'\nEntry='main.m2'\nType='bin'\n";
    fs::write(root.path().join("Project.toml"), project).unwrap();
    let mut stamps = std::collections::HashMap::new();
    for (mode, std_mode) in [
        ("default", "debug"),
        ("debug", "debug"),
        ("release", "release"),
        ("rel-with-info", "release"),
    ] {
        invoke(
            root.path(),
            &[
                "build",
                "app",
                &format!("--{mode}"),
                "--config",
                "config.toml",
            ],
        );
        let exe = root.path().join("build").join(mode).join("app.exe");
        assert!(Command::new(&exe).status().unwrap().success());
        assert!(
            root.path()
                .join("build/objects")
                .join(mode)
                .join(triplet)
                .join("main.obj")
                .is_file()
        );
        let object = root
            .path()
            .join("cache")
            .join(std_mode)
            .join(triplet)
            .join("std.obj");
        let bytes = fs::read(&object).unwrap();
        assert_eq!(
            bytes.windows(8).any(|bytes| bytes == b".debug$S"),
            std_mode == "debug"
        );
        let stamp = fs::metadata(&object).unwrap().modified().unwrap();
        if let Some(previous) = stamps.insert(std_mode, stamp) {
            assert_eq!(previous, stamp, "std {std_mode} should be reused by {mode}");
        }
        invoke(
            root.path(),
            &[
                "ll",
                "main.m2",
                &format!("--{mode}"),
                "--config",
                "config.toml",
                "--lib-dir",
                "lib",
            ],
        );
        assert!(
            root.path()
                .join("build/ll")
                .join(mode)
                .join(triplet)
                .join("main.ll")
                .is_file()
        );
        assert!(
            root.path()
                .join("build/ll")
                .join(std_mode)
                .join(triplet)
                .join("std.ll")
                .is_file()
        );
    }
    // A project-level override selects release std even for a debug consumer.
    let configured = project.replace(
        "LibCacheDir='cache'",
        "LibCacheDir='cache'\nstd_type='release'",
    );
    fs::write(root.path().join("Project.toml"), &configured).unwrap();
    invoke(
        root.path(),
        &["build", "app", "--debug", "--config", "config.toml"],
    );
    let stamp = fs::metadata(
        root.path()
            .join("cache/release")
            .join(triplet)
            .join("std.obj"),
    )
    .unwrap()
    .modified()
    .unwrap();
    assert_eq!(stamp, stamps["release"]);
    // Target-level overrides win, and only debug/release are accepted for std.
    fs::write(
        root.path().join("Project.toml"),
        format!("{configured}std_type='debug'\n"),
    )
    .unwrap();
    invoke(
        root.path(),
        &["build", "app", "--release", "--config", "config.toml"],
    );
    let stamp = fs::metadata(
        root.path()
            .join("cache/debug")
            .join(triplet)
            .join("std.obj"),
    )
    .unwrap()
    .modified()
    .unwrap();
    assert_eq!(stamp, stamps["debug"]);
    fs::write(
        root.path().join("Project.toml"),
        project.replace(
            "LibCacheDir='cache'",
            "LibCacheDir='cache'\nstd_type='rel-with-info'",
        ),
    )
    .unwrap();
    assert!(
        !Command::new(env!("CARGO_BIN_EXE_mlc"))
            .current_dir(root.path())
            .args(["build", "app", "--config", "config.toml"])
            .status()
            .unwrap()
            .success()
    );
}
