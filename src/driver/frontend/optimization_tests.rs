use super::*;
use mlc_builder::plan::ImportResolver;

#[test]
fn optimizer_switches_control_ir_without_changing_required_calls() {
    let root = tempfile::tempdir().unwrap();
    let entry = root.path().join("main.m2");
    std::fs::write(&entry, "global var hits=0+0;func touch()->i32{hits=hits+1;return hits;}func unused(){}func main()->i32{var discarded=touch();var arithmetic=1+2;if(true){hits=hits+1;}else{hits=hits+10;}return hits-2;}").unwrap();
    let plan = BuildPlan::discover(&entry, &ImportResolver { lib_dirs: vec![] }).unwrap();
    let compiler =
        mlc_builder::llvm::IrCompiler::init(mlc_manifest::Target::default().triplet()).unwrap();
    for (label, options) in [
        ("off", mlc_optimizer::OptimizeConfig::disabled()),
        ("on", mlc_optimizer::OptimizeConfig::enabled()),
    ] {
        let generated = generate_with_options(
            &plan,
            mlc_manifest::Target::default().triplet(),
            TargetKind::Bin,
            options,
        )
        .unwrap();
        let mut objects = vec![];
        for (id, ir) in generated
            .modules
            .into_iter()
            .chain(generated.supplement.into_iter().map(|ir| (usize::MAX, ir)))
        {
            if id != usize::MAX {
                assert!(ir.contains("call i32 @\"main::touch\""), "{ir}");
                assert_eq!(
                    ir.contains("define void @\"main::unused\""),
                    label == "off",
                    "{ir}"
                );
                assert_eq!(ir.contains("if.then"), label == "off", "{ir}");
            }
            let object = root.path().join(format!("{label}-{id}.obj"));
            compiler.emit(ir, &object).unwrap();
            objects.push(object);
        }
        #[cfg(windows)]
        {
            let executable = root.path().join(format!("{label}.exe"));
            let linked = super::test_support::link(&objects, &executable);
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
}

#[test]
fn pruned_control_flow_preserves_side_effects_and_unmatched_match_fallthrough() {
    let root = tempfile::tempdir().unwrap();
    let entry = root.path().join("main.m2");
    std::fs::write(
        &entry,
        r#"
        global var hits = 0;
        func touch() -> bool { hits = hits+1; return true; }
        func main() -> i32 {
            if (1 < 2) { hits = hits+1; } else { hits = hits+100; }
            if (false && touch()) { hits = hits+100; }
            while (false) { touch(); }
            for i in test_range(3, 1) { touch(); }
            for j in test_range(0, 3) {
                match (99) { 1 => { break; }, _ => {} }
                hits = hits+1;
            }
            match (2) { _ => { hits = hits+100; }, 2 => { hits = hits+1; } }
            if (touch() || true) { hits = hits+1; }
            return hits-7;
        }
    "#,
    )
    .unwrap();
    let source = std::fs::read_to_string(&entry).unwrap();
    if source.contains("test_range(") {
        std::fs::write(
            &entry,
            source + include_str!("../../../tests/fixtures/iterator.m2"),
        )
        .unwrap();
    }
    let plan = BuildPlan::discover(&entry, &ImportResolver { lib_dirs: vec![] }).unwrap();
    let generated = generate(
        &plan,
        mlc_manifest::Target::default().triplet(),
        TargetKind::Bin,
    )
    .unwrap();
    let compiler =
        mlc_builder::llvm::IrCompiler::init(mlc_manifest::Target::default().triplet()).unwrap();
    let mut objects = Vec::new();
    let mut modules = generated.modules;
    if let Some(ir) = generated.supplement {
        modules.push((usize::MAX, ir));
    }
    for (id, ir) in modules {
        let object = root.path().join(format!("module{id}.obj"));
        compiler.emit(ir, &object).unwrap();
        objects.push(object);
    }
    #[cfg(windows)]
    {
        let executable = root.path().join("main.exe");
        let output = super::test_support::link(&objects, &executable);
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
            Some(0)
        );
    }
}

#[test]
fn host_os_is_a_constant_and_optimized_matches_keep_only_the_host_arm() {
    let root = tempfile::tempdir().unwrap();
    let entry = root.path().join("main.m2");
    std::fs::write(
        &entry,
        r#"
        import std::host;
        func main() -> i32 {
            const system: std::host::System = std::host::host_os();
            match (system) {
                std::host::System::Windows => { return 11; },
                std::host::System::Linux => { return 22; },
                std::host::System::Macos => { return 33; },
            }
        }
    "#,
    )
    .unwrap();
    let plan = BuildPlan::discover(&entry, &ImportResolver { lib_dirs: vec![] }).unwrap();
    let generated = generate_with_options(
        &plan,
        mlc_manifest::Target::default().triplet(),
        TargetKind::Bin,
        mlc_optimizer::OptimizeConfig::enabled(),
    )
    .unwrap();
    let ir = &generated.modules[0].1;
    let expected = match mlc_manifest::System::host() {
        mlc_manifest::System::Windows => 11,
        mlc_manifest::System::Linux => 22,
        mlc_manifest::System::Macos => 33,
    };
    assert_eq!(ir.matches("ret i32").count(), 1, "{ir}");
    assert!(ir.contains(&format!("store i32 {expected},")), "{ir}");
    assert!(!ir.contains("match.arm"), "{ir}");
}
