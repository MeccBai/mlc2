#![cfg(windows)]
mod support;
use std::{
    fs,
    process::{Command, Output},
};
fn compile(source: &str, triplet: &str) -> (tempfile::TempDir, Output) {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("main.m2"), source).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_mlc"))
        .current_dir(root.path())
        .args(["ll", "main.m2", "--triplet", triplet])
        .output()
        .unwrap();
    (root, output)
}

fn ir(root: &tempfile::TempDir, target: &str) -> String {
    fs::read_to_string(
        root.path()
            .join("build/ll/default")
            .join(target)
            .join("main.ll"),
    )
    .unwrap()
}
fn success(output: &Output) {
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
#[test]
fn native_constants_execute_loops_recursion_generics_and_nested_calls() {
    let (root, output) = compile(
        "const func square(x:i32)->i32{return x*x;}const func<T> identity(x:T)->T{return x;}const func factorial(n:i32)->i32{if(n<=1){return 1;}return n*factorial(n-1);}const func sum(n:i32)->i32{var i=0;var total=0;while(i<n){total+=square(i);i+=1;}return total;}func main()->i32{const a=sum(5)+factorial(5);const b=identity<i32>(a);return b;}",
        mlc_manifest::Target::default().triplet(),
    );
    success(&output);
    assert!(ir(&root, mlc_manifest::Target::default().triplet()).contains("store i32 150"));
    assert!(!String::from_utf8_lossy(&output.stderr).contains("uses pointers"));
}
#[test]
fn native_constants_copy_strings_floats_arrays_and_large_aggregate_returns() {
    let (root, output) = compile(
        r#"unit Pair{pub a:i64;pub b:i64;};const func pair()->Pair{return Pair{cast<i64>(1234567890123),cast<i64>(7)};}const func array()->[i32:3]{return [3,4,5];}const func number(x:f64)->f64{return x+0.5;}const func text(x:str)->str{return x;}func main()->i32{const p=pair();const a=array();const n=number(2.0);const s=text("中\0x");return a[0];}"#,
        mlc_manifest::Target::default().triplet(),
    );
    success(&output);
    let ir = ir(&root, mlc_manifest::Target::default().triplet());
    assert!(ir.contains("1234567890123"));
    assert!(ir.contains("0x4004000000000000"));
}
#[test]
fn local_pointers_are_unrestricted() {
    let (root, output) = compile(
        "const func pointer()->i32{var a=4;val p=@mut a;$p+=3;return a;}func main()->i32{const result=pointer();return result;}",
        mlc_manifest::Target::default().triplet(),
    );
    success(&output);
    assert!(ir(&root, mlc_manifest::Target::default().triplet()).contains("store i32 7"));
    assert!(!String::from_utf8_lossy(&output.stderr).contains("uses pointers"));
}
#[test]
fn constant_functions_reject_nonconstant_calls_and_runtime_globals() {
    for (source, message) in [
        (
            "func runtime()->i32{return 1;}const func bad()->i32{return runtime();}func main()->i32{const x=bad();return x;}",
            "only call const functions",
        ),
        (
            "global var shared=1;const func bad()->i32{return shared;}func main()->i32{const x=bad();return x;}",
            "const functions cannot access global var or val bindings",
        ),
        (
            "global val shared=1;const func bad()->i32{val local=shared;return local;}func main()->i32{return 0;}",
            "const functions cannot access global var or val bindings",
        ),
        (
            "global val shared=\"text\";const func bad()->str{return shared;}func main(){}",
            "const functions cannot access global var or val bindings",
        ),
        (
            "global var shared=\"text\";const func bad()->str{return shared;}func main(){}",
            "const functions cannot access global var or val bindings",
        ),
    ] {
        let (_, output) = compile(source, mlc_manifest::Target::default().triplet());
        assert!(!output.status.success());
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(message),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
#[test]
fn cross_compilation_allows_scalar_results_from_local_pointer_operations() {
    let target = mlc_manifest::Target::default()
        .with_system(mlc_manifest::System::Linux)
        .with_arch(mlc_manifest::Arch::Aarch64)
        .triplet();
    let (root, output) = compile(
        "const func twice(x:i32)->i32{return x*2;}func main()->i32{const x=twice(7);return x;}",
        target,
    );
    success(&output);
    assert!(ir(&root, target).contains("store i32 14"));
    assert!(String::from_utf8_lossy(&output.stderr).contains("output target"));
    let (root, output) = compile(
        "const func pointer()->i32{var a=1;val p=@a;return $p;}func main()->i32{const x=pointer();return x;}",
        mlc_manifest::Target::default()
            .with_system(mlc_manifest::System::Windows)
            .with_arch(mlc_manifest::Arch::X86)
            .triplet(),
    );
    success(&output);
    assert!(
        ir(
            &root,
            mlc_manifest::Target::default()
                .with_system(mlc_manifest::System::Windows)
                .with_arch(mlc_manifest::Arch::X86)
                .triplet()
        )
        .contains("store i32 1")
    );
}
#[test]
fn imported_constant_functions_have_native_bodies_available() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("math.m2"),
        "export const func twice(n:i32)->i32{return n*2;}",
    )
    .unwrap();
    fs::write(
        root.path().join("main.m2"),
        "import math;func main()->i32{const x=math::twice(21);return x;}",
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_mlc"))
        .current_dir(root.path())
        .args(["ll", "main.m2"])
        .output()
        .unwrap();
    success(&output);
    assert!(ir(&root, mlc_manifest::Target::default().triplet()).contains("store i32 42"));
}

#[test]
fn cached_nongeneric_constant_functions_keep_their_native_source_bodies() {
    let root = tempfile::tempdir().unwrap();
    let library = root
        .path()
        .join("lib")
        .join(mlc_manifest::Target::default().triplet());
    fs::create_dir_all(&library).unwrap();
    let source = library.join("math.m2");
    fs::write(&source, "export const func twice(x:i32)->i32{return x*2;}").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_mlc"))
        .current_dir(root.path())
        .args([
            "setup",
            "--lib-dir",
            "lib",
            "--triplet",
            mlc_manifest::Target::default().triplet(),
            "--debug",
            "-o",
            "install",
        ])
        .output()
        .unwrap();
    success(&output);
    let stem = root
        .path()
        .join("install/objects/debug")
        .join(mlc_manifest::Target::default().triplet())
        .join("math");
    assert!(stem.with_extension("mg").is_file());
    let manifest_path = stem.with_extension("sym");
    let mut artifact = mlc_builder::artifacts::load(&manifest_path).unwrap();
    artifact.manifest.config.object = mlc_builder::artifacts::ObjectMode::Only;
    fs::write(
        &manifest_path,
        mlc_builder::artifacts::to_binary(&artifact.manifest).unwrap(),
    )
    .unwrap();
    fs::remove_file(source).unwrap();
    fs::write(
        root.path().join("config.toml"),
        "LibDirs=['lib']\nLibCacheDir='install/objects'\n",
    )
    .unwrap();
    fs::write(
        root.path().join("main.m2"),
        "import math;func main()->i32{const x=math::twice(21);return x;}",
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_mlc"))
        .current_dir(root.path())
        .args(["ll", "main.m2", "--debug", "--config", "config.toml"])
        .output()
        .unwrap();
    success(&output);
    let ir = fs::read_to_string(
        root.path()
            .join("build/ll/debug")
            .join(mlc_manifest::Target::default().triplet())
            .join("main.ll"),
    )
    .unwrap();
    assert!(ir.contains("store i32 42"));
}

#[test]
fn pointer_const_calls_are_rejected_outside_const_functions() {
    for source in [
        "const func bad(p:$i32)->i32{return $p;}func main(){var n=1;var result=bad(@n);}",
        "unit Box{pub p:$i32;};const func bad(x:Box)->i32{return 0;}func main(){var n=1;var result=bad(Box{@n});}",
        "unit Box<T>{pub value:T;};const func bad(x:[Box<$i32>:2])->i32{return 0;}func main(){var n=1;var result=bad([Box<$i32>{@n},Box<$i32>{@n}]);}",
        "union Input[i32,$i32];const func bad(x:Input)->i32{return 0;}func main(){var result=bad(Input{1});}",
        "const func<T> bad(x:T)->i32{return 0;}func main(){var n=1;var result=bad<$i32>(@n);}",
    ] {
        let (_, output) = compile(source, mlc_manifest::Target::default().triplet());
        assert!(!output.status.success());
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("cannot contain pointers"),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
#[test]
fn constant_functions_reject_c_calls_even_when_marked_constant() {
    for modifier in ["", "const "] {
        let source = format!(
            "#[c_abi]# {modifier}func foreign(x:i32)->i32{{return x;}}const func bad()->i32{{return foreign(1);}}func main(){{}}"
        );
        let (_, output) = compile(&source, mlc_manifest::Target::default().triplet());
        assert!(!output.status.success());
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("cannot call C functions"),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
#[test]
fn constant_evaluation_rejects_direct_c_abi_calls() {
    let (_, output) = compile(
        "#[c_abi]# const func foreign(x:i32)->i32{return x;}func main()->i32{const result=foreign(1);return result;}",
        mlc_manifest::Target::default().triplet(),
    );
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("cannot call C functions"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
#[test]
fn target_pointer_config_controls_malloc_size_t_width() {
    let target = mlc_manifest::Target::default()
        .with_system(mlc_manifest::System::Windows)
        .with_arch(mlc_manifest::Arch::X86)
        .triplet();
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("main.m2"),"import std::mem;func main()->i32{var p:res=std::mem::alloc<i32>(1);std::mem::dealloc<i32>(p);return 0;}").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_mlc"))
        .current_dir(root.path())
        .args(["ll", "main.m2", "--triplet", target, "--lib-dir"])
        .arg(support::library())
        .output()
        .unwrap();
    success(&output);
    assert!(
        ir(&root, target).contains("declare ptr @\"malloc\"(i32)")
            || ir(&root, target).contains("declare ptr @malloc(i32)")
    );
}

#[test]
fn constant_arguments_support_boolean_expressions_and_aggregate_members() {
    let (root, output) = compile(
        "unit Pair{pub a:i32;pub b:i32;};const func pair()->Pair{return Pair{3,4};}const func pick(value:bool)->i32{if(value){return 30;}return 0;}const func add(x:i32)->i32{return x+1;}func main()->i32{const p=pair();const a=[5,6];const value=pick(2>1)+add(p.a)+add(a[0]);return value;}",
        mlc_manifest::Target::default().triplet(),
    );
    success(&output);
    assert!(ir(&root, mlc_manifest::Target::default().triplet()).contains("store i32 40"));
}

fn cache_directories(root: &std::path::Path) -> Vec<std::path::PathBuf> {
    let directory = root
        .join("build/ctfe")
        .join(mlc_manifest::Target::default().triplet());
    let mut entries = fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.join("cache.toml").is_file())
        .collect::<Vec<_>>();
    entries.sort();
    entries
}
fn invoke_cached(root: &std::path::Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_mlc"))
        .current_dir(root)
        .args(args)
        .output()
        .unwrap()
}
#[test]
fn persistent_constant_cache_skips_codegen_linking_and_worker_on_hits() {
    let source =
        "const func twice(x:i32)->i32{return x*2;}func main()->i32{const x=twice(21);return x;}";
    let (root, output) = compile(source, mlc_manifest::Target::default().triplet());
    success(&output);
    let directories = cache_directories(root.path());
    assert_eq!(directories.len(), 1);
    let entry = &directories[0];
    let record: toml::Value =
        toml::from_str(&fs::read_to_string(entry.join("cache.toml")).unwrap()).unwrap();
    assert_eq!(record["temp_ast_hash"].as_str().unwrap().len(), 64);
    assert_eq!(record["optimization"].as_str(), Some("O3"));
    let old = std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000);
    for name in ["constant.dll", "result.json", "cache.toml"] {
        fs::OpenOptions::new()
            .write(true)
            .open(entry.join(name))
            .unwrap()
            .set_modified(old)
            .unwrap();
    }
    let times = ["constant.dll", "result.json", "cache.toml"]
        .map(|name| fs::metadata(entry.join(name)).unwrap().modified().unwrap());
    success(&invoke_cached(root.path(), &["ll", "main.m2"]));
    assert_eq!(
        times,
        ["constant.dll", "result.json", "cache.toml"]
            .map(|name| fs::metadata(entry.join(name)).unwrap().modified().unwrap())
    );
    fs::write(
        root.path().join("main.m2"),
        format!("// whitespace changes positions, not the native AST\n{source}\n"),
    )
    .unwrap();
    success(&invoke_cached(root.path(), &["ll", "main.m2"]));
    assert_eq!(
        times,
        ["constant.dll", "result.json", "cache.toml"]
            .map(|name| fs::metadata(entry.join(name)).unwrap().modified().unwrap())
    );
    let source = source.replace("return x;}", "const y=twice(22);return x+y;}");
    fs::write(root.path().join("main.m2"), &source).unwrap();
    success(&invoke_cached(root.path(), &["ll", "main.m2"]));
    assert_eq!(cache_directories(root.path()).len(), 2);
    assert_eq!(
        fs::metadata(entry.join("cache.toml"))
            .unwrap()
            .modified()
            .unwrap(),
        times[2]
    );
    fs::write(root.path().join("main.m2"), source.replace("x*2", "x*3")).unwrap();
    success(&invoke_cached(root.path(), &["ll", "main.m2"]));
    let updated: toml::Value =
        toml::from_str(&fs::read_to_string(entry.join("cache.toml")).unwrap()).unwrap();
    assert_ne!(record["temp_ast_hash"], updated["temp_ast_hash"]);
    assert!(ir(&root, mlc_manifest::Target::default().triplet()).contains("store i32 63"));
    assert!(ir(&root, mlc_manifest::Target::default().triplet()).contains("store i32 66"));
}
#[test]
fn persistent_constant_cache_rebuilds_corruption_and_respects_output_roots() {
    let (root, output) = compile(
        "const func answer()->i32{return 42;}func main()->i32{const x=answer();return x;}",
        mlc_manifest::Target::default().triplet(),
    );
    success(&output);
    let entry = cache_directories(root.path()).pop().unwrap();
    for name in ["constant.dll", "result.json", "cache.toml"] {
        fs::write(entry.join(name), "broken").unwrap();
        success(&invoke_cached(root.path(), &["ll", "main.m2"]));
        assert!(ir(&root, mlc_manifest::Target::default().triplet()).contains("store i32 42"));
        assert!(
            fs::read(entry.join("constant.dll"))
                .unwrap()
                .starts_with(b"MZ")
        );
        let _: toml::Value =
            toml::from_str(&fs::read_to_string(entry.join("cache.toml")).unwrap()).unwrap();
    }
    success(&invoke_cached(
        root.path(),
        &["ll", "main.m2", "--release", "-o", "custom"],
    ));
    let directory = root
        .path()
        .join("custom/ctfe")
        .join(mlc_manifest::Target::default().triplet());
    assert_eq!(fs::read_dir(directory).unwrap().count(), 1);
}

#[test]
fn wide_integer_const_functions_normalize_literal_operands() {
    let (root, output) = compile(
        "const func square(x:i64)->i64{return x*x;}const func sum_squares(n:i64)->i64{var i:i64=0;var total:i64=0;while(i<n){total+=square(i);i+=1;}return total;}const func factorial(n:i64)->i64{if(n<=1){return 1;}return n*factorial(n-1);}func ordinary(n:i64)->i64{if(1>=n){return 1;}return n*ordinary(n-1);}func main()->i32{const n:i64=10000;const sum=sum_squares(n);const product=factorial(cast<i64>(10));return 0;}",
        mlc_manifest::Target::default().triplet(),
    );
    success(&output);
    let ll = ir(&root, mlc_manifest::Target::default().triplet());
    assert!(ll.contains("store i64 333283335000"));
    assert!(ll.contains("store i64 3628800"));
}

#[test]
fn constant_functions_can_read_global_constants_and_cache_tracks_changes() {
    let source = "global const base=6;global const shared=base*7;const func answer()->i32{return shared;}func main()->i32{const result=answer();return result;}";
    let (root, output) = compile(source, mlc_manifest::Target::default().triplet());
    success(&output);
    assert!(ir(&root, mlc_manifest::Target::default().triplet()).contains("store i32 42"));
    fs::write(
        root.path().join("main.m2"),
        source.replace("base=6", "base=8"),
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_mlc"))
        .current_dir(root.path())
        .args(["ll", "main.m2"])
        .output()
        .unwrap();
    success(&output);
    assert!(ir(&root, mlc_manifest::Target::default().triplet()).contains("store i32 56"));
}

#[test]
fn constant_functions_can_read_globals_initialized_by_constant_calls() {
    let (root, output) = compile(
        "const func twice(x:i32)->i32{return x*2;}global const shared=twice(21);const func answer()->i32{return shared;}func main()->i32{const result=answer();return result;}",
        mlc_manifest::Target::default().triplet(),
    );
    success(&output);
    assert!(ir(&root, mlc_manifest::Target::default().triplet()).contains("store i32 42"));
}

#[test]
fn const_functions_allow_reference_parameters_and_internal_aliasing() {
    let (root, output) = compile(
        "unit Box{pub p:$mut i32;};const func unused(p:$i32)->i32{return $p;}const func<T> write(a:$mut T,b:$T,value:T)->T{$a=value;return $b;}const func nested(box:Box)->i32{return write<i32>(box.p,cast<$i32>(box.p),17);}const func calculate()->i32{var a=9;return nested(Box{@mut a});}func main()->i32{const result=calculate();return result;}",
        mlc_manifest::Target::default().triplet(),
    );
    success(&output);
    assert!(ir(&root, mlc_manifest::Target::default().triplet()).contains("store i32 17"));
}
#[test]
fn reference_const_calls_do_not_fall_back_to_runtime() {
    for source in [
        "const func read(p:$i32)->i32{return $p;}func runtime(p:$i32)->i32{return read(p);}func main(){}",
        "const func update(p:$mut i32){$p+=1;}func main(){var n=1;update(@mut n);}",
        "const func read(p:$i32)->i32{return $p;}global const n=1;global const result=read(@n);func main(){}",
    ] {
        let (_, output) = compile(source, mlc_manifest::Target::default().triplet());
        assert!(!output.status.success());
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("outside a const function"),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn ctfe_cache_is_shared_across_all_profiles_and_optimization_overrides() {
    let (root, output) = compile(
        "const func twice(x:i32)->i32{return x*2;}func main()->i32{const result=twice(21);return result;}",
        mlc_manifest::Target::default().triplet(),
    );
    success(&output);
    let entries = cache_directories(root.path());
    assert_eq!(entries.len(), 1);
    let entry = &entries[0];
    let old = std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000);
    for name in ["constant.dll", "result.json", "cache.toml"] {
        fs::OpenOptions::new()
            .write(true)
            .open(entry.join(name))
            .unwrap()
            .set_modified(old)
            .unwrap();
    }
    let times = ["constant.dll", "result.json", "cache.toml"]
        .map(|name| fs::metadata(entry.join(name)).unwrap().modified().unwrap());
    for flag in [
        "--debug",
        "--release",
        "--rel-with-info",
        "--default",
        "--no-optimize",
        "--optimize",
    ] {
        success(&invoke_cached(root.path(), &["ll", "main.m2", flag]));
        assert_eq!(cache_directories(root.path()), entries);
        assert_eq!(
            times,
            ["constant.dll", "result.json", "cache.toml"].map(|name| fs::metadata(
                entry.join(name)
            )
            .unwrap()
            .modified()
            .unwrap()),
            "{flag}"
        );
    }
    assert!(
        !root
            .path()
            .join("build/objects/default")
            .join(mlc_manifest::Target::default().triplet())
            .join("ctfe")
            .exists()
    );
    let target = mlc_manifest::Target::default()
        .with_system(mlc_manifest::System::Windows)
        .with_arch(mlc_manifest::Arch::X86)
        .triplet();
    success(&invoke_cached(
        root.path(),
        &["ll", "main.m2", "--triplet", target],
    ));
    assert_eq!(
        fs::read_dir(
            root.path()
                .join("build/ctfe")
                .join(mlc_manifest::Target::default().triplet())
        )
        .unwrap()
        .count(),
        1
    );
    assert!(!root.path().join("build/ctfe").join(target).exists());
}
