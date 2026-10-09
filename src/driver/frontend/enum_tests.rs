use super::*;
use mlc_builder::plan::ImportResolver;

fn generate_source(
    source: &str,
    dependency: Option<&str>,
) -> (tempfile::TempDir, Result<Generated, String>) {
    let root = tempfile::tempdir().unwrap();
    let entry = root.path().join("main.m2");
    std::fs::write(&entry, source).unwrap();
    if let Some(dependency) = dependency {
        std::fs::write(root.path().join("colors.m2"), dependency).unwrap();
    }
    let plan = BuildPlan::discover(&entry, &ImportResolver { lib_dirs: vec![] }).unwrap();
    let result = generate(&plan, "x86_64-pc-windows-msvc", TargetKind::Bin);
    (root, result)
}

fn verify_and_run(root: &std::path::Path, generated: Generated) {
    let compiler = mlc_builder::llvm::IrCompiler::init("x86_64-pc-windows-msvc").unwrap();
    let mut objects = Vec::new();
    let mut modules = generated.modules;
    if let Some(ir) = generated.supplement {
        modules.push((usize::MAX, ir));
    }
    for (id, ir) in modules {
        let object = root.join(format!("module{id}.obj"));
        compiler.emit(ir, &object).unwrap();
        objects.push(object);
    }
    #[cfg(windows)]
    {
        let executable = root.join("main.exe");
        let clang = std::env::var_os("MLC_TEST_CLANG").unwrap_or_else(|| "clang".into());
        let linked = std::process::Command::new(clang)
            .arg("-fuse-ld=lld")
            .args(&objects)
            .arg("-o")
            .arg(&executable)
            .output()
            .unwrap();
        assert!(
            linked.status.success(),
            "{}",
            String::from_utf8_lossy(&linked.stderr)
        );
        assert_eq!(
            std::process::Command::new(executable)
                .status()
                .unwrap()
                .code(),
            Some(0)
        );
    }
}

#[test]
fn inferred_generic_swap_and_nested_calls_compile_and_run() {
    let (root, result) = generate_source(
        r#"
        generic Base { std::generic::is_integer; }
        func<T:Base> swap(a:$mut T,b:$mut T) {
            var temporary = $a; $a = $b; $b = temporary;
        }
        unit Box<T> { pub value:T; }
        func<T> read(boxed:Box<T>) -> T { return boxed.value; }
        func<T> identity(value:T) -> T { return value; }
        func<T> outer(value:T) -> T { return identity(value); }
        func main() -> i32 {
            var a = 10; var b = 20;
            swap(@mut a,@mut b);
            if (a != 20 || b != 10) { return 1; }
            swap<i32>(@mut a,@mut b);
            if (a != 10 || b != 20) { return 2; }
            var value = read(Box<i32>{outer(42)});
            if (value != 42) { return 3; }
            return 0;
        }
    "#,
        None,
    );
    verify_and_run(root.path(), result.unwrap());
}

#[test]
fn numeric_initialization_preserves_large_constants_and_runtime_widening() {
    let (root, result) = generate_source(
        r#"
        global var large:u64 = 4294967296;
        func widen(a:i8,b:u8,c:i32) -> i32 {
            var x:i64 = a;
            var y:i64 = b;
            var f:f64 = c;
            if (x != cast<i64>(-128)) { return 1; }
            if (y != cast<i64>(255)) { return 2; }
            if (f != 42.0) { return 3; }
            return 0;
        }
        func wide() -> u64 { return 4294967296; }
        func main() -> i32 {
            var number:u64 = 4294967296;
            var array:[u64:2] = [4294967296,1];
            var floating:f32 = 1.5;
            var integer:u32 = 42.0;
            if (number != large || array[0] != wide()) { return 4; }
            if (floating != cast<f32>(1.5)) { return 5; }
            if (integer != cast<u32>(42)) { return 6; }
            return widen(cast<i8>(-128),cast<u8>(255),42);
        }
        "#,
        None,
    );
    verify_and_run(root.path(), result.unwrap());
}

#[test]
fn enum_ordinals_parameters_returns_comparisons_and_match_lower_to_i32() {
    let (root, result) = generate_source(
        r#"
        enum Color { Red, Green, Blue }
        func pick(value:Color) -> Color { return value; }
        func main() -> i32 {
            var inferred = Color::Red;
            var annotated:Color = pick(Color::Green);
            inferred = Color::Blue;
            if (inferred != Color::Blue) { return 1; }
            if (annotated == Color::Red) { return 2; }
            match (annotated) {
                Color::Red => { return 3; },
                Color::Green => { return 0; },
                _ => { return 4; }
            }
        }
    "#,
        None,
    );
    let generated = result.unwrap();
    let ir = &generated.modules[0].1;
    assert!(ir.contains("define i32 @\"main::pick\"(i32"), "{ir}");
    assert!(ir.contains("alloca i32"), "{ir}");
    assert!(ir.contains("icmp eq i32"), "{ir}");
    verify_and_run(root.path(), generated);
}

#[test]
fn enums_work_inside_aggregates_globals_references_and_generic_calls() {
    let (root, result) = generate_source(
        r#"
        enum Color { Red, Green }
        global var selected = Color::Green;
        unit Holder { pub color:Color; }
        func<T> identity(value:T) -> T { return value; }
        func main() -> i32 {
            var holder = Holder { selected };
            var values:[Color:2] = [Color::Red, holder.color];
            var reference = @mut holder.color;
            $reference = identity<Color>(values[1]);
            match (holder.color) {
                Color::Green => { return 0; },
                _ => { return 1; }
            }
        }
    "#,
        None,
    );
    verify_and_run(root.path(), result.unwrap());
}

#[test]
fn imported_enum_keeps_identity_and_uses_i32_across_objects() {
    let (root, result) = generate_source(
        r#"
        import colors;
        func main() -> i32 {
            var color:colors::Color = colors::pick(colors::Color::Green);
            match (color) {
                colors::Color::Green => { return 0; },
                _ => { return 1; }
            }
        }
    "#,
        Some(
            "export enum Color { Red, Green } export func pick(value:Color) -> Color { return value; }",
        ),
    );
    verify_and_run(root.path(), result.unwrap());
}

#[test]
fn i32_lowering_does_not_relax_nominal_enum_type_checks() {
    for source in [
        "enum A { X } enum B { X } func main() -> i32 { var a:A = B::X; return 0; }",
        "enum A { X } func main() -> i32 { var a:A = 0; return 0; }",
        "enum A { X } func main() -> i32 { match (A::X) { 0 => { return 0; }, _ => { return 1; } } }",
    ] {
        let (_, result) = generate_source(source, None);
        assert!(result.is_err(), "{source}");
    }
}

#[test]
fn imported_enum_errors_distinguish_missing_owner_and_variant() {
    let dependency = "export enum Color { Red, Green }";
    for (path, message) in [
        (
            "colors::Color::Nope",
            "Enum `colors::Color` has no variant `Nope`",
        ),
        ("Color::Red", "Enum `Color` is not in scope"),
        (
            "colors::Missing::Red",
            "Enum `colors::Missing` is not in scope",
        ),
    ] {
        let source = format!("import colors; func main() -> i32 {{ var c = {path}; return 0; }}");
        let (_, result) = generate_source(&source, Some(dependency));
        let error = result.err().expect("invalid enum path must fail");
        assert!(error.contains(message), "{error}");
        assert!(!error.contains("Unknown variable"), "{error}");
    }
}
