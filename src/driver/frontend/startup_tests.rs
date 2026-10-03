use super::*;
use mlc_builder::plan::ImportResolver;
use mlc_core::ast::symbol_name::SymbolName;

#[test]
fn static_initializers_follow_dependencies_and_run_only_once() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(
        root.path().join("dep.m2"),
        "global var count = 0; global var value = initialize(); \
         func initialize() -> i32 { count = count+1; return 7; } \
         export func get() -> i32 { return value; } \
         export func calls() -> i32 { return count; }",
    )
    .unwrap();
    std::fs::write(
        root.path().join("middle.m2"),
        "import dep; global var value = dep::get(); export func get() -> i32 { return value; }",
    )
    .unwrap();
    let entry = root.path().join("library.m2");
    std::fs::write(
        &entry,
        "import dep; import middle; global var value = middle::get()+dep::get(); \
         #[c_abi]# export func answer() -> i32 { return value; }",
    )
    .unwrap();
    let plan = BuildPlan::discover(&entry, &ImportResolver { lib_dirs: vec![] }).unwrap();
    let generated = generate(&plan, "x86_64-pc-windows-msvc", TargetKind::Static).unwrap();
    assert!(generated.supplement.is_none());
    let root_init = SymbolName::module_initializer("library");
    let dep_init = SymbolName::module_initializer("dep");
    let compiler = mlc_builder::llvm::IrCompiler::init("x86_64-pc-windows-msvc").unwrap();
    let mut objects = vec![];
    for (id, ir) in generated.modules {
        assert!(!ir.contains("@llvm.global_ctors"));
        assert!(ir.contains("internal global i1 false"));
        let object = root.path().join(format!("module{id}.obj"));
        compiler.emit(ir, &object).unwrap();
        objects.push(object);
    }
    #[cfg(windows)]
    {
        let archive = root.path().join("library.lib");
        let output = std::process::Command::new("llvm-ar")
            .arg("rcs")
            .arg(&archive)
            .args(&objects)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let caller = format!(
            "declare void @\"{root_init}\"()\ndeclare void @\"{dep_init}\"()\ndeclare i32 @answer()\ndeclare i32 @\"dep::calls\"()\n\
            define i32 @main() {{\nentry:\n call void @\"{root_init}\"()\n call void @\"{root_init}\"()\n call void @\"{dep_init}\"()\n\
            %value = call i32 @answer()\n %count = call i32 @\"dep::calls\"()\n %v = icmp eq i32 %value, 14\n\
            %c = icmp eq i32 %count, 1\n %ok = and i1 %v, %c\n %status = select i1 %ok, i32 0, i32 1\n ret i32 %status\n}}\n"
        );
        let object = root.path().join("caller.obj");
        compiler.emit(caller, &object).unwrap();
        let executable = root.path().join("caller.exe");
        let clang = std::env::var_os("MLC_TEST_CLANG").unwrap_or_else(|| "clang".into());
        let output = std::process::Command::new(clang)
            .arg("-fuse-ld=lld")
            .arg(object)
            .arg(archive)
            .arg("-o")
            .arg(&executable)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            std::process::Command::new(executable)
                .status()
                .unwrap()
                .success()
        );
    }
}

#[test]
fn initializer_names_are_stable_and_collision_free() {
    assert_eq!(
        SymbolName::module_initializer("std::io"),
        SymbolName::module_initializer("std::io")
    );
    assert_ne!(
        SymbolName::module_initializer("a::b"),
        SymbolName::module_initializer("a_b")
    );
}

#[test]
fn prebuilt_imports_keep_global_init_metadata_and_startup_calls() {
    use mlc_builder::schedule::{self, BuildOptions, CompileOutput};
    use std::collections::HashMap;
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("library.m2");
    std::fs::write(
        &source,
        "global var value = 7; export func get() -> i32 { return value; }",
    )
    .unwrap();
    let resolver = ImportResolver { lib_dirs: vec![] };
    let plan = BuildPlan::discover(&source, &resolver).unwrap();
    let generated = generate(&plan, "x86_64-pc-windows-msvc", TargetKind::Static).unwrap();
    let modules: HashMap<_, _> = generated.modules.into_iter().collect();
    let inits: HashMap<_, _> = generated.global_inits.into_iter().collect();
    let output = root.path().join("objects");
    let report = schedule::build(
        plan,
        BuildOptions {
            output: output.clone(),
            target: "x86_64-pc-windows-msvc".into(),
            compiler_id: "startup-test".into(),
            compiler_options: vec![],
            workers: 1,
        },
        move |request| {
            Ok(CompileOutput {
                ir: modules.get(&request.target.id.0).cloned(),
                global_init: inits.get(&request.target.id.0).cloned(),
            })
        },
    )
    .unwrap();
    assert!(report.succeeded(), "{report:?}");
    let artifact = report.results[0].artifact().unwrap();
    let init = artifact.manifest.config.global_init.clone().unwrap();
    let reloaded = mlc_builder::artifacts::load(&artifact.path).unwrap();
    assert_eq!(reloaded.manifest.config.global_init.as_ref(), Some(&init));
    let entry = output.join("main.m2");
    std::fs::write(
        &entry,
        "import library; func main() -> i32 { return library::get(); }",
    )
    .unwrap();
    let plan = BuildPlan::discover(&entry, &resolver).unwrap();
    let generated = generate(&plan, "x86_64-pc-windows-msvc", TargetKind::Bin).unwrap();
    assert_eq!(generated.modules.len(), 1);
    let ir = &generated.modules[0].1;
    assert!(ir.contains(&format!("call void @\"{init}\"()")), "{ir}");
    let compiler = mlc_builder::llvm::IrCompiler::init("x86_64-pc-windows-msvc").unwrap();
    compiler
        .emit(ir.clone(), &root.path().join("consumer.obj"))
        .unwrap();
    compiler
        .emit(
            generated.supplement.unwrap(),
            &root.path().join("entry.obj"),
        )
        .unwrap();
}
