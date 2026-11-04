use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

fn clean(root: &Path, extra: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_mlc"))
        .current_dir(root)
        .arg("clean")
        .args(extra)
        .output()
        .unwrap()
}

#[test]
fn clean_removes_all_build_profiles_and_is_idempotent() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir_all(root.path().join("build/objects/release")).unwrap();
    fs::write(root.path().join("build/objects/release/main.obj"), "object").unwrap();
    fs::write(root.path().join("main.m2"), "func main(){}").unwrap();
    for _ in 0..2 {
        let result = clean(root.path(), &[]);
        assert!(result.status.success(), "{:?}", result);
        assert!(!root.path().join("build").exists());
        assert!(root.path().join("main.m2").is_file());
    }
}

#[test]
fn clean_respects_project_output_and_explicit_override() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("Project.toml"),
        "[Project]\nName='app'\nVersion='1'\nOutputDir='out'\nTargets=[]",
    )
    .unwrap();
    for directory in ["out", "custom", "build"] {
        fs::create_dir(root.path().join(directory)).unwrap();
    }
    assert!(clean(root.path(), &[]).status.success());
    assert!(!root.path().join("out").exists());
    assert!(clean(root.path(), &["-o", "custom"]).status.success());
    assert!(!root.path().join("custom").exists());
    assert!(root.path().join("build").is_dir());
}

#[test]
fn clean_rejects_project_parent_files_and_nested_projects() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("main.m2"), "func main(){}").unwrap();
    fs::create_dir(root.path().join("nested")).unwrap();
    fs::write(root.path().join("nested/Project.toml"), "").unwrap();
    for target in [".", "..", "main.m2", "nested"] {
        assert!(
            !clean(root.path(), &["-o", target]).status.success(),
            "{target}"
        );
    }
    assert!(root.path().join("main.m2").exists());
    assert!(root.path().join("nested/Project.toml").exists());
}
