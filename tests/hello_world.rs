//! Real Windows executable: language array -> printf declaration -> LLVM -> CRT -> stdout.
#![cfg(windows)]

use mlc_builder::{
    artifacts,
    config::GlobalConfig,
    llvm::IrCompiler,
    paths::PathResolver,
    plan::{BuildPlan, TargetInput},
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
use std::{path::PathBuf, process::Command};

#[test]
fn hello_world_array_printf_compiles_links_runs_and_matches_stdout() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let paths =
        PathResolver::new(&root.join("mlc.exe"), &root, None, &GlobalConfig::default()).unwrap();
    let plan = BuildPlan::discover_with_paths(&root.join("tests/fixtures/hello_world.m2"), &paths)
        .unwrap();
    let mut package = PackageSymbolTable::new();
    let mut asts = vec![];
    for id in &plan.order {
        let target = &plan.targets[id.0];
        let module = match &target.input {
            TargetInput::Source { module, .. } => module.clone(),
            TargetInput::Declaration(_) => artifacts::load(&target.file)
                .unwrap()
                .manifest
                .symbols
                .declarations(),
        };
        let file = target.file.to_string_lossy().into_owned();
        let config = Config::new(
            FileId::new(id.0),
            vec![],
            target.module_name.clone(),
            file.clone(),
            ErrorHandle::new(file.clone()),
            WarningHandle::new(file),
        );
        package.set_imports(
            FileId::new(id.0),
            target.requires.iter().map(|id| FileId::new(id.0)),
        );
        let mut ast = AbstractSyntaxTree::new(config, module);
        ast.export(&mut package);
        if matches!(target.input, TargetInput::Source { .. }) {
            ast.analysis(&mut package);
        }
        assert!(!ast.config.is_poisoned(), "{:?}", ast.config.error_handle());
        asts.push(ast);
    }
    let triplet = mlc_builder::manifest::X86_64_PC_WINDOWS_GNU;
    let mut generator = IrGenerator::new(triplet.into());
    for ast in asts {
        generator.generate_globals(ast.config.file_id(), &package);
        for function in ast.body {
            generator.generate(&package, function);
        }
    }
    let ir = generator.finish().unwrap();
    assert!(ir.contains("declare i32 @\"printf\"(ptr"), "{ir}");
    assert!(!ir.contains("define i32 @\"printf\""), "{ir}");
    let output = tempfile::tempdir().unwrap();
    let object = output.path().join("hello.obj");
    let executable = output.path().join("hello.exe");
    IrCompiler::init(triplet)
        .unwrap()
        .emit(ir, &object)
        .unwrap();

    // The installed LLVM-MinGW/UCRT driver supplies CRT startup and library paths.
    // Only linking uses clang; language compilation and object emission use MLC/LLVM.
    // This smoke test uses scalar/pointer C ABI signatures compatible with both CRT toolchains.
    let clang = std::env::var_os("MLC_TEST_CLANG")
        .unwrap_or_else(|| mlc_builder::manifest::DEFAULT_WINDOWS_LINKER.into());
    let link = Command::new(clang)
        .arg("-fuse-ld=lld")
        .arg(&object)
        .arg("-o")
        .arg(&executable)
        .output()
        .expect("Install LLVM-MinGW/UCRT clang or set MLC_TEST_CLANG");
    assert!(
        link.status.success(),
        "Link failed:\n{}\n{}",
        String::from_utf8_lossy(&link.stdout),
        String::from_utf8_lossy(&link.stderr)
    );
    let run = Command::new(executable).output().unwrap();
    assert_eq!(
        run.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(run.stdout, b"Hello World!\r\n");
    assert!(
        run.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
}
