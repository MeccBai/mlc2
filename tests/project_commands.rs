mod support;
use mlc_builder::{artifacts, project::ProjectFile};
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

fn invoke(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_mlc"))
        .current_dir(root)
        .args(args)
        .output()
        .unwrap()
}
fn success(result: Output) -> Output {
    assert!(
        result.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    result
}

#[test]
fn init_creates_a_valid_project_and_never_overwrites_existing_files() {
    let root = tempfile::tempdir().unwrap();
    success(invoke(root.path(), &["init", "demo"]));
    let project = root.path().join("demo");
    let config = ProjectFile::load(&project).unwrap().unwrap();
    assert_eq!(config.project.name, "demo");
    assert_eq!(config.project.targets[0].entry, Path::new("src/main.m2"));
    let source = project.join("src/main.m2");
    let original = fs::read(&source).unwrap();
    assert!(!invoke(root.path(), &["init", "demo"]).status.success());
    assert_eq!(fs::read(&source).unwrap(), original);
    fs::write(project.join("config.toml"), "").unwrap();
    success(
        Command::new(env!("CARGO_BIN_EXE_mlc"))
            .current_dir(&project)
            .args(["ll", "src/main.m2", "--config", "config.toml", "--lib-dir"])
            .arg(support::library())
            .output()
            .unwrap(),
    );
    // Project names are TOML-escaped rather than interpolated into configuration text.
    success(invoke(root.path(), &["init", "项目-one"]));
    assert_eq!(
        ProjectFile::load(&root.path().join("项目-one"))
            .unwrap()
            .unwrap()
            .project
            .name,
        "项目-one"
    );
}

#[test]
fn missing_inputs_and_imports_report_expected_absolute_paths() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("config.toml"), "").unwrap();
    for args in [
        vec!["ll", "missing.m2", "--config", "config.toml"],
        vec!["symbols", "missing.h"],
        vec!["build"],
        vec!["ll", "main.m2", "--config", "missing-config.toml"],
    ] {
        let result = invoke(root.path(), &args);
        assert!(!result.status.success());
        let error = String::from_utf8_lossy(&result.stderr);
        assert!(error.contains("Expected path"), "{error}");
        assert!(error.contains(root.path().to_str().unwrap()), "{error}");
    }
    fs::write(
        root.path().join("main.m2"),
        "import absent::leaf;func main(){}",
    )
    .unwrap();
    let result = invoke(
        root.path(),
        &[
            "ll",
            "main.m2",
            "--config",
            "config.toml",
            "--lib-dir",
            "lib",
        ],
    );
    let error = String::from_utf8_lossy(&result.stderr).replace('\\', "/");
    assert!(!result.status.success());
    assert!(
        error.contains(
            &root
                .path()
                .join("absent/leaf.m2")
                .display()
                .to_string()
                .replace('\\', "/")
        ),
        "{error}"
    );
    assert!(
        error.contains(
            &root
                .path()
                .join("objects/universal/absent/leaf.sym")
                .display()
                .to_string()
                .replace('\\', "/")
        ),
        "{error}"
    );
}

#[test]
fn setup_prepares_every_library_without_linking_and_reuses_objects() {
    let root = tempfile::tempdir().unwrap();
    let library = root.path().join("lib");
    fs::create_dir_all(library.join("universal/c_std")).unwrap();
    fs::write(
        library.join("universal/c_std/native.h"),
        "int32_t abs(int32_t value);",
    )
    .unwrap();
    fs::write(
        library.join("universal/decl.toml"),
        include_str!("fixtures/c_declarations.toml"),
    )
    .unwrap();
    fs::write(library.join("universal/shared.m2"), "import c_std::native;export func value()->i32{return c_std::native::abs(-3);}export func<T> id(value:T)->T{return value;}").unwrap();
    fs::write(
        library.join("universal/unreferenced.m2"),
        "export func isolated()->i32{return 7;}",
    )
    .unwrap();
    for target in ["x86_64-pc-windows-msvc", "aarch64-unknown-linux-gnu"] {
        fs::create_dir_all(library.join(target)).unwrap();
        fs::write(
            library.join(target).join("api.m2"),
            "import shared;export func answer()->i32{return shared::value();}",
        )
        .unwrap();
    }
    // An old per-mode declaration must not shadow newly converted headers.
    let old_header = root.path().join("old.h");
    fs::write(&old_header, "int32_t obsolete(void);").unwrap();
    artifacts::convert_c_header(
        &old_header,
        &root
            .path()
            .join("objects/debug/x86_64-pc-windows-msvc/c_std/native.sym"),
        "c_std::native",
        "x86_64-pc-windows-msvc",
    )
    .unwrap();
    let args = [
        "setup",
        "--lib-dir",
        "lib",
        "--linker",
        "missing-linker",
        "--archiver",
        "missing-archiver",
    ];
    success(invoke(root.path(), &args));
    let decl = artifacts::load(&root.path().join("objects/universal/c_std/native.sym")).unwrap();
    assert_eq!(decl.manifest.config.object, artifacts::ObjectMode::None);
    assert!(decl.manifest.config.target.is_empty());
    assert!(root.path().join("objects/universal/decl.sym").is_file());
    let mut timestamps = Vec::new();
    for target in ["x86_64-pc-windows-msvc", "aarch64-unknown-linux-gnu"] {
        for mode in ["debug", "release"] {
            let objects = root.path().join("objects").join(mode).join(target);
            for module in ["api", "shared", "unreferenced"] {
                let artifact =
                    artifacts::load(&objects.join(module).with_extension("sym")).unwrap();
                let object = artifact.object.unwrap();
                timestamps.push((
                    object.clone(),
                    fs::metadata(object).unwrap().modified().unwrap(),
                ));
            }
            assert!(objects.join("shared.mg").is_file());
        }
    }
    success(invoke(root.path(), &args));
    for (path, modified) in timestamps {
        assert_eq!(fs::metadata(path).unwrap().modified().unwrap(), modified);
    }
    assert!(!root.path().join("build").exists());
    assert!(!root.path().join("debug/api.lib").exists());
}

#[test]
fn setup_missing_library_path_is_reported_and_flags_select_output_layout() {
    let root = tempfile::tempdir().unwrap();
    let result = invoke(root.path(), &["setup", "--lib-dir", "missing"]);
    assert!(!result.status.success());
    assert!(
        String::from_utf8_lossy(&result.stderr)
            .contains(&root.path().join("missing").display().to_string())
    );
    fs::create_dir_all(root.path().join("lib/universal")).unwrap();
    fs::write(
        root.path().join("lib/universal/native.h"),
        "int32_t abs(int32_t value);",
    )
    .unwrap();
    fs::write(
        root.path().join("lib/universal/value.m2"),
        "import native;export func value()->i32{return native::abs(-1);}",
    )
    .unwrap();
    success(invoke(
        root.path(),
        &[
            "setup",
            "--lib-dir",
            "lib",
            "--triplet",
            "x86_64-pc-windows-msvc",
            "--release",
            "-o",
            "cache",
        ],
    ));
    assert!(
        root.path()
            .join("cache/objects/release/x86_64-pc-windows-msvc/value.obj")
            .is_file()
    );
    assert!(!root.path().join("cache/objects/debug").exists());
}
