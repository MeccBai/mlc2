#![cfg(windows)]
use std::{fs, path::Path, process::Command};
fn invoke(root: &Path, args: &[&str]) {
    let result = Command::new(env!("CARGO_BIN_EXE_mlc"))
        .current_dir(root)
        .env("MLC", root)
        .args(args)
        .output()
        .unwrap();
    assert!(result.status.success(), "{result:?}");
}
#[test]
fn modes_share_only_the_selected_package_std_variant() {
    let root = tempfile::tempdir().unwrap();
    let root = root.path();
    fs::create_dir_all(root.join("packages")).unwrap();
    let version = env!("CARGO_PKG_VERSION");
    let triplet = mlc_builder::paths::HOST_TRIPLET;
    fs::write(
        root.join("packages/packages.toml"),
        format!(
            "[[Packages]]\nName='std'\nVersion='{version}'\n[Packages.Source]\nEntry='std.m2'\n"
        ),
    )
    .unwrap();
    {
        let source = root.join("packages/std").join(version).join("source");
        fs::create_dir_all(&source).unwrap();
        fs::write(
            source.join("std.m2"),
            "export func value()->i32{var x=1+2;return x;}",
        )
        .unwrap();
    }
    fs::write(
        root.join("main.m2"),
        "import std;func main()->i32{return std::value()-3;}",
    )
    .unwrap();
    fs::write(root.join("config.toml"), "").unwrap();
    let project = "[Project]\nName='layout'\nVersion='1.0.0'\nOutputDir='build'\n[[Project.Targets]]\nName='app'\nEntry='main.m2'\nType='bin'\n";
    fs::write(root.join("Project.toml"), project).unwrap();
    let mut stamps = std::collections::HashMap::new();
    for (mode, std_mode) in [
        ("default", "debug"),
        ("debug", "debug"),
        ("release", "release"),
        ("rel-with-info", "release"),
    ] {
        invoke(
            root,
            &[
                "build",
                "app",
                &format!("--{mode}"),
                "--config",
                "config.toml",
            ],
        );
        assert!(
            Command::new(root.join("build").join(mode).join("app.exe"))
                .status()
                .unwrap()
                .success()
        );
        let object = root
            .join("packages/std")
            .join(version)
            .join("source/.compiled/objects")
            .join(std_mode)
            .join(triplet)
            .join("std.obj");
        let bytes = fs::read(&object).unwrap();
        assert_eq!(
            bytes.windows(8).any(|b| b == b".debug$S"),
            std_mode == "debug"
        );
        let stamp = fs::metadata(&object).unwrap().modified().unwrap();
        if let Some(previous) = stamps.insert(std_mode, stamp) {
            assert_eq!(previous, stamp);
        }
    }
    fs::write(
        root.join("Project.toml"),
        project.replace("OutputDir='build'", "OutputDir='build'\nstd_type='release'"),
    )
    .unwrap();
    invoke(
        root,
        &["build", "app", "--debug", "--config", "config.toml"],
    );
    let release = root
        .join("packages/std")
        .join(version)
        .join("source/.compiled/objects/release")
        .join(triplet)
        .join("std.obj");
    assert_eq!(
        fs::metadata(release).unwrap().modified().unwrap(),
        stamps["release"]
    );
    assert!(!root.join("lib").exists());
    assert!(!root.join("objects").exists());
}
