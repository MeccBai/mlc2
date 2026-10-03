#![cfg(windows)]
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

fn mlc(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_mlc"))
        .current_dir(root)
        .args(args)
        .output()
        .unwrap()
}

fn success(result: &Output) {
    assert!(
        result.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
#[ignore = "Requires sibling lld-link and configured MSVC CRT/SDK libraries"]
fn cli_hello_world_builds_runs_and_reuses_objects() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let output = tempfile::tempdir().unwrap();
    let destination = output.path().to_str().unwrap();
    let args = [
        "tests/fixtures/hello_world.m2",
        "--lib-dir",
        "lib",
        "-o",
        destination,
    ];
    success(&mlc(&root, &args));
    let executable = output.path().join("hello_world.exe");
    let run = Command::new(&executable).output().unwrap();
    success(&run);
    assert_eq!(run.stdout, b"Hello World!\r\n");
    assert!(run.stderr.is_empty());
    let object = output
        .path()
        .join("objects")
        .join(mlc_builder::paths::HOST_TRIPLET)
        .join("hello_world.obj");
    let before = fs::metadata(&object).unwrap().modified().unwrap();
    success(&mlc(&root, &args));
    assert_eq!(before, fs::metadata(object).unwrap().modified().unwrap());
}

#[test]
#[ignore = "Requires sibling lld-link and configured MSVC CRT/SDK libraries"]
fn project_build_supports_normal_main_globals_imports_and_generics() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("Project.toml"), "[Project]\nName='demo'\nVersion='1'\nOutputDir='build'\n[[Project.Targets]]\nName='app'\nEntry='main.m2'\nType='bin'").unwrap();
    fs::write(
        root.path().join("lib.m2"),
        "export func<T> id(x:T) -> T { return x; }",
    )
    .unwrap();
    fs::write(
        root.path().join("main.m2"),
        "import lib; global var value = 7; func main() -> i32 { return lib::id<i32>(value); }",
    )
    .unwrap();
    success(&mlc(root.path(), &["build", "app"]));
    let run = Command::new(root.path().join("build/app.exe"))
        .output()
        .unwrap();
    assert_eq!(run.status.code(), Some(7), "{run:?}");
    success(&mlc(root.path(), &["build"]));
}

#[test]
#[ignore = "Requires sibling lld-link and configured MSVC CRT/SDK libraries"]
fn cli_library_kinds_and_bad_arguments() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("library.m2"),
        "#[c_abi]# export func answer() -> i32 { return 42; }",
    )
    .unwrap();
    for kind in ["static", "shared"] {
        success(&mlc(root.path(), &["library.m2", "--type", kind]));
    }
    assert!(root.path().join("build/library.lib").is_file());
    assert!(root.path().join("build/library.dll").is_file());
    assert!(
        !mlc(root.path(), &["library.m2", "--type", "lib"])
            .status
            .success()
    );
    assert!(
        !mlc(root.path(), &["library.m2", "-j", "0"])
            .status
            .success()
    );
    assert!(!mlc(root.path(), &["missing.m2"]).status.success());
    success(&mlc(root.path(), &["--help"]));
}

#[test]
#[ignore = "Requires sibling lld-link and configured MSVC CRT/SDK libraries"]
fn native_linkage_names_are_distinct_between_modules() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("a.m2"),
        "export func value() -> i32 { return 2; }",
    )
    .unwrap();
    fs::write(
        root.path().join("b.m2"),
        "export func value() -> i32 { return 3; }",
    )
    .unwrap();
    fs::write(
        root.path().join("main.m2"),
        "import a; import b; func main() -> i32 { return a::value()+b::value(); }",
    )
    .unwrap();
    success(&mlc(root.path(), &["main.m2"]));
    let run = Command::new(root.path().join("build/main.exe"))
        .output()
        .unwrap();
    assert_eq!(run.status.code(), Some(5));
}

#[test]
fn cli_help_and_invalid_output_kind_need_no_linker() {
    let root = tempfile::tempdir().unwrap();
    success(&mlc(root.path(), &["--help"]));
    assert!(
        !mlc(root.path(), &["main.m2", "--type", "lib"])
            .status
            .success()
    );
}

#[test]
fn semantic_error_prints_source_context_instead_of_debug_enum() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("main.m2"),
        "func main() {\r\n    var a = 1;\r\n    return 1;\r\n}\r\n",
    )
    .unwrap();
    let result = mlc(root.path(), &["main.m2"]);
    assert!(!result.status.success());
    let stderr = String::from_utf8(result.stderr).unwrap();
    assert!(stderr.contains("Cannot return a value"), "{stderr}");
    assert!(stderr.trim_end().ends_with("1 error generated."));
    assert!(stderr.contains("main.m2:3:"), "{stderr}");
    assert!(stderr.contains("return 1;"));
    assert!(stderr.contains('^'));
    assert!(!stderr.contains("ErrorHandle"));
    assert!(!stderr.contains("UnexpectedReturnValue"));
    assert!(!stderr.contains(r"\\?\"));
}
