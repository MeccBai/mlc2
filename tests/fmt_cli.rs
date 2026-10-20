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

fn success(output: Output) {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn formats_one_file_without_resolving_imports_or_loading_project() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("Project.toml"), "invalid [").unwrap();
    let file = root.path().join("main.m2");
    fs::write(&file, "import missing;func main(){var a=1;// keep\n}").unwrap();
    fs::write(root.path().join("other.m2"), "broken").unwrap();
    let first = invoke(root.path(), &["fmt", "main.m2"]);
    let summary = String::from_utf8_lossy(&first.stdout);
    assert!(summary.contains("Format completed in "), "{summary}");
    assert!(summary.contains("1 changed, 0 unchanged"), "{summary}");
    assert!(!first.stderr.contains(&b'\r'));
    success(first);
    let text = fs::read_to_string(&file).unwrap();
    assert!(text.contains("    var a = 1; // keep\n"));
    let modified = fs::metadata(&file).unwrap().modified().unwrap();
    let second = invoke(root.path(), &["fmt", "main.m2"]);
    assert!(String::from_utf8_lossy(&second.stdout).contains("0 changed, 1 unchanged"));
    success(second);
    assert_eq!(modified, fs::metadata(&file).unwrap().modified().unwrap());
    assert_eq!(
        fs::read_to_string(root.path().join("other.m2")).unwrap(),
        "broken"
    );
}

#[test]
fn discovers_project_root_and_skips_generated_directories() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("Project.toml"),
        "[Project]\nName='fmt'\nVersion='1'\nOutputDir='output'\nTargets=[]",
    )
    .unwrap();
    fs::create_dir_all(root.path().join("src")).unwrap();
    fs::write(root.path().join("main.m2"), "func main(){}").unwrap();
    fs::write(root.path().join("src/leaf.m2"), "func leaf(){}").unwrap();
    for dir in [
        "build", "target", "objects", "tools", "runtimes", ".git", "output",
    ] {
        fs::create_dir_all(root.path().join(dir)).unwrap();
        fs::write(root.path().join(dir).join("broken.m2"), "broken").unwrap();
    }
    success(invoke(&root.path().join("src"), &["fmt"]));
    assert_eq!(
        fs::read_to_string(root.path().join("main.m2")).unwrap(),
        "func main() {\n}\n"
    );
    assert_eq!(
        fs::read_to_string(root.path().join("src/leaf.m2")).unwrap(),
        "func leaf() {\n}\n"
    );
}

#[test]
fn syntax_error_does_not_modify_any_project_file() {
    let root = tempfile::tempdir().unwrap();
    let original = "func main(){}";
    fs::write(root.path().join("a.m2"), original).unwrap();
    fs::write(root.path().join("z.m2"), "func broken(){var a;}").unwrap();
    let output = invoke(root.path(), &["fmt"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("z.m2"));
    assert_eq!(
        fs::read_to_string(root.path().join("a.m2")).unwrap(),
        original
    );
}

#[test]
fn rejects_missing_file_and_directory_argument() {
    let root = tempfile::tempdir().unwrap();
    assert!(!invoke(root.path(), &["fmt", "missing.m2"]).status.success());
    assert!(!invoke(root.path(), &["fmt", "."]).status.success());
}
