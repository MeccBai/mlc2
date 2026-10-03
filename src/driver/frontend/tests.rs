use super::*;
use mlc_builder::plan::ImportResolver;

#[test]
fn type_name_calls_are_not_builtin_conversions() {
    for name in ["i8", "i32", "u64", "f64"] {
        let root = tempfile::tempdir().unwrap();
        let entry = root.path().join("main.m2");
        std::fs::write(
            &entry,
            format!("func main() -> i32 {{ var value = {name}(1); return 0; }}"),
        )
        .unwrap();
        let plan = BuildPlan::discover(
            &entry,
            &ImportResolver {
                lib_dirs: vec![
                    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("lib/universal"),
                ],
            },
        )
        .unwrap();
        let error = generate(&plan, "x86_64-pc-windows-msvc", TargetKind::Bin)
            .err()
            .expect("legacy conversion must fail before linking");
        assert!(error.contains("Unknown function"), "{error}");
    }
}

#[test]
fn builtins_lower_without_external_language_symbols() {
    let root = tempfile::tempdir().unwrap();
    let entry = root.path().join("main.m2");
    std::fs::write(
        &entry,
        "import std::mem; #[c_abi]# func main() -> i32 { var p = std::mem::alloc<i32>(2); $p = 42; \
         var value = $p; var narrow:i8 = cast<i8>(300); \
         var wide = cast<i32>(narrow); std::mem::dealloc<i32>(p); return value + wide; }",
    )
    .unwrap();
    let plan = BuildPlan::discover(
        &entry,
        &ImportResolver {
            lib_dirs: vec![
                std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("lib/universal"),
            ],
        },
    )
    .unwrap();
    let generated = generate(&plan, "x86_64-pc-windows-msvc", TargetKind::Bin).unwrap();
    let ir = &generated.modules[0].1;
    assert!(ir.contains("declare ptr @malloc(i64)"), "{ir}");
    assert!(ir.contains("declare void @free(ptr)"), "{ir}");
    assert!(ir.contains("44"), "{ir}");
    assert!(!ir.contains("declare i8 @\"main::"), "{ir}");
    let object = root.path().join("main.obj");
    mlc_builder::llvm::IrCompiler::init("x86_64-pc-windows-msvc")
        .unwrap()
        .emit(ir.clone(), &object)
        .unwrap();
    #[cfg(windows)]
    {
        let executable = root.path().join("main.exe");
        let clang = std::env::var_os("MLC_TEST_CLANG").unwrap_or_else(|| "clang".into());
        let output = std::process::Command::new(clang)
            .arg("-fuse-ld=lld")
            .arg(&object)
            .arg("-o")
            .arg(&executable)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            std::process::Command::new(executable)
                .status()
                .unwrap()
                .code(),
            Some(86)
        );
    }
}

#[test]
fn builtin_arguments_are_validated_before_generation() {
    for statement in [
        "var p = std::mem::alloc<i32>();",
        "var p = std::mem::alloc<i32>(-1);",
        "var p = std::mem::alloc<i32>(1.5);",
        "var p = std::mem::alloc<i32,i8>(1);",
        "std::mem::dealloc<i32>(1);",
        "var p = std::mem::alloc<i32>(1); std::mem::dealloc<i8>(p);",
        "var c = cast<i32>();",
    ] {
        let root = tempfile::tempdir().unwrap();
        let entry = root.path().join("main.m2");
        std::fs::write(
            &entry,
            format!("import std::mem; func main() -> i32 {{ {statement} return 0; }}"),
        )
        .unwrap();
        let plan = BuildPlan::discover(
            &entry,
            &ImportResolver {
                lib_dirs: vec![
                    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("lib/universal"),
                ],
            },
        )
        .unwrap();
        assert!(
            generate(&plan, "x86_64-pc-windows-msvc", TargetKind::Bin).is_err(),
            "{statement}"
        );
    }
}

#[test]
fn builtin_casts_and_generic_allocations_pass_llvm_verification() {
    let root = tempfile::tempdir().unwrap();
    let entry = root.path().join("main.m2");
    std::fs::write(
        &entry,
        "import std::mem; func<T> make(n:i32) -> $mut T { return std::mem::alloc<T>(n); } \
         func main() -> i32 { var p = make<i32>(1); $p = 10; std::mem::dealloc<i32>(p); \
         var v = 300; var narrow = cast<i8>(v); var f = cast<f64>(v); \
         var single = cast<f32>(f); var integer = cast<i32>(single); \
         var big = cast<i64>(4294967296); const folded:i8 = cast<i8>(300); \
         return integer; }",
    )
    .unwrap();
    let plan = BuildPlan::discover(
        &entry,
        &ImportResolver {
            lib_dirs: vec![
                std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("lib/universal"),
            ],
        },
    )
    .unwrap();
    let generated = generate(&plan, "x86_64-pc-windows-msvc", TargetKind::Bin).unwrap();
    let compiler = mlc_builder::llvm::IrCompiler::init("x86_64-pc-windows-msvc").unwrap();
    for (id, ir) in generated.modules {
        assert!(ir.contains("trunc i32"), "{ir}");
        compiler
            .emit(ir, &root.path().join(format!("module{id}.obj")))
            .unwrap();
    }
    compiler
        .emit(
            generated.supplement.unwrap(),
            &root.path().join("instances.obj"),
        )
        .unwrap();
}

#[test]
fn multilevel_reference_reads_and_writes_generate_valid_ir() {
    let root = tempfile::tempdir().unwrap();
    let entry = root.path().join("main.m2");
    std::fs::write(
        &entry,
        "func main() -> i32 { var a = 10; var p = @mut a; var q = @mut p; \
         var r = @mut q; $$q = 20; $$$r = 30; var copy = $$q; return $$$r; }",
    )
    .unwrap();
    let plan = BuildPlan::discover(
        &entry,
        &ImportResolver {
            lib_dirs: vec![
                std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("lib/universal"),
            ],
        },
    )
    .unwrap();
    let generated = generate(&plan, "x86_64-pc-windows-msvc", TargetKind::Bin).unwrap();
    let ir = &generated.modules[0].1;
    assert!(ir.matches("load ptr").count() >= 5, "{ir}");
    assert!(ir.contains("load i32"), "{ir}");
    assert!(ir.contains("store i32"), "{ir}");
    mlc_builder::llvm::IrCompiler::init("x86_64-pc-windows-msvc")
        .unwrap()
        .emit(ir.clone(), &root.path().join("main.obj"))
        .unwrap();
}

#[test]
fn constant_integer_narrowing_reports_out_of_range() {
    for (statement, valid) in [
        ("var c:i8 = 300;", false),
        ("var c:i8 = 100+200;", false),
        ("var c:i8 = 127;", true),
        ("var c:i8 = -128;", true),
        ("var c:i8 = -129;", false),
        ("var c:u8 = 255;", true),
        ("var c:u8 = 256;", false),
        ("var c:u8 = -1;", false),
        ("var c:[i8:2] = [1,300];", false),
        ("var c:i8 = 0; c = 300;", false),
    ] {
        let root = tempfile::tempdir().unwrap();
        let entry = root.path().join("main.m2");
        std::fs::write(
            &entry,
            format!("func main() -> i32 {{ {statement} return 0; }}"),
        )
        .unwrap();
        let plan = BuildPlan::discover(
            &entry,
            &ImportResolver {
                lib_dirs: vec![
                    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("lib/universal"),
                ],
            },
        )
        .unwrap();
        let result = generate(&plan, "x86_64-pc-windows-msvc", TargetKind::Bin);
        if valid {
            assert!(result.is_ok(), "{statement}: {:?}", result.err());
        } else {
            let error = result.err().expect("out-of-range constant must fail");
            assert!(error.contains("out of range"), "{statement}: {error}");
        }
    }
}

#[test]
fn array_initializers_allow_missing_elements_and_reject_braces_and_excess() {
    for (initializer, valid) in [
        ("[]", true),
        ("[1,2]", true),
        ("[1,2,3]", true),
        ("[1,2,3,4]", false),
        ("{}", false),
    ] {
        let root = tempfile::tempdir().unwrap();
        let entry = root.path().join("main.m2");
        std::fs::write(
            &entry,
            format!("func main() -> i32 {{ var t:[i32:3] = {initializer}; return 0; }}"),
        )
        .unwrap();
        let plan = BuildPlan::discover(
            &entry,
            &ImportResolver {
                lib_dirs: vec![
                    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("lib/universal"),
                ],
            },
        )
        .unwrap();
        let result = generate(&plan, "x86_64-pc-windows-msvc", TargetKind::Bin);
        if valid {
            let generated = result.unwrap();
            assert!(generated.modules[0].1.contains("llvm.memset.p0.i64"));
        } else {
            let error = result.err().expect("invalid initialization must fail");
            if initializer == "{}" {
                assert!(error.contains("Array initializers must use"), "{error}");
            }
        }
    }
}

#[test]
fn local_generic_instance_is_defined_only_in_supplement() {
    let root = tempfile::tempdir().unwrap();
    let entry = root.path().join("main.m2");
    std::fs::write(
        &entry,
        "generic T {}; func<ty:T> swap(a:$mut ty,b:$mut ty) { \
         var temp = $a; $a = $b; $b = temp; } \
         func main() -> i32 { var a = 10; var b = 20; \
         swap<i32>(@mut a,@mut b); swap<i32>(@mut a,@mut b); return 0; }",
    )
    .unwrap();
    let plan = BuildPlan::discover(
        &entry,
        &ImportResolver {
            lib_dirs: vec![
                std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("lib/universal"),
            ],
        },
    )
    .unwrap();
    let generated = generate(&plan, "x86_64-pc-windows-msvc", TargetKind::Bin).unwrap();
    let symbol = "@\"main::swap<i32>\"";
    let definitions = |ir: &str| {
        ir.lines()
            .filter(|line| line.starts_with("define ") && line.contains(symbol))
            .count()
    };
    assert_eq!(generated.modules.len(), 1);
    let module = &generated.modules[0].1;
    assert_eq!(definitions(module), 0, "{module}");
    assert!(
        module
            .lines()
            .any(|line| line.starts_with("declare ") && line.contains(symbol)),
        "{module}"
    );
    let supplement = generated.supplement.unwrap();
    assert_eq!(definitions(&supplement), 1, "{supplement}");
}
