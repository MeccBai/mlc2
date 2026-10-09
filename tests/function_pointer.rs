#![cfg(windows)]
use mlc_builder::{
    llvm::IrCompiler,
    manifest::{DEFAULT_WINDOWS_LINKER, X86_64_PC_WINDOWS_GNU},
};
use std::{fs, process::Command};

#[test]
fn ordinary_generic_and_c_abi_function_pointers_run() {
    let root = tempfile::tempdir().unwrap();
    let source = r#"
import c_std::io;
func max(a:i32,b:i32)->i32 { if(a>b){return a;}else{return b;} }
func<T> identity(a:T)->T { return a; }
func other(a:i32,b:i32)->i32 { return a+b; }
func apply(f:func(i32,i32)->i32)->i32 { return f(4,7); }
func make()->func(i32,i32)->i32 { return std::function(max); }
func<T> apply_typed(f:func(T)->T,x:T)->T { return f(x); }
unit TypedHolder<T> {pub callback:func(T)->T;};
global var global_ptr=std::function(max);
unit Holder<F> { pub callback:F; };
func<F> invoke(f:F)->i32 { return f(4,7); }
func<F> check_holder(ptr:F)->i32 {
    var holder=Holder<F>{ptr};
    if(holder.callback(4,7)!=11){return 5;}
    var borrowed=@holder;
    if(borrowed->callback(4,7)!=11){return 6;}
    return 0;
}
func main()->i32 {
    if(global_ptr(4,7)!=7){return 8;}
    var ptr:func(i32,i32)->i32=make();
    if(apply(ptr)!=7){return 9;}
    var typed=TypedHolder<u32>{std::function<u32>(identity)};
    if(apply_typed(typed.callback,cast<u32>(42))!=cast<u32>(42)){return 10;}
    if(ptr(4,7)!=7){return 1;}
    ptr=std::function(other);
    if(ptr(4,7)!=11){return 2;}
    var ptr_i=std::function<u32>(identity);
    if(ptr_i(cast<u32>(42))!=cast<u32>(42)){return 3;}
    if(invoke(std::function(max))!=7){return 4;}
    if(check_holder(ptr)!=0){return 5;}
    var refs=@ptr;
    if(($refs)(4,7)!=11){return 7;}
    var print=std::function(c_std::io::printf);
    print(const_c_str("function pointer: %d\n"),ptr(4,7));
    return 0;
}
"#;
    fs::write(root.path().join("main.m2"), source).unwrap();
    let lib = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("lib");
    let build = Command::new(env!("CARGO_BIN_EXE_mlc"))
        .current_dir(root.path())
        .args(["ll", "main.m2", "--lib-dir"])
        .arg(lib)
        .output()
        .unwrap();
    assert!(
        build.status.success(),
        "{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let compiler = IrCompiler::init(X86_64_PC_WINDOWS_GNU).unwrap();
    let mut objects = Vec::new();
    for file in fs::read_dir(root.path().join("build/ll")).unwrap() {
        let path = file.unwrap().path();
        if path.extension().is_some_and(|extension| extension == "ll") {
            let object = path.with_extension("obj");
            compiler
                .emit(fs::read_to_string(path).unwrap(), &object)
                .unwrap();
            objects.push(object);
        }
    }
    let executable = root.path().join("function_pointer.exe");
    let link = Command::new(DEFAULT_WINDOWS_LINKER)
        .arg("-fuse-ld=lld")
        .args(objects)
        .arg("-o")
        .arg(&executable)
        .output()
        .unwrap();
    assert!(
        link.status.success(),
        "{}",
        String::from_utf8_lossy(&link.stderr)
    );
    let run = Command::new(executable).output().unwrap();
    assert!(
        run.status.success(),
        "exit={:?}; {}",
        run.status.code(),
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&run.stdout).replace("\r\n", "\n"),
        "function pointer: 11\n"
    );
}
