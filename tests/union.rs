#![cfg(windows)]
mod support;
use mlc_builder::{llvm::IrCompiler, manifest::X86_64_PC_WINDOWS_MSVC};
use std::{fs, process::Command};

#[test]
fn union_match_move_and_active_payload_cleanup_run() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("main.m2"),
        r#"
import std::mem;
unit Inner { pub p:res $mut i32; };
union Value[i32,Inner];
union<T> Result[T,i32];
union Pointer[res $mut i32,i32];
func make()->Value {
    var p:res = std::mem::alloc<i32>(1);
    p[0]=42;
    return Value{Inner{p}};
}
func read(x:$mut Value)->i32 {
    match(y=x) {
        i32=>{return $y;},
        Inner=>{return y->p[0];}
    }
}
func main()->i32 {
    var x=make();
    if(read(@mut x)!=42){return 1;}
    var moved:res=x;
    var p:res=std::mem::alloc<i32>(1);
    moved=Value{Inner{p}};
    moved=Value{7};
    match(y=@mut moved){i32=>{$y=9;},Inner=>{return 2;}}
    if(read(@mut moved)!=9){return 3;}
    var q:res=std::mem::alloc<i32>(1);
    var nested=Result<Value>{Value{Inner{q}}};
    match(y=@nested){Value=>{},i32=>{return 4;}}
    var r:res=std::mem::alloc<i32>(1);
    var specialized=Result<Inner>{Inner{r}};
    match(y=@specialized){Inner=>{},i32=>{return 5;}}
    var final_owner:res=specialized;
    var allocation:res=std::mem::alloc<i32>(1);
    allocation[0]=11;
    var pointer=Pointer{allocation};
    match(y=@pointer){res $mut i32=>{if(($y)[0]!=11){return 6;}},i32=>{return 7;}}
    return 0;
}
"#,
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_mlc"))
        .current_dir(root.path())
        .args(["ll", "main.m2", "--lib-dir"])
        .arg(support::library())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let compiler = IrCompiler::init(X86_64_PC_WINDOWS_MSVC).unwrap();
    let mut objects = Vec::new();
    for file in fs::read_dir(
        root.path()
            .join("build/ll/default")
            .join(mlc_builder::paths::HOST_TRIPLET),
    )
    .unwrap()
    {
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
    let executable = root.path().join("union.exe");
    objects.push(support::compile_c(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/union_runtime.c"),
        root.path(),
    ));
    let link = support::link(&objects, &executable);
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
