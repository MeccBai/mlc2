#![cfg(windows)]
use mlc_builder::{
    llvm::IrCompiler,
    manifest::{DEFAULT_WINDOWS_LINKER, X86_64_PC_WINDOWS_GNU},
};
use std::{fs, process::Command};
#[test]
fn resource_errors_are_reported_before_generation() {
    let root = tempfile::tempdir().unwrap();
    let lib = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("lib");
    for (body, message) in [
        ("var p = std::mem::alloc<i32>(1);", "explicit res owner"),
        (
            "var p:res $mut i32 = std::mem::alloc<i32>(1); std::mem::dealloc<i32>(p); $p = 1;",
            "has been moved or destroyed",
        ),
        (
            "var p:res $mut i32 = std::mem::alloc<i32>(1); var q = p; std::mem::dealloc<i32>(p);",
            "while it is borrowed",
        ),
        ("var p = @mut 1; var q:res $mut i32 = p;", "assignment"),
    ] {
        fs::write(
            root.path().join("main.m2"),
            format!("import std::mem; func main() {{ {body} }}"),
        )
        .unwrap();
        let compile = Command::new(env!("CARGO_BIN_EXE_mlc"))
            .current_dir(root.path())
            .args(["ll", "main.m2", "--lib-dir"])
            .arg(&lib)
            .output()
            .unwrap();
        let error = String::from_utf8_lossy(&compile.stderr);
        assert!(!compile.status.success(), "{body}");
        assert!(error.contains(message), "{error}");
    }
}

#[test]
fn resources_move_return_and_cleanup_run_with_gnu_ucrt() {
    let root = tempfile::tempdir().unwrap();
    let source = r#"
import std::mem;
unit Outer { inner:Inner; extra:i64; };
unit Inner { p:res $mut i32; };
unit Box<T> { value:T; };
unit ArrayOwner { items:[Inner:2]; };
func consume_outer(value:Outer) {}
func wrap(p:res $mut i32) -> Outer { return Outer{Inner{p},7}; }
func consume_box(value:Box<res $mut i32>) {}
func make() -> res $mut i32 {
    var p:res = std::mem::alloc<i32>(3);
    p[0] = 10;
    p[1] = 20;
    return p;
}

func consume(p:res $mut i32) -> i32 { return p[0]+p[1]; }
func main() -> i32 {
    var array_a:res = std::mem::alloc<i32>(1);
    var array_b:res = std::mem::alloc<i32>(1);
    var array = ArrayOwner{[Inner{array_a},Inner{array_b}]};
    var array_moved:res = array;
    var inner:res = std::mem::alloc<i32>(1);
    var outer = wrap(inner);
    var moved:res = outer;
    consume_outer(moved);
    var boxed:res = std::mem::alloc<i32>(1);
    consume_box(Box<res $mut i32>{boxed});
    var old:res = std::mem::alloc<i32>(1);
    var replaced_unit = Inner{old};
    var fresh:res = std::mem::alloc<i32>(1);
    replaced_unit = Inner{fresh};
    var a:res $mut i32 = make();
    var b:res = a;
    val sum = consume(b);
    for i in [0,3] {
        var t:res $mut i32 = std::mem::alloc<i32>(1);
        t[0] = i;
        if (i == 0) { continue; }
        break;
    }
    anonymous {
        var manual:res $mut i32 = std::mem::alloc<i32>(1);
        std::mem::dealloc<i32>(manual);
    }
    var replaced:res $mut i32 = std::mem::alloc<i32>(1);
    replaced = std::mem::alloc<i32>(2);
    return sum-30;
}
"#;
    fs::write(root.path().join("main.m2"), source).unwrap();
    let lib = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("lib");
    let compile = Command::new(env!("CARGO_BIN_EXE_mlc"))
        .current_dir(root.path())
        .args(["ll", "main.m2", "--lib-dir"])
        .arg(lib)
        .output()
        .unwrap();
    assert!(
        compile.status.success(),
        "{}",
        String::from_utf8_lossy(&compile.stderr)
    );
    let compiler = IrCompiler::init(X86_64_PC_WINDOWS_GNU).unwrap();
    let mut objects = Vec::new();
    for file in fs::read_dir(root.path().join("build/ll")).unwrap() {
        let path = file.unwrap().path();
        if path.extension().is_some_and(|extension| extension == "ll") {
            let object = path.with_extension("obj");
            let ir = fs::read_to_string(&path)
                .unwrap()
                .replace("@\"free\"", "@\"tracked_free\"")
                .replace("@free(", "@tracked_free(");
            compiler.emit(ir, &object).unwrap();
            objects.push(object);
        }
    }
    let executable = root.path().join("resources.exe");
    let link = Command::new(DEFAULT_WINDOWS_LINKER)
        .arg("-fuse-ld=lld")
        .args(objects)
        .arg(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/resources_runtime.c"),
        )
        .arg("-o")
        .arg(&executable)
        .output()
        .unwrap();
    assert!(
        link.status.success(),
        "{}",
        String::from_utf8_lossy(&link.stderr)
    );
    let output = Command::new(executable).output().unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
