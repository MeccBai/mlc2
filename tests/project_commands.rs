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
    assert_eq!(
        config.project.targets[0].profile,
        Some(mlc_builder::project::BuildMode::Default)
    );
    let source = project.join("src/main.m2");
    let original = fs::read(&source).unwrap();
    assert!(!invoke(root.path(), &["init", "demo"]).status.success());
    assert_eq!(fs::read(&source).unwrap(), original);
    fs::write(project.join("config.toml"), "").unwrap();
    success(
        Command::new(env!("CARGO_BIN_EXE_mlc"))
            .current_dir(&project)
            .args(["ll", "src/main.m2", "--config", "config.toml"])
            .env("MLC", support::library())
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
                .join("lib/x86_64-pc-windows-msvc/absent/leaf.sym")
                .display()
                .to_string()
                .replace('\\', "/")
        ),
        "{error}"
    );
}

#[test]
fn setup_prepares_packages_and_reuses_objects() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("packages/demo/1.0.0/source");
    fs::create_dir_all(&source).unwrap();
    fs::write(
        root.path().join("packages/packages.toml"),
        "[[Packages]]\nName='demo'\nVersion='1.0.0'\n[Packages.Source]\nEntry='main.m2'\n",
    )
    .unwrap();
    fs::write(
        source.join("main.m2"),
        "import demo::native;export func value()->i32{return demo::native::abs(-3);}",
    )
    .unwrap();
    fs::write(source.join("native.h"), "int32_t abs(int32_t value);").unwrap();
    let run = || {
        Command::new(env!("CARGO_BIN_EXE_mlc"))
            .current_dir(root.path())
            .env("MLC", root.path())
            .args([
                "setup",
                "--debug",
                "--triplet",
                "x86_64-pc-windows-msvc",
                "--linker",
                "missing",
                "--archiver",
                "missing",
            ])
            .output()
            .unwrap()
    };
    success(run());
    let declaration = source.join(".compiled/declarations/x86_64-pc-windows-msvc/demo/native.sym");
    assert!(declaration.is_file());
    assert_eq!(
        artifacts::load(&declaration)
            .unwrap()
            .manifest
            .config
            .object,
        artifacts::ObjectMode::None
    );
    let object = source.join(".compiled/build/objects/debug/x86_64-pc-windows-msvc/demo.obj");
    assert!(object.is_file(), "{}", object.display());
    let modified = fs::metadata(&object).unwrap().modified().unwrap();
    success(run());
    assert_eq!(fs::metadata(object).unwrap().modified().unwrap(), modified);
    assert!(!root.path().join("objects").exists());
    assert!(!root.path().join("lib").exists());
}
#[test]
fn setup_missing_catalogue_reports_its_path() {
    let root = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_mlc"))
        .current_dir(root.path())
        .env("MLC", root.path())
        .arg("setup")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(
            &root
                .path()
                .join("packages/packages.toml")
                .display()
                .to_string()
        )
    );
}
