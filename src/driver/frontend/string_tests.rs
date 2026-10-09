use super::*;
use mlc_builder::plan::ImportResolver;

fn generate_source(source: &str) -> (tempfile::TempDir, Result<Generated, String>) {
    let root = tempfile::tempdir().unwrap();
    let entry = root.path().join("main.m2");
    std::fs::write(&entry, source).unwrap();
    let resolver = ImportResolver {
        lib_dirs: vec![std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("lib/universal")],
    };
    let plan = BuildPlan::discover(&entry, &resolver).unwrap();
    let generated = generate(&plan, "x86_64-pc-windows-msvc", TargetKind::Bin);
    (root, generated)
}

#[test]
fn memory_intrinsics_require_import_and_have_no_bare_alias() {
    for statement in [
        "var p = alloc<i32>(1);",
        "dealloc<i32>(@1);",
        "var p = std::mem::alloc<i32>(1);",
    ] {
        let (_, result) =
            generate_source(&format!("func main() -> i32 {{ {statement} return 0; }}"));
        assert!(result.is_err(), "{statement}");
    }
    let (_, result) = generate_source(
        "import std::mem; using allocate = std::mem::alloc; func main() -> i32 { var p:res $mut i32 = allocate<i32>(1); std::mem::dealloc<i32>(p); return 0; }",
    );
    assert!(result.is_ok(), "{:?}", result.err());
}

#[test]
fn const_c_str_rejects_nonconstant_strings_and_mutable_access() {
    for statement in [
        "var s = const_c_str(42);",
        "var s = const_c_str();",
        "var s = const_c_str<i8>(\"hello\");",
        "var text:[i8:4] = \"abc\"; var s = const_c_str(text);",
        "var s = const_c_str(\"hello\"); $s = 1;",
    ] {
        let (_, result) =
            generate_source(&format!("func main() -> i32 {{ {statement} return 0; }}"));
        assert!(result.is_err(), "{statement}");
    }
}

#[test]
fn const_c_str_static_utf8_empty_and_deduplicated_strings_print() {
    let (root, result) = generate_source(
        r#"
        import c_std::io;
        func text() -> $i8 { return const_c_str("Hello World!\n"); }
        #[c_abi]# func main() -> i32 {
            const message = const_c_str("Hello World!\n");
            c_std::io::printf(message);
            c_std::io::printf(text());
            c_std::io::printf(const_c_str(""));
            c_std::io::printf(const_c_str("中文\n"));
            return 0;
        }
    "#,
    );
    let generated = result.unwrap();
    let ir = &generated.modules[0].1;
    assert_eq!(
        ir.matches("private unnamed_addr constant [14 x i8]")
            .count(),
        1,
        "{ir}"
    );
    assert!(ir.contains("constant [1 x i8] c\"\\00\""), "{ir}");
    assert!(!ir.contains("@malloc"), "{ir}");
    assert!(!ir.contains("alloca [14 x i8]"), "{ir}");
    let object = root.path().join("main.obj");
    mlc_builder::llvm::IrCompiler::init("x86_64-pc-windows-msvc")
        .unwrap()
        .emit(ir.clone(), &object)
        .unwrap();
    #[cfg(windows)]
    {
        let executable = root.path().join("main.exe");
        let clang = std::env::var_os("MLC_TEST_CLANG").unwrap_or_else(|| "clang".into());
        let linked = std::process::Command::new(clang)
            .arg("-fuse-ld=lld")
            .arg(&object)
            .arg("-o")
            .arg(&executable)
            .output()
            .unwrap();
        assert!(
            linked.status.success(),
            "{}",
            String::from_utf8_lossy(&linked.stderr)
        );
        let output = std::process::Command::new(executable).output().unwrap();
        assert!(output.status.success());
        assert_eq!(
            String::from_utf8(output.stdout)
                .unwrap()
                .replace("\r\n", "\n"),
            "Hello World!\nHello World!\n中文\n"
        );
    }
}
