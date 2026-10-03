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
                std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("lib/universal"),
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
            let expected = match example.name {
                "hello-world" => "Hello World!\n",
                "variables" => "answer=42 counter=43 narrowed=44\n",
                "arrays" => "Hello World!\nnumbers[1]=25\n",
                "control-flow" => "sum=10\nmatched ten\n",
                "functions" => "fibonacci(10)=55\n",
                "generics" => "a=20 b=10\n",
                "units" => "point=(30,20)\n",
                "references" => "value=42\n",
                "memory" => "allocated value=42\n",
                name => panic!("Add a stdout assertion for {name}"),
            };
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
