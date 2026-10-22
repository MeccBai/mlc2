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
fn line_tables_share_generic_sources_hide_macros_and_mark_pipeline_calls() {
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
    let ir = fs::read_to_string(
        root.path()
            .join("ir/ll/debug/x86_64-pc-windows-msvc/main.ll"),
    )
    .unwrap();
    assert!(ir.contains("DICompileUnit"));
    assert!(ir.contains("line: 8,"));
    let normal = ir
        .lines()
        .find(|line| line.contains("call i32 @\"main::value\""))
        .unwrap();
    let pipe = ir
        .lines()
        .find(|line| line.contains("call i32 @\"main::main::__mlc_step_over_"))
        .unwrap();
    let normal_id = debug_id(normal);
    let pipe_id = debug_id(pipe);
    assert!(ir.contains(&format!("!{normal_id} = !DILocation(line: 8,")));
    assert!(ir.contains(&format!("!{pipe_id} = !DILocation(line: 15728639,")));
    let supplementary = fs::read_to_string(
        root.path()
            .join("ir/ll/debug/x86_64-pc-windows-msvc/__mlc_instances.ll"),
    )
    .unwrap();
    assert_eq!(
        supplementary
            .matches("!DISubprogram(name: \"main::id\"")
            .count(),
        1
    );
    assert!(supplementary.contains("!mlc.generic.templates"));
    assert!(supplementary.contains("linkageName: \"main::id<i32>\""));
    assert!(supplementary.contains("linkageName: \"main::id<f64>\""));
    assert_eq!(
        supplementary
            .lines()
            .filter(
                |line| line.contains("linkageName: \"main::id<") && line.contains("declaration: !")
            )
            .count(),
        2
    );
    assert!(
        !supplementary
            .lines()
            .any(|line| line.contains("DISubprogram") && line.contains("__mlc_macro_"))
    );
    assert!(supplementary.lines().any(|line| line.starts_with("define ")
        && line.contains("main::id<i32>")
        && line.contains("!dbg")));
    assert!(ir.contains("noinline optnone \"disable-tail-calls\"=\"true\""));
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
        !fs::read_to_string(
            root.path()
                .join("release/ll/release/x86_64-pc-windows-msvc/main.ll")
        )
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
    let pdb = root.path().join("build/debug/app.pdb");
    let executable = root.path().join("build/debug/app.exe");
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
    assert!(!root.path().join("release/release/app.pdb").exists());
}

#[test]
#[ignore = "Requires Scoop LLDB, bundled linker and MSVC/UCRT libraries"]
fn lldb_steps_into_shared_generic_sources_and_skips_pipeline_stages() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("config.toml"), "").unwrap();
    fs::write(root.path().join("main.m2"), "import std; func<T> id(x:T)->T {\n    return x;\n}\nfunc value()->i32 { return 3; }\nfunc add(a:i32,b:i32)->i32 { return a+b; }\nfunc main()->i32 {\n    var a=id(1);\n    var b=id(1.5);\n    var c=a |> add(value());\n    var d=add(c,0);\n    std::println!(\"{}\",d);\n    return d-4;\n}\n").unwrap();
    invoke(
        root.path(),
        &[
            "build",
            "main.m2",
            "--debug",
            "--config",
            "config.toml",
            "-o",
            "build",
        ],
    );
    let mut command = Command::new("lldb");
    command.current_dir(root.path()).arg("--batch");
    for action in [
        "breakpoint set --file main.m2 --line 7",
        "run",
        "step",
        "thread backtrace",
        "finish",
        "next",
        "step",
        "thread backtrace",
        "finish",
        "next",
        "step",
        "thread backtrace",
        "finish",
        "step",
        "thread backtrace",
        "step",
        "thread backtrace",
        "finish",
        "next",
        "step",
        "thread backtrace",
        "continue",
    ] {
        command.args(["-o", action]);
    }
    let output = command.arg("build/debug/main.exe").output().unwrap();
    let log = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.status.success(), "{log}");
    assert!(log.contains("main::id<i32> at main.m2:2"), "{log}");
    assert!(log.contains("main::id<f64> at main.m2:2"), "{log}");
    assert!(log.contains("main::value at main.m2:4"), "{log}");
    // Only the final direct add is entered; the preceding pipeline add is skipped.
    let stops: Vec<_> = log.split("(lldb) step").collect();
    assert!(stops[4].contains("main::main at main.m2:10"), "{log}");
    assert!(!stops[4].contains("main::add at"), "{log}");
    assert!(stops[5].contains("main::add at main.m2:5"), "{log}");
    assert!(stops[6].contains("main::main at main.m2:12"), "{log}");
    assert!(!stops[6].contains("__mlc_macro_"), "{log}");
    assert!(log.contains("exited with status = 0"), "{log}");
}

#[test]
#[ignore = "Requires bundled linker and MSVC/UCRT libraries"]
fn debug_forwarding_preserves_indirect_aggregate_void_and_variadic_calls() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("config.toml"), "").unwrap();
    fs::write(
        root.path().join("main.m2"),
        r#"
import std;
import c_std::io;
unit Pair {pub x:i32; pub y:i32; pub z:i32;};
func identity(p:Pair)->Pair {return p;}
func twice(x:i32)->i32 {return x*2;}
func consume(x:i32) {}
func main()->i32 {
    val callback=std::function(twice);
    var a=3 |> callback;
    var p=Pair{a,4,5} |> identity;
    a |> consume;
    const_c_str("%d\n") |> c_std::io::printf(p.x+p.y+p.z);
    return p.x+p.y+p.z-15;
}
"#,
    )
    .unwrap();
    for mode in ["--debug", "--release"] {
        invoke(
            root.path(),
            &[
                "build",
                "main.m2",
                mode,
                "--config",
                "config.toml",
                "-o",
                "build",
            ],
        );
        let run = Command::new(
            root.path()
                .join("build")
                .join(mode.trim_start_matches('-'))
                .join("main.exe"),
        )
        .output()
        .unwrap();
        assert!(run.status.success(), "{mode}: {:?}", run);
        assert_eq!(String::from_utf8_lossy(&run.stdout).trim(), "15");
    }
}

#[test]
fn four_modes_control_ast_optimization_and_debug_metadata_independently() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("config.toml"), "").unwrap();
    fs::write(
        root.path().join("main.m2"),
        "func main()->i32 { var a=2+3; return a; }",
    )
    .unwrap();
    for (flag, optimized, debug) in [
        ("--default", false, false),
        ("--debug", false, true),
        ("--release", true, false),
        ("--rel-with-info", true, true),
    ] {
        invoke(
            root.path(),
            &["ll", "main.m2", flag, "--config", "config.toml", "-o", "ir"],
        );
        let ir = fs::read_to_string(
            root.path()
                .join("ir/ll")
                .join(flag.trim_start_matches('-'))
                .join("x86_64-pc-windows-msvc/main.ll"),
        )
        .unwrap();
        assert_eq!(ir.contains("!llvm.dbg.cu"), debug, "{flag}: {ir}");
        assert_eq!(ir.contains(" = add i32"), !optimized, "{flag}: {ir}");
        if debug {
            assert!(
                ir.contains(&format!("isOptimized: {optimized}")),
                "{flag}: {ir}"
            );
        }
    }
    invoke(
        root.path(),
        &["ll", "main.m2", "--config", "config.toml", "-o", "ir"],
    );
    let ir = fs::read_to_string(
        root.path()
            .join("ir/ll/default/x86_64-pc-windows-msvc/main.ll"),
    )
    .unwrap();
    assert!(!ir.contains("!dbg") && ir.contains(" = add i32"), "{ir}");
}

#[test]
#[ignore = "Requires bundled linker and MSVC/UCRT libraries"]
fn all_four_modes_build_run_and_invalidate_object_cache() {
    let root = fixture();
    let mut previous = None;
    for (flag, debug) in [
        ("--default", false),
        ("--debug", true),
        ("--release", false),
        ("--rel-with-info", true),
    ] {
        let directory = flag.trim_start_matches('-');
        invoke(
            root.path(),
            &[
                "build",
                "main.m2",
                flag,
                "--config",
                "config.toml",
                "-o",
                directory,
            ],
        );
        assert_eq!(
            root.path()
                .join(directory)
                .join(directory)
                .join("main.pdb")
                .exists(),
            debug
        );
        let run = Command::new(root.path().join(directory).join(directory).join("main.exe"))
            .output()
            .unwrap();
        assert!(run.status.success(), "{flag}: {run:?}");
        assert_eq!(String::from_utf8_lossy(&run.stdout).trim(), "4");
        let object = root
            .path()
            .join(directory)
            .join("objects")
            .join(directory)
            .join("x86_64-pc-windows-msvc/main.sym");
        let manifest = mlc_builder::artifacts::load(&object).unwrap();
        let key = manifest.manifest.config.build_key;
        assert_ne!(previous.as_ref(), Some(&key));
        previous = Some(key);
    }
}
