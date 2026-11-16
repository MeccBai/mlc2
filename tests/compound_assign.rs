use std::{fs, process::Command};

fn compile(source: &str) -> std::process::Output {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("main.m2"), source).unwrap();
    fs::write(root.path().join("config.toml"), "").unwrap();
    Command::new(env!("CARGO_BIN_EXE_mlc"))
        .current_dir(root.path())
        .args(["ll", "main.m2", "--config", "config.toml"])
        .output()
        .unwrap()
}

#[test]
fn immutable_and_invalid_places_are_rejected() {
    for statement in [
        "val x=1; x+=2;",
        "const x=1; x-=2;",
        "var x=1; val p=@x; $p*=2;",
        "1/=2;",
        "var x=true; x%=false;",
        "var x:i8=1; x+=300;",
        "var x:i8=1; x+=200+100;",
        "var x:u32=1; x+=-1;",
        "unit P{v:i32;} var p=P{1}; p+=P{2};",
    ] {
        let output = compile(&format!("func main(){{{statement}}}"));
        assert!(!output.status.success(), "{statement}");
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains("error:"), "{error}");
        assert!(!error.contains("panicked"), "{error}");
    }
}

#[test]
fn formatting_preserves_compound_operators() {
    let source = "func main(){var x=10;x+=1;x-=2;x*=3;x/=4;x%=5;}";
    let formatted = mlc_formatter::format(source).unwrap();
    for op in ["+=", "-=", "*=", "/=", "%="] {
        assert!(formatted.contains(&format!("x {op} ")), "{formatted}");
    }
    assert_eq!(mlc_formatter::format(&formatted).unwrap(), formatted);
}

#[cfg(windows)]
#[test]
fn arithmetic_and_places_run_with_single_target_evaluation() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("config.toml"), "").unwrap();
    fs::write(
        root.path().join("main.m2"),
        r#"
unit Pair { pub value:i32; }
func index(count:$mut i32)->i32 { $count += 1; return 0; }
func change(p:$mut i32)->i32 { $p=100; return 2; }
func<T> add(p:$mut T,value:T) { $p += value; }
func main()->i32 {
    var x=20;
    x+=2; x-=4; x*=3; x/=2; x%=10;
    if(x!=7){return 1;}
    var u:u32=10; u+=2; u-=1; u*=3; u/=3; u%=5;
    if(u!=cast<u32>(1)){return 2;}
    var extra:u32=2; extra=4; u+=1+extra;
    if(u!=cast<u32>(6)){return 7;}
    add<u32>(@mut u,cast<u32>(2));
    if(u!=cast<u32>(8)){return 8;}
    var f=10.0; f+=2.0; f-=4.0; f*=3.0; f/=2.0;
    if(f!=12.0){return 3;}
    var pair=Pair{5}; pair.value*=2;
    if(pair.value!=10){return 4;}
    val p=@mut x; $p+=2*3;
    if(x!=13){return 5;}
    var count=0; var a:[i32:1]=[10];
    a[index(@mut count)]+=2+3;
    if(count!=1 || a[0]!=15){return 6;}
    x=1; x+=change(@mut x);
    if(x!=3){return 9;}
    return 0;
}
"#,
    )
    .unwrap();
    let repository = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    for mode in ["--default", "--release", "--debug"] {
        let output = Command::new(env!("CARGO_BIN_EXE_mlc"))
            .current_dir(root.path())
            .args([
                "run",
                "main.m2",
                "--config",
                "config.toml",
                mode,
                "--linker",
            ])
            .arg(repository.join("tools").join(format!(
                "{}{}",
                mlc_manifest::Target::default().linker(),
                mlc_manifest::EXE_SUFFIX
            )))
            .arg("--runtime-dir")
            .arg(repository.join(format!(
                "runtimes/{}",
                mlc_manifest::Target::default().triplet()
            )))
            .args(["--triplet", mlc_manifest::Target::default().triplet()])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{mode}: {output:?}\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
