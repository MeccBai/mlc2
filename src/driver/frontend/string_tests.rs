use super::*;

fn generate_source(source: &str) -> (tempfile::TempDir, Result<Generated, String>) {
    let root = tempfile::tempdir().unwrap();
    let entry = root.path().join("main.m2");
    std::fs::write(&entry, source).unwrap();
    let plan = super::test_support::plan(&entry);
    let generated = generate(
        &plan,
        mlc_manifest::Target::default().triplet(),
        TargetKind::Bin,
    );
    (root, generated)
}

#[test]
fn memory_intrinsics_require_import_and_have_no_bare_alias() {
    for statement in [
        "var p = alloc<i32>(1);",
        "dealloc<i32>(@1);",
        "var p = std::mem::alloc<i32>(1);",
        "var p = std::mem::address<i32>(0);",
        "var n = std::bit::or(1, 2);",
    ] {
        let (_, result) =
            generate_source(&format!("func main() -> i32 {{ {statement} return 0; }}"));
        assert!(result.is_err(), "{statement}");
    }
    let (_, result) = generate_source(
        "import std::mem; using allocate = std::mem::alloc; func main() -> i32 { var p:res $mut i32 = allocate<i32>(1); std::mem::dealloc<i32>(p); return 0; }",
    );
    assert!(result.is_ok(), "{:?}", result.err());

    let (_, result) = generate_source(
        "import std::mem; func main() -> i32 { var p = std::mem::address<i32>(1073741824); return $p; }",
    );
    let generated = result.unwrap();
    assert!(
        generated
            .modules
            .iter()
            .any(|(_, ir)| ir.contains("inttoptr"))
    );
}

#[test]
fn builtin_modules_follow_imports_and_can_be_shadowed() {
    let root = tempfile::tempdir().unwrap();
    let entry = root.path().join("main.m2");
    let bridge = root.path().join("bridge.m2");
    let resolver = mlc_builder::plan::ImportResolver { lib_dirs: vec![] };
    let compile = || {
        let plan = BuildPlan::discover(&entry, &resolver).unwrap();
        generate(
            &plan,
            mlc_manifest::Target::default().triplet(),
            TargetKind::Bin,
        )
    };
    std::fs::write(
        &entry,
        "import bridge; func main() -> i32 { return cast<i32>(std::bit::or(1, 2)); }",
    )
    .unwrap();
    std::fs::write(&bridge, "import std::bit;").unwrap();
    assert!(compile().is_err());
    std::fs::write(&bridge, "export import std::bit;").unwrap();
    assert!(compile().is_ok());

    let local_std = root.path().join("std");
    std::fs::create_dir(&local_std).unwrap();
    std::fs::write(
        local_std.join("bit.m2"),
        "export func or(a:i32, b:i32) -> i32 { return a + b; }",
    )
    .unwrap();
    let ir = compile()
        .unwrap()
        .modules
        .into_iter()
        .map(|(_, ir)| ir)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(ir.contains("call i32 @\"std::bit::or\""), "{ir}");
    std::fs::write(local_std.join("bit.m2"), "").unwrap();
    assert!(compile().is_err());
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
    mlc_builder::llvm::IrCompiler::init(mlc_manifest::Target::default().triplet())
        .unwrap()
        .emit(ir.clone(), &object)
        .unwrap();
    #[cfg(windows)]
    {
        let executable = root.path().join("main.exe");
        let linked = super::test_support::link(std::slice::from_ref(&object), &executable);
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
