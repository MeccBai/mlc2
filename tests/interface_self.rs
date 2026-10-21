#![cfg(windows)]

mod support;
use mlc_builder::{
    llvm::IrCompiler,
    manifest::{DEFAULT_WINDOWS_LINKER, X86_64_PC_WINDOWS_GNU},
};
use std::path::Path;
use std::{fs, process::Command};

fn compile(source: &str, root: &Path) -> std::process::Output {
    fs::write(root.join("main.m2"), source).unwrap();
    Command::new(env!("CARGO_BIN_EXE_mlc"))
        .current_dir(root)
        .args(["ll", "main.m2", "--lib-dir"])
        .arg(support::library())
        .output()
        .unwrap()
}

#[test]
fn self_is_an_ordinary_borrow_in_interface_bodies_and_calls() {
    let root = tempfile::tempdir().unwrap();
    let output = compile(
        r#"
unit Counter { value:i32; };
pub Counter::func read(self)->i32 { return self->value; }
pub Counter::func set(mut self, value:i32) { self->value=value; }
pub i32::func read(self)->i32 { return $self; }
pub i32::func set(mut self, value:i32) { $self=value; }
func main()->i32 {
    var counter=Counter{1};
    counter.set(10);
    var pointer=@mut counter;
    pointer->set(20);
    pointer->read();
    if(counter.read()!=20){return 1;}
    var integer=40;
    integer.set(50);
    if(integer.read()!=50){return 2;}
    return 0;
}
"#,
        root.path(),
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
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
    let executable = root.path().join("self.exe");
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
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(0));
}

#[test]
fn readonly_self_and_receivers_cannot_mutate_the_owner() {
    for source in [
        "unit A { value:i32; }; A::func set(self) { self->value=1; } func main(){}",
        "pub i32::func set(mut self) { $self=1; } func main(){val n=0; n.set();}",
        "pub i32::func set(mut self) { $self=1; } func main(){var n=0; val p=@n; p->set();}",
    ] {
        let root = tempfile::tempdir().unwrap();
        let output = compile(source, root.path());
        assert!(!output.status.success(), "{source}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("error:"), "{stderr}");
        assert!(!stderr.contains("GenerationErrors"), "{stderr}");
    }
}
