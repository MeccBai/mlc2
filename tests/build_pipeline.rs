//! Exercise the actual frontend and IR generator inside build workers.
use mlc_builder::{
    plan::{BuildPlan, ImportResolver, TargetInput},
    schedule::{self, BuildOptions, CompileOutput, CompileRequest, NodeResult},
};
use mlc_codegen::gens::IrGenerator;
use mlc_core::{
    ast::{
        AbstractSyntaxTree,
        config::{Config, FileId},
        symbols::PackageSymbolTable,
    },
    diagnostic::{error::ErrorHandle, warning::WarningHandle},
};

fn compile(request: CompileRequest<'_>) -> Result<CompileOutput, String> {
    let TargetInput::Source { module, .. } = &request.target.input else {
        return Err("Only source nodes enter the compiler callback".into());
    };
    let file = request.target.file.to_string_lossy().into_owned();
    let config = Config::new(
        FileId::new(request.target.id.0),
        vec![],
        request.target.module_name.clone(),
        file.clone(),
        ErrorHandle::new(file.clone()),
        WarningHandle::new(file),
    );
    // All Rc values, arenas and LLVM generation state remain on this worker.
    let mut ast = AbstractSyntaxTree::new(config, module.clone());
    let mut package = PackageSymbolTable::new();
    ast.analysis(&mut package);
    if ast.config.is_poisoned() {
        return Err(format!("{:?}", ast.config.error_handle()));
    }
    let mut generator = IrGenerator::new(request.options.target.clone());
    generator.generate_globals(ast.config.file_id(), &package);
    for function in ast.body {
        generator.generate(&package, function);
    }
    generator
        .finish()
        .map(|ir| CompileOutput {
            ir: Some(ir),
            global_init: None,
        })
        .map_err(|e| format!("{e:?}"))
}

#[test]
fn text_to_ast_to_ir_to_object_and_incremental_cache() {
    let root = tempfile::tempdir().unwrap();
    let entry = root.path().join("main.m2");
    std::fs::write(
        &entry,
        "func sum(a:i32,b:i32) -> i32 { return a+b; } func main() -> i32 { return sum(1,2); }",
    )
    .unwrap();
    let resolver = ImportResolver { lib_dirs: vec![] };
    let options = BuildOptions {
        output: root.path().join("out"),
        target: "x86_64-pc-windows-msvc".into(),
        compiler_id: "integration-test".into(),
        compiler_options: vec![],
        module_fingerprints: Default::default(),
        workers: 2,
    };
    let report = schedule::build(
        BuildPlan::discover(&entry, &resolver).unwrap(),
        options.clone(),
        compile,
    )
    .unwrap();
    assert!(report.succeeded(), "{report:?}");
    assert_eq!(report.objects.len(), 1);
    assert!(std::fs::metadata(&report.objects[0]).unwrap().len() > 0);
    let cached = schedule::build(
        BuildPlan::discover(&entry, &resolver).unwrap(),
        options,
        |_| Err("A cache hit must not compile".into()),
    )
    .unwrap();
    assert!(matches!(cached.results[0], NodeResult::Cached(_)));
    assert_eq!(report.link_key, cached.link_key);
}

#[test]
fn semantic_error_is_a_node_result_and_publishes_no_manifest() {
    let root = tempfile::tempdir().unwrap();
    let entry = root.path().join("main.m2");
    std::fs::write(&entry, "func main() -> i32 { return unknown; }").unwrap();
    let plan = BuildPlan::discover(&entry, &ImportResolver { lib_dirs: vec![] }).unwrap();
    let options = BuildOptions {
        output: root.path().join("out"),
        target: "x86_64-pc-windows-msvc".into(),
        compiler_id: "integration-test".into(),
        compiler_options: vec![],
        module_fingerprints: Default::default(),
        workers: 1,
    };
    let report = schedule::build(plan, options, compile).unwrap();
    assert!(matches!(report.results[0], NodeResult::Failed { .. }));
    assert!(!root.path().join("out/main.sym").exists());
}

#[test]
fn package_backed_cross_file_ast_and_generic_instances_emit_valid_llvm_object() {
    use mlc_core::ast::Function;
    let mut package = PackageSymbolTable::new();
    let mut asts = Vec::new();
    for (id, module, source) in [
        (
            0,
            "b",
            "export unit P { pub x:i32; }; export func use(p:P) -> i32 { return p.x; } export func<T> id(x:T) -> T { return x; }",
        ),
        (
            1,
            "a",
            "func main() -> i32 { var p = b::P{1}; return b::id<i32>(b::use(p)); }",
        ),
    ] {
        let tokens = mlc_syntax::lexer::tokenize(source).unwrap();
        let (temp, errors) = mlc_syntax::parser::parse(&tokens.tokens, source.len());
        assert!(errors.is_empty());
        let file = format!("{module}.m2");
        let config = Config::new(
            FileId::new(id),
            vec![],
            module.into(),
            file.clone(),
            ErrorHandle::new(file.clone()),
            WarningHandle::new(file),
        );
        if id == 1 {
            package.set_imports(FileId::new(id), [FileId::new(0)]);
        }
        let mut ast = AbstractSyntaxTree::new(config, temp.unwrap());
        ast.export(&mut package);
        ast.analysis(&mut package);
        assert!(!ast.config.is_poisoned(), "{:?}", ast.config.error_handle());
        asts.push(ast);
    }
    let target = "x86_64-pc-windows-msvc";
    let mut generator = IrGenerator::new(target.into());
    for ast in asts {
        for function in ast.body {
            generator.generate(&package, function);
        }
    }
    // Instances created after the defining AST's analysis remain package-owned.
    for (_, file) in package.arenas().iter() {
        for body in file.function_instances.values() {
            generator.generate(&package, Function::Func(body.clone()));
        }
        for body in file.interface_instances.values() {
            generator.generate(&package, Function::Interface(body.clone()));
        }
    }
    let ir = generator.finish().unwrap();
    let output = tempfile::tempdir().unwrap();
    let object = output.path().join("cross-file.obj");
    mlc_builder::llvm::IrCompiler::init(target)
        .unwrap()
        .emit(ir, &object)
        .unwrap();
    assert!(std::fs::metadata(object).unwrap().len() > 0);
}
