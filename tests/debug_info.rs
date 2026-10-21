#![cfg(windows)]
mod support;
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};
const SOURCE: &str = "import std;\nfunc<T> id(x:T)->T{return x;}\nfunc value()->i32{return 3;}\nfunc add(a:i32,b:i32)->i32{return a+b;}\nfunc main()->i32{\nvar a=id(1);\nvar b=id(1.5);\nvar c=a |> add(value());\nstd::println!(\"{}\",c);\nreturn c-4+flow();\n}\nfunc flow()->i32{\nvar x=0;\nwhile(x<2){x=x+1;}\nfor i in [0,2]{x=x+i;}\nif(x==3){return 0;}\nreturn 1;\n}\n";
fn invoke(root: &Path, args: &[&str]) -> Output {
    let output = Command::new(env!("CARGO_BIN_EXE_mlc"))
        .current_dir(root)
        .args(args)
        .arg("--lib-dir")
        .arg(support::library())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}
fn fixture() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("main.m2"), SOURCE).unwrap();
    fs::write(root.path().join("config.toml"), "").unwrap();
    root
}
fn debug_id(line: &str) -> &str {
    line.rsplit("!dbg !").next().unwrap().trim()
}
#[test]
fn line_tables_hide_expansions_and_mark_pipeline_calls() {
    let root = fixture();
    invoke(
        root.path(),
        &[
            "ll",
            "main.m2",
            "--debug",
            "--config",
            "config.toml",
            "-o",
            "ir",
        ],
    );
    let ir = fs::read_to_string(root.path().join("ir/main.ll")).unwrap();
    assert!(ir.contains("DICompileUnit"));
    assert!(ir.contains("line: 8,"));
    let normal = ir
        .lines()
        .find(|line| line.contains("call i32 @\"main::value\""))
        .unwrap();
    let pipe = ir
        .lines()
        .find(|line| line.contains("call i32 @\"main::add\""))
        .unwrap();
    let normal_id = debug_id(normal);
    let pipe_id = debug_id(pipe);
    assert!(ir.contains(&format!("!{normal_id} = !DILocation(line: 8,")));
    assert!(ir.contains(&format!("!{pipe_id} = !DILocation(line: 15728639,")));
    let supplementary = fs::read_to_string(root.path().join("ir/__mlc_instances.ll")).unwrap();
    assert_eq!(
        supplementary
            .matches("!DISubprogram(name: \"main::id\"")
            .count(),
        1
    );
    assert!(supplementary.contains("!mlc.generic.templates"));
    assert!(
        !supplementary
            .lines()
            .any(|line| line.contains("DISubprogram") && line.contains("__mlc_macro_"))
    );
    assert!(
        !supplementary
            .lines()
            .any(|line| line.starts_with("define ") && line.contains("!dbg"))
    );
    invoke(
        root.path(),
        &[
            "ll",
            "main.m2",
            "--release",
            "--config",
            "config.toml",
            "-o",
            "release",
        ],
    );
    assert!(
        !fs::read_to_string(root.path().join("release/main.ll"))
            .unwrap()
            .contains("!dbg")
    );
}
#[test]
fn debug_and_release_are_mutually_exclusive() {
    let root = fixture();
    let result = Command::new(env!("CARGO_BIN_EXE_mlc"))
        .current_dir(root.path())
        .args(["ll", "main.m2", "--debug", "--release"])
        .output()
        .unwrap();
    assert!(!result.status.success());
}
#[test]
#[ignore = "Requires bundled lld-link and MSVC/UCRT libraries"]
fn project_mode_pdb_and_cache_are_consistent() {
    let root = fixture();
    fs::write(root.path().join("Project.toml"), "[Project]\nName='debug-test'\nVersion='1'\nOutputDir='build'\nMode='release'\n[[Project.Targets]]\nName='app'\nEntry='main.m2'\nType='bin'\nMode='debug'\n").unwrap();
    invoke(root.path(), &["build", "app", "--config", "config.toml"]);
    let pdb = root.path().join("build/app.pdb");
    let executable = root.path().join("build/app.exe");
    let pdb_bytes = fs::read(&pdb).unwrap();
    assert!(pdb_bytes.starts_with(b"Microsoft C/C++ MSF 7.00"));
    assert!(
        pdb_bytes
            .windows(4)
            .any(|bytes| u32::from_le_bytes(bytes.try_into().unwrap()) & 0x00ff_ffff == 0xf00f00),
        "NeverStepInto must survive LLVM object emission and PDB linking"
    );
    let run = Command::new(&executable).output().unwrap();
    assert!(run.status.success());
    assert_eq!(
        String::from_utf8(run.stdout).unwrap().replace("\r\n", "\n"),
        "4\n"
    );
    let modified = fs::metadata(&executable).unwrap().modified().unwrap();
    invoke(root.path(), &["build", "app", "--config", "config.toml"]);
    assert_eq!(
        modified,
        fs::metadata(&executable).unwrap().modified().unwrap()
    );
    fs::remove_file(&pdb).unwrap();
    invoke(root.path(), &["build", "app", "--config", "config.toml"]);
    assert!(pdb.is_file());
    invoke(
        root.path(),
        &[
            "build",
            "app",
            "--release",
            "--config",
            "config.toml",
            "-o",
            "release",
        ],
    );
    assert!(!root.path().join("release/app.pdb").exists());
}
