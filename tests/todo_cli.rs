use std::{fs, process::Command};
#[test]
fn todo_accepts_expected_types_and_exits_with_a_source_location() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("main.m2");
    fs::write(&path,"unit Pair{pub value:i32;};func missing()->Pair{return todo();}func accept(value:bool){}func main()->i32{\nvar value:f64=todo();\naccept(todo());\nreturn 0;}\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_mlc"))
        .current_dir(root.path())
        .args(["run", "main.m2"])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("not implemented (todo)"),
        "{stdout}\n{stderr}"
    );
    assert!(stdout.contains("main.m2:2:"), "{stdout}");
    assert!(!output.status.success());
}

#[test]
fn todo_supports_conditions_and_expected_aggregate_string_pointer_types() {
    let root = tempfile::tempdir().unwrap();
    for source in [
        "func main()->i32{if(todo()){return 1;}return 0;}",
        "const func identity(x:i32)->i32{return x;}func main()->i32{const result=identity(todo());return result;}",
        "func<T> missing()->T{return todo();}func main()->i32{var a=missing<f64>();var b=missing<i32>();return b;}",
        "unit Pair{pub value:i32;};func a()->Pair{return todo();}func b()->str{return todo();}func c()->$i32{return todo();}func d()->[i32:3]{return todo();}func main()->i32{return 0;}",
    ] {
        fs::write(root.path().join("main.m2"), source).unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_mlc"))
            .current_dir(root.path())
            .args(["ll", "main.m2", "--release"])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
