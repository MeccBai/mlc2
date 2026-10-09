use mlc_builder::artifacts;
use std::{fs, path::Path, process::Command};

fn run(root: &Path) {
    let output = Command::new(env!("CARGO_BIN_EXE_mlc"))
        .current_dir(root)
        .args([
            "main.m2",
            "--type",
            "static",
            "--config",
            "config.toml",
            "-o",
            "out",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn cli_implementation_change_rebuilds_only_its_object_and_relinks() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("config.toml"), "").unwrap();
    fs::write(
        root.path().join("main.m2"),
        "import leaf; export func entry() -> i32 { return leaf::f(); }",
    )
    .unwrap();
    fs::write(
        root.path().join("leaf.m2"),
        "export func f() -> i32 { return 1; }",
    )
    .unwrap();
    run(root.path());
    let objects = root
        .path()
        .join("out/objects")
        .join(mlc_builder::paths::HOST_TRIPLET);
    let main = objects.join("main.obj");
    let leaf = objects.join("leaf.obj");
    let library = root.path().join("out/main.lib");
    let main_time = fs::metadata(&main).unwrap().modified().unwrap();
    let leaf_time = fs::metadata(&leaf).unwrap().modified().unwrap();
    let library_time = fs::metadata(&library).unwrap().modified().unwrap();
    let previous = artifacts::load(&objects.join("leaf.sym")).unwrap();
    run(root.path());
    assert_eq!(fs::metadata(&main).unwrap().modified().unwrap(), main_time);
    assert_eq!(fs::metadata(&leaf).unwrap().modified().unwrap(), leaf_time);
    assert_eq!(
        fs::metadata(&library).unwrap().modified().unwrap(),
        library_time
    );
    fs::write(
        root.path().join("leaf.m2"),
        "export func f() -> i32 { return 2; }",
    )
    .unwrap();
    run(root.path());
    let current = artifacts::load(&objects.join("leaf.sym")).unwrap();
    assert_eq!(
        previous.manifest.config.symbols_hash,
        current.manifest.config.symbols_hash
    );
    assert_eq!(fs::metadata(&main).unwrap().modified().unwrap(), main_time);
    assert_ne!(fs::metadata(&leaf).unwrap().modified().unwrap(), leaf_time);
    assert_ne!(
        fs::metadata(&library).unwrap().modified().unwrap(),
        library_time
    );
}

#[test]
fn generic_instance_object_is_reused_until_its_ir_changes() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("config.toml"), "").unwrap();
    fs::write(
        root.path().join("main.m2"),
        "func<T> id(x:T) -> T { return x; } export func entry() -> i32 { return id(1); }",
    )
    .unwrap();
    run(root.path());
    let object = root
        .path()
        .join("out/objects")
        .join(mlc_builder::paths::HOST_TRIPLET)
        .join("__mlc_instances.obj");
    let original = fs::metadata(&object).unwrap().modified().unwrap();
    run(root.path());
    assert_eq!(fs::metadata(&object).unwrap().modified().unwrap(), original);
    fs::write(
        root.path().join("main.m2"),
        "func<T> id(x:T) -> T { return x+x; } export func entry() -> i32 { return id(1); }",
    )
    .unwrap();
    run(root.path());
    assert_ne!(fs::metadata(&object).unwrap().modified().unwrap(), original);
}
