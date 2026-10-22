//! Keep bundled examples valid through semantics and LLVM object generation.
use super::*;
use mlc_builder::plan::ImportResolver;

#[test]
fn all_bundled_examples_build_and_run() {
    let compiler = mlc_builder::llvm::IrCompiler::init("x86_64-pc-windows-msvc").unwrap();
    for example in mlc_examples::EXAMPLES {
        let root = tempfile::tempdir().unwrap();
        let entry = root.path().join("main.m2");
        std::fs::write(&entry, example.source).unwrap();
        let resolver = ImportResolver {
            lib_dirs: vec![
                super::test_support::library().join("x86_64-pc-windows-msvc"),
                super::test_support::library().join("universal"),
            ],
        };
        let plan = BuildPlan::discover(&entry, &resolver).unwrap();
        let generated = generate(&plan, "x86_64-pc-windows-msvc", TargetKind::Bin)
            .unwrap_or_else(|error| panic!("{}: {error}", example.name));
        let mut objects = Vec::new();
        for (id, ir) in generated.modules {
            let object = root.path().join(format!("module{id}.obj"));
            compiler.emit(ir, &object).unwrap();
            objects.push(object);
        }
        if let Some(ir) = generated.supplement {
            let object = root.path().join("instances.obj");
            compiler.emit(ir, &object).unwrap();
            objects.push(object);
        }
        #[cfg(windows)]
        {
            let executable = root.path().join("main.exe");
            let linked = super::test_support::link(&objects, &executable);
            assert!(
                linked.status.success(),
                "{}: {}",
                example.name,
                String::from_utf8_lossy(&linked.stderr)
            );
            let output = std::process::Command::new(executable).output().unwrap();
            assert!(
                output.status.success(),
                "{}: {:?}",
                example.name,
                output.status
            );
            let expected = example.expected_stdout;
            assert_eq!(
                String::from_utf8(output.stdout)
                    .unwrap()
                    .replace("\r\n", "\n"),
                expected,
                "{}",
                example.name
            );
        }
    }
}
